# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/polymarket_arb_hft /app/
COPY --from=builder /app/target/release/*.so /app/ 2>/dev/null || true

COPY --chown=appuser:appuser frontend/ /app/frontend/
RUN chown -R appuser:appuser /app
USER appuser

ENV PORT=8080
EXPOSE 8080

CMD ["/app/polymarket_arb_hft", "serve", "--port", "8080", "--frontend", "/app/frontend"]
