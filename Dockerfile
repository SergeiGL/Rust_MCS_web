#############################
# Stage 1: Builder
#############################
FROM rust:1.86-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies by copying Cargo manifest files first
COPY Cargo.toml Cargo.lock ./
# Create a dummy main file to allow caching dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
# Build dependencies (this step is cached if Cargo.toml and Cargo.lock don’t change)
RUN cargo build --release
# Remove dummy code
RUN rm -rf src

# Copy the full source code and rebuild the actual binary
COPY . .
RUN cargo build --release

#############################
# Stage 2: Runtime
#############################
FROM rust:1.86-slim-bookworm

# Install runtime dependency (if needed)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*


# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/Rust_MCS_web .

# Expose the port the application listens on
EXPOSE 3001

# Run the binary directly
CMD ["./Rust_MCS_web"]
