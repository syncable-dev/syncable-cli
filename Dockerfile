# Build stage
FROM rust:1.70-alpine AS builder

WORKDIR /app
RUN apk add --no-cache musl-dev

# Copy and build dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy source code and build
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.18

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/target/release/sync-ctl /usr/local/bin/

EXPOSE 8080
ENV RUST_LOG=info
ENV PORT=8080

USER nobody

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD sync-ctl --version || exit 1

ENTRYPOINT ["sync-ctl"]
CMD ["--help"] 