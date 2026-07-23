# Pinned base image — avoid surprise rebuilds when :slim floats.
FROM rust:1.95-bookworm AS builder
WORKDIR /app

# Cache dependencies separately from app sources so a source-only change
# doesn't bust the registry+compile layer.
COPY Cargo.toml Cargo.lock ./
COPY vendor/topcoat ./vendor/topcoat
RUN mkdir -p src tests \
    && echo 'fn main() { println!("dep-cache placeholder"); }' > src/main.rs \
    && cargo build --release --locked --bin metro-dash \
    && rm -rf src tests target/release/deps/metro_dash* target/release/metro-dash

COPY src ./src
COPY tests ./tests
RUN cargo build --release --locked --bin metro-dash

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 metro

WORKDIR /app
COPY --from=builder /app/target/release/metro-dash /usr/local/bin/metro-dash
RUN mkdir -p /app/.cache && chown -R metro:metro /app

USER metro
ENV HOST=0.0.0.0 \
    PORT=3000 \
    METRO_CACHE_PATH=/app/.cache/namma-metro-ridership.csv \
    METRO_REFRESH_SECONDS=21600 \
    RUST_BACKTRACE=1 \
    RUST_LOG=info
EXPOSE 3000
VOLUME ["/app/.cache"]

# tini reaps zombies and forwards signals — important because Tokio's
# Ctrl-C / SIGTERM handlers only see the signal once, and a Rust binary
# without an init can leave spawned helpers as zombies on shutdown.
ENTRYPOINT ["/usr/bin/tini", "--"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:${PORT}/ >/dev/null || exit 1

CMD ["metro-dash"]
