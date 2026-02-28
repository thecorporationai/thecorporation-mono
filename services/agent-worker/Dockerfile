FROM rust:1.85-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/

RUN cargo build --release --bin agent-worker

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/agent-worker /usr/local/bin/agent-worker

# Worker needs access to Docker socket (mounted as volume)
# and workspace directory for agent volumes

ENV RUST_LOG=agent_worker=info

ENTRYPOINT ["agent-worker"]
