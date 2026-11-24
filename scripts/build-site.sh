#!/usr/bin/env bash
# Assemble the static site into site/:
#   site/index.html  — SSR'd page exported in-process from the Rust router
#   site/wasm/        — wasm-bindgen glue + compute core (.js + _bg.wasm)
#   site/_headers     — Cloudflare Pages security headers (checked in)
#
# The client fetches the ridership CSV live from GitHub on each page load,
# so index.html only needs regenerating when the app code or the SSR
# snapshot itself should be refreshed.
set -euo pipefail
cd "$(dirname "$0")/.."

WASM_BINDGEN_VERSION="$(awk -F'"' '/^wasm-bindgen = /{gsub("=", "", $2); print $2; exit}' wasm/Cargo.toml)"

echo "==> Building wasm compute core (wasm32-unknown-unknown)"
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
cargo build --release --target wasm32-unknown-unknown --manifest-path wasm/Cargo.toml

INSTALLED="$(wasm-bindgen --version | awk '{print $2}')"
if [ "$INSTALLED" != "$WASM_BINDGEN_VERSION" ]; then
  echo "wasm-bindgen CLI $INSTALLED != crate $WASM_BINDGEN_VERSION; installing matching CLI"
  cargo install wasm-bindgen-cli --version "$WASM_BINDGEN_VERSION" --locked
fi

echo "==> Generating wasm-bindgen glue"
mkdir -p site/wasm
wasm-bindgen --target web \
  --out-dir site/wasm --out-name metro_wasm \
  wasm/target/wasm32-unknown-unknown/release/metro_wasm.wasm

echo "==> Building static exporter"
cargo build --release --bin export

echo "==> Exporting index.html"
./target/release/export --out site

echo "==> Done: site/ is ready to deploy"
