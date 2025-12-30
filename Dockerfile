# Build stage
FROM rust:1.84-bookworm AS builder

# Install nightly toolchain
RUN rustup default nightly && rustup update nightly

WORKDIR /app

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src
COPY migrations ./migrations
COPY templates ./templates

# Touch main.rs to invalidate the cache and rebuild with actual code
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --user-group app
USER app
WORKDIR /home/app

# Copy binary from builder
COPY --from=builder /app/target/release/personal-os ./personal-os

# Copy static assets and templates
COPY --chown=app:app src/static ./src/static
COPY --chown=app:app templates ./templates
COPY --chown=app:app migrations ./migrations

# Expose port
EXPOSE 3000

# Set environment variables
ENV DATABASE_URL=sqlite:data.db?mode=rwc
ENV RUST_LOG=personal_os=info,tower_http=info
ENV HOST=0.0.0.0
ENV PORT=3000

# Run the application
CMD ["./personal-os"]
