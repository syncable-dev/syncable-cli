# syntax=docker/dockerfile:1.7

# syncable-cli uses Rust 2024 edition and depends on crates that currently require nightly.
ARG RUST_VERSION=1.85.0
ARG RUST_NIGHTLY=nightly-2025-12-01
ARG DEBIAN_VERSION=bookworm

############################
# Builder
############################
FROM rust:${RUST_VERSION}-slim-${DEBIAN_VERSION} AS builder

# System deps for building crates that rely on OpenSSL (e.g., reqwest default-tls)
RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    pkg-config \
    libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# Install a pinned nightly toolchain (reproducible) for crates requiring unstable features.
ARG RUST_NIGHTLY
RUN rustup toolchain install "${RUST_NIGHTLY}" --profile minimal \
  && rustup default "${RUST_NIGHTLY}"

WORKDIR /app

# Where we'll place build artifacts that must persist between layers
RUN mkdir -p /out

# 1) Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
  && printf 'fn main() { println!("dep-cache"); }\n' > src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked

# 2) Build the real binary
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --bin sync-ctl \
 && cp /app/target/release/sync-ctl /out/sync-ctl

############################
# Runtime
############################
FROM debian:${DEBIAN_VERSION}-slim AS final

# Minimal runtime deps:
# - ca-certificates for HTTPS
# - libssl3 for native-tls (OpenSSL) at runtime
RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
  && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /usr/sbin/nologin --uid 10001 appuser

WORKDIR /home/appuser

COPY --from=builder /out/sync-ctl /usr/local/bin/sync-ctl

USER 10001:10001

# Basic healthcheck: verifies the binary can start
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/usr/local/bin/sync-ctl", "--version"]

ENTRYPOINT ["/usr/local/bin/sync-ctl"]
CMD ["--help"]
