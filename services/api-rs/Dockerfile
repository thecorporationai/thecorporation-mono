# ── Builder ────────────────────────────────────────────────────────────────
FROM rust:slim AS builder

RUN apt-get update && apt-get install -y libssl-dev pkg-config python3 && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy shared crates
COPY crates/agent-types /build/crates/agent-types
COPY crates/corp-store /build/crates/corp-store

# Copy governance AST (referenced via include_str! from policy_ast.rs)
COPY governance /build/governance

# Copy api-rs source (Cargo.toml references ../../crates/agent-types)
COPY services/api-rs/Cargo.toml services/api-rs/Cargo.lock services/api-rs/build.rs /build/services/api-rs/
COPY services/api-rs/src /build/services/api-rs/src

WORKDIR /build/services/api-rs
RUN cargo build --release

# ── Runtime ───────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y git libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/services/api-rs/target/release/api-rs /usr/local/bin/api-rs

EXPOSE 8000

CMD ["api-rs"]
