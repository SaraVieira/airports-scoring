# Stage 1: Build Rust binary
FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

# Build release binary
RUN cargo build --release

# Stage 2: Runtime with Python for sentiment pipeline
FROM debian:bookworm-slim

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libpq5 \
    python3 python3-pip python3-venv \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Rust binary
COPY --from=builder /app/target/release/airport-fetch .

# Copy Python sentiment pipeline
COPY python/ python/
COPY migrations/ migrations/

# Install Python dependencies in a venv
RUN python3 -m venv /app/venv
RUN /app/venv/bin/pip install --no-cache-dir -r python/requirements.txt

ENV PATH="/app/venv/bin:$PATH"
EXPOSE 8080

# Default: start the API server
CMD ["./airport-fetch", "serve", "--port", "8080"]
