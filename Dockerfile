# Build stage
FROM public.ecr.aws/docker/library/rust:slim-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        libpq-dev \
        libssl-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml ./
COPY crates/ ./crates/

# Generate new Cargo.lock and build dependencies
RUN cargo build --release

# Runtime stage
FROM public.ecr.aws/docker/library/debian:bookworm-slim

# Install runtime dependencies including PostgreSQL client
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/recon /app/recon

# Copy configuration and migrations
COPY config/ /app/config/
COPY migrations/ /app/migrations/
COPY diesel.toml /app/diesel.toml

# Create a non-root user and logs directory
RUN useradd --user-group --system --no-create-home --no-log-init appuser && \
    mkdir -p /app/logs && \
    chown -R appuser:appuser /app
USER appuser

EXPOSE 8080

CMD ["./recon"]
