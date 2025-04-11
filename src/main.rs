use actix_cors::Cors;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::SystemTime,
};
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct BackendResult {
    success: bool,
    time: SystemTime,
    run_output: Option<String>,
    params_used: Option<Payload>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Payload {
    nsweeps: String,
    nf: String,
    smax: String,
    local: String,
    code: String,
}

// AppState to store shared state
struct AppState {
    redis_con: redis::aio::MultiplexedConnection,
    request_in_progress: AtomicBool,
}

const IP: &str = "0.0.0.0:4004";

// Constants for validation bounds
const NSWEEPS_MIN: u32 = 20;
const NSWEEPS_MAX: u32 = 3_000;
const NF_MIN: u32 = 10_000;
const NF_MAX: u32 = 2_000_000;
const SMAX_MIN: u32 = 100;
const SMAX_MAX: u32 = 5_000;
const LOCAL_MIN: u32 = 0;
const LOCAL_MAX: u32 = 500;
const REDIS_CACHE_TTL: u64 = 86400; // 24 hours in seconds

fn validate_unsigned(value: &str, min: u32, max: u32) -> Result<String, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|e| format!("'{value}' is not a valid unsigned integer: {e}"))?;

    if parsed < min || parsed > max {
        return Err(format!(
            "Value {parsed} is out of acceptable range {min}:{max}"
        ));
    }

    Ok(parsed.to_string())
}

fn validate_input(payload: &Payload) -> Result<Payload, String> {
    Ok(Payload {
        nsweeps: validate_unsigned(&payload.nsweeps, NSWEEPS_MIN, NSWEEPS_MAX)?,
        nf: validate_unsigned(&payload.nf, NF_MIN, NF_MAX)?,
        smax: validate_unsigned(&payload.smax, SMAX_MIN, SMAX_MAX)?,
        local: validate_unsigned(&payload.local, LOCAL_MIN, LOCAL_MAX)?,
        code: payload.code.clone(),
    })
}

async fn execute_code(validated_payload: Payload) -> Result<BackendResult, String> {
    // Create a temporary directory for our project
    let temp_dir =
        TempDir::new().map_err(|e| format!("Failed to create temporary directory: {e}"))?;

    let temp_path = temp_dir.path();
    let src_dir = temp_path.join("src");

    // Create src directory
    fs::create_dir(&src_dir)
        .await
        .map_err(|e| format!("Failed to create src directory: {e}"))?;

    // Create Cargo.toml with project dependencies
    let cargo_toml = r#"
[package]
name = "temp_project"
version = "0.1.0"
edition = "2024"

[dependencies]
nalgebra = "0.33.2"
Rust_MCS = { git = "https://github.com/SergeiGL/Rust_MCS" }
"#;

    let main_rs = format!(
        r#"
use nalgebra::{{SVector, SMatrix}};
use Rust_MCS::*;

fn main() {{
    let nsweeps = {nsweeps};    // maximum number of sweeps
    let nf = {nf};              // maximum number of function evaluations

    const SMAX: usize = {smax};                      // number of levels used
    let local = {local};                             // local search level
    let gamma = 2e-14;                               // acceptable relative accuracy for local search

    {code}

    let hess = SMatrix::<f64, N, N>::repeat(1.);     // sparsity pattern of Hessian


    let (xbest, fbest, _, _, _, _, exitflag) = mcs::<SMAX, N>(func, &u, &v, nsweeps, nf, local, gamma, &hess).unwrap();
    println!("xbest: {{xbest}}");
    println!("fbest: {{fbest:?}}");
    println!("flag: {{exitflag:?}}");
}}"#,
        nsweeps = validated_payload.nsweeps,
        nf = validated_payload.nf,
        smax = validated_payload.smax,
        local = validated_payload.local,
        code = validated_payload.code,
    );

    // Write files to disk
    fs::write(temp_path.join("Cargo.toml"), cargo_toml)
        .await
        .map_err(|e| format!("Failed to write Cargo.toml: {e}"))?;

    fs::write(src_dir.join("main.rs"), main_rs)
        .await
        .map_err(|e| format!("Failed to write main.rs: {e}"))?;

    // Run cargo with tokio for async execution
    let output = Command::new("cargo")
        .current_dir(temp_path)
        .arg("run")
        .arg("--release")
        .output()
        .await
        .map_err(|e| format!("Failed to execute cargo command: {e}"))?;

    if output.status.success() {
        Ok(BackendResult {
            success: true,
            time: SystemTime::now(),
            run_output: Some(String::from_utf8_lossy(&output.stdout).to_string()),
            params_used: Some(validated_payload),
            error: None,
        })
    } else {
        Err(format!(
            "Cargo execution error:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn generate_cache_key(payload: &Payload) -> String {
    let payload_str = serde_json::to_string(payload).unwrap_or_default();

    md5::compute(payload_str)
        .0
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

async fn submit_handler(
    payload: web::Json<Payload>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let cache_key = generate_cache_key(&payload);
    let mut redis_conn = app_state.redis_con.clone();

    // Try to get cached response first
    if let Ok(Some(cached)) = redis_conn.get::<_, Option<String>>(&cache_key).await {
        return HttpResponse::Ok()
            .content_type("application/json")
            .body(cached);
    }

    // Check if other request is in progress
    // error if initially the state was true
    // Do not cache this response
    if app_state
        .request_in_progress
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return HttpResponse::Ok()
            .content_type("application/json")
            .json(BackendResult {
                success: true,
                time: SystemTime::now(),
                run_output: None,
                params_used: None,
                error: Some(
                    "Too many requests. The system is at peak capacity. Try again later."
                        .to_string(),
                ),
            });
    }

    // Process the request if not in cache
    let result = match validate_input(&payload) {
        Ok(validated_payload) => execute_code(validated_payload)
            .await
            .unwrap_or_else(|error| BackendResult {
                success: false,
                time: SystemTime::now(),
                run_output: None,
                params_used: None,
                error: Some(error),
            }),
        Err(error) => BackendResult {
            success: false,
            time: SystemTime::now(),
            run_output: None,
            params_used: None,
            error: Some(error),
        },
    };

    // Convert result to JSON
    let result_json = match serde_json::to_string(&result) {
        Ok(json) => json,
        Err(_) => {
            app_state.request_in_progress.store(false, Ordering::SeqCst);
            return HttpResponse::InternalServerError()
                .content_type("application/json")
                .finish();
        }
    };

    // Store in cache
    let _ = redis_conn
        .set_ex::<_, _, ()>(&cache_key, &result_json, REDIS_CACHE_TTL)
        .await;

    app_state.request_in_progress.store(false, Ordering::SeqCst);

    HttpResponse::Ok()
        .content_type("application/json")
        .body(result_json)
}

async fn get_redis_connection() -> redis::RedisResult<redis::aio::MultiplexedConnection> {
    println!("Redis connection: Starting!");

    // Get the Redis host from the environment variable or default to "127.0.0.1" for local development
    let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    // Initialize Redis connection
    let redis_client = redis::Client::open(format!("redis://{}/", redis_host))?;
    let redis_con = redis_client.get_multiplexed_async_connection().await?;

    println!("Redis connection: Success!");

    Ok(redis_con)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        redis_con: get_redis_connection().await.unwrap(),
        request_in_progress: AtomicBool::new(false),
    });
    println!("Starting server at {IP}");
    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/mcs_form_submit", web::post().to(submit_handler))
    })
    .bind(IP)?
    .run()
    .await
}
