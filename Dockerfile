# BUILD
FROM --platform=$BUILDPLATFORM rust:alpine3.23@sha256:f6c22e0a256c05d44fca23bf530120b5d4a6249a393734884281ca80782329bc AS builder

# Build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static

WORKDIR /app

COPY . ./

RUN cargo build --release

# RUNTIME
FROM alpine:3.23.2@sha256:865b95f46d98cf867a156fe4a135ad3fe50d2056aa3f25ed31662dff6da4eb62

# Runtime dependencies
RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/target/release/twipla-monitor /app/twipla-monitor

# Run as non-root user
RUN adduser -D -u 1000 appuser && \
    chown -R appuser:appuser /app

USER appuser

ENTRYPOINT ["/app/twipla-monitor"]
