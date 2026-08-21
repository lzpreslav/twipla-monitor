# BUILD
FROM rust:alpine3.23@sha256:c4a364ddbf684fe038e6fa6a4f25b30c8dc85247423e0e660676ece0d17be4a2 AS builder

# Build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /app

# Build dependencies against a stub main first, so this layer stays cached
# until Cargo.toml/Cargo.lock change
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src

# touch, or cargo may consider the stub-built binary fresh
RUN touch src/main.rs && cargo build --release

# RUNTIME
FROM alpine:3.23.5@sha256:fd791d74b68913cbb027c6546007b3f0d3bc45125f797758156952bc2d6daf40

# Runtime dependencies
RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/target/release/twipla-monitor /app/twipla-monitor

# Run as non-root user
RUN adduser -D -u 1000 appuser && \
    chown -R appuser:appuser /app

USER appuser

ENTRYPOINT ["/app/twipla-monitor"]
