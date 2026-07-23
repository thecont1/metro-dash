FROM rust:1.95-slim-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY vendor/topcoat ./vendor/topcoat
COPY src ./src
RUN cargo build --locked --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 metro

WORKDIR /app
COPY --from=builder /app/target/release/metro-dash /usr/local/bin/metro-dash
RUN mkdir -p /app/.cache && chown -R metro:metro /app

USER metro
ENV HOST=0.0.0.0 \
    PORT=3000 \
    METRO_CACHE_PATH=/app/.cache/namma-metro-ridership.csv \
    METRO_REFRESH_SECONDS=21600
EXPOSE 3000
VOLUME ["/app/.cache"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:${PORT}/ >/dev/null || exit 1

CMD ["metro-dash"]
