FROM rust:1.86-slim-bookworm

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

RUN cargo build --release

ENV REDIS_HOST=redis

EXPOSE 4004

ENTRYPOINT ["/app/target/release/Rust_MCS_web"]

