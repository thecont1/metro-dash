# NammaMetro in Charts — Technical Specification

## Overview

**NammaMetro in Charts** is a data-analytical view into Bangalore's metro ridership data, designed for non-technical users who want to explore visualisations. The app is a static site: the HTML is rendered at build time by the same Rust code that powers the compute, and the chart math runs in the browser via a Rust→WASM core — one source of truth for all statistics. The dataset CSV is fetched live from a public GitHub repository on every page load.

## Product vision

- **Audience**: Non-technical public-data consumers
- **Scale**: Three interactive chart slides
- **Data source**: Fresh CSV pulled from GitHub by the browser on each page load
- **Interaction model**: One chart per view; previous/next controls, a chart jump menu, left/right arrows, and swipe navigate between charts; date range controls recalculate all visualisations locally in WASM
- **Accessibility**: SSR fallback for every chart, keyboard navigation, ARIA labels, reduced-motion support
- **Clean URL**: The URL is always `/`; chart and range state live in `sessionStorage` for the current pageview only and reset on refresh

## Live app

<https://namma.thecontrarian.in>

(Previously served from Fly.io at <https://metrodash.fly.dev>; the static site is the primary deployment, the Rust server binary remains available for self-hosting.)

## Architecture

```
Build time (scripts/build-site.sh)
──────────────────────────────────
  cargo build --release --bin export
        │
        ▼
  export bin ──▶ Router::handle(GET /) ──▶ fetch_dataset (GitHub CSV)
        │                                    │
        │                                    ▼
        │                              parse_dataset → Dataset
        │                                    │
        │                                    ▼
        │                              choose_range → ChartPayload
        │                                    │
        ▼                                    ▼
  site/index.html  ◀── SSR HTML + embedded JSON + CSS/JS
  site/wasm/metro_wasm.{js,wasm}  ◀── wasm-bindgen glue (compute core)
  site/_headers                 ◀── security headers for Pages

Runtime (browser, zero VM)
───────────────────────────
  Page load ──▶ fetch CSV from SOURCE_URL (raw.githubusercontent.com)
        │
        ▼
  dynamic-import metro_wasm.js → init() → loadDataset(csv, now)
        │
        ▼
  Dataset held in WASM memory; embedded payload renders first chart
        │
        ▼
  Date range change ──▶ chartPayload(start, end) locally in WASM
        │
        ▼
  Re-render with D3   (fallback: GET /api/chart on self-hosted server)
```

### Request flow (static deployment)

1. **Page load (`GET /`)**: Cloudflare Pages serves the pre-rendered `index.html`, which embeds the build-time `ChartPayload`, SSR chart markup, CSS, client JS, and pointers to the WASM module.
2. **Boot**: Client JS imports `wasm/metro_wasm.js`, fetches the CSV from `payload.dataset.sourceUrl`, and calls `loadDataset`. `datasetInfo()` refreshes the "data updated" timestamp to the fetch time.
3. **Date range change**: Client calls `chartPayload(start, end)` in WASM — the same Rust functions the server API uses — and re-renders with D3. No network call.
4. **Chart navigation**: Left/right arrows or arrow keys switch the active chart client-side. The state is stored in `sessionStorage`, not the URL.
5. **Page refresh**: `sessionStorage` is cleared at script start, so the app always resets to the default data-card view with the latest datapoint.

### Request flow (self-hosted server, `server` feature)

1. `GET /` — topcoat handler renders HTML with a live dataset (fetched at startup, refreshed every `METRO_REFRESH_SECONDS`).
2. `GET /api/chart?start&end` — JSON payload recomputed in-process.
3. `POST /api/refresh` — webhook that reloads the dataset (used by the sister repo's data workflow before the static migration; retained for self-hosters).

## Framework choice: [Topcoat](https://github.com/tokio-rs/topcoat/)

Topcoat powers the self-hosted server build and the build-time static exporter (`src/bin/export.rs` drives the same `Router` in-process). The production static site needs no server framework at runtime — chart math runs in the browser via the `wasm/` crate.

### Why Topcoat works for this app

| Need | How topcoat addresses it |
|---|---|
| Single-file SSR + JSON API | `#[page("/")]` and `#[route(GET "/api/chart")]` macros co-locate both routes |
| Type-safe HTML | `view!` macro provides compile-time HTML structure validation |
| Zero frontend build step | CSS and JS are embedded as Rust string constants, injected via `Unescaped::new_unchecked()` |
| Shared types between SSR and JSON | `ChartPayload` struct serializes to JSON for the API and is also embedded in the SSR page |
| Async data loading | `app_context` shares `AppState` (cached dataset in `Arc<RwLock<Option<Dataset>>>`) across handlers |
| Background refresh | `tokio::spawn` runs a periodic refresh loop that updates the `RwLock` without blocking requests |
| Testable router | Router can be constructed in tests with mock `AppState` and invoked directly |

### Trade-offs

- **Niche ecosystem**: Limited documentation, few community examples, no middleware marketplace
- **No hot reload for CSS/JS**: Changing embedded assets requires a full Rust recompile
- **Single-file friction**: Addressed by splitting into modules (`src/data.rs`, `src/charts.rs`, `src/payload.rs`, `src/render.rs`, `src/client.rs`, `src/style.rs`)

### Alternatives considered

- **Axum + askama/maud**: Larger ecosystem, similar type-safe SSR, but requires separate template files and an asset pipeline
- **Leptos / Dioxus**: Full-stack Rust with reactive client-side, but overkill for an app with thin client JS

## Module structure

```
src/
├── lib.rs        Crate root: module declarations, re-exports (wasm-compatible)
├── main.rs       Thin server entry point (requires `server` feature)
├── server.rs     topcoat handlers, AppState, security headers, refresh loop
├── bin/
│   └── export.rs Static exporter: drives Router::handle() to write site/index.html
├── data.rs       Data types, CSV parsing, dataset loading, statistics
├── charts.rs     Chart registry, SSR markup generation, formatting helpers
├── payload.rs    JSON payload types and construction for /api/chart and WASM
├── render.rs     View rendering, query parsing, date range logic, date picker
├── client.rs     Client-side JavaScript (embedded constant)
└── style.rs      CSS (embedded constant)

wasm/
├── Cargo.toml    cdylib crate, depends on metro-dash with default-features = false
└── src/lib.rs    wasm-bindgen exports: loadDataset, datasetInfo, chartPayload
```

### Dependency graph

```
lib (no default features) ──▶ data.rs, charts.rs, payload.rs, render.rs, client.rs, style.rs
server.rs (feature = "server") ──▶ lib + topcoat, reqwest, tokio
bin/export.rs ──▶ server.rs (uses the same router the binary serves)
wasm crate ──▶ lib (default-features = false)
render.rs ──▶ data.rs, charts.rs, payload.rs, client.rs, style.rs
payload.rs ──▶ data.rs, charts.rs
charts.rs ──▶ data.rs
```

The `server` cargo feature (on by default) gates `topcoat`, `reqwest`, and `tokio` plus the filesystem/network parts of `data.rs`. `cargo check --no-default-features --target wasm32-unknown-unknown` verifies the compute core stays wasm-clean.

## Data pipeline

### Loading

Two paths feed the same `Dataset` type:

- **Static site (primary)**: the export binary calls `fetch_dataset()` once at build time for the embedded payload; at runtime the browser fetches `SOURCE_URL` itself (the CSV endpoint sends `Access-Control-Allow-Origin: *`) and hands it to `wasm::load_dataset()`.
- **Self-hosted server (`server` feature)**: on startup `fetch_dataset()` pulls the CSV via reqwest; on failure `load_cached_dataset()` reads `.cache/`; a background task refreshes every `METRO_REFRESH_SECONDS` and stores the result in `Arc<RwLock<Option<Dataset>>>`.

### Parsing

- `parse_dataset()` uses the `csv` crate with flexible mode
- `FieldMapping::from_headers()` auto-detects column names (handles aliases)
- `parse_date()` tries multiple date formats (`%d-%m-%Y`, `%Y-%m-%d`, `%d/%m/%Y`, `%Y/%m/%d`)
- `normalise_record()` computes derived fields:
  - `total_ridership`: uses supplied total, or sums all fare media if missing
  - `commuter_ridership`: Smart Card + NCMC
  - `casual_ridership`: Token + QR + Group Ticket
- `sum_complete()` returns `None` if any component is missing (missing ≠ zero)

### Statistics

- `summarise()` computes: calendar days, observation days, missing days, percentile bands (p2–p98), min/max totals, weekday metrics (mean, sample SD, min/max per weekday)
- `weekday_baseline_series()` computes a rolling same-weekday median and interquartile band from up to six earlier and six later published reports; the current day is excluded and at least four nearby reports are required
- `quantile()` uses linear interpolation
- Sample standard deviation uses n−1 denominator

## Chart system

Charts are defined as a `const` array of `ChartDefinition` structs in `src/charts.rs`:

1. **Data card (`data-card`)** — Today's ridership, broken down by fare media
2. **Calendar heatmap (`calendar`)** — Daily total ridership, one cell per day, percentile colour bands
3. **Weekday line chart (`commute-casual`)** — Average commuter vs casual ridership by weekday
4. **Weekday baseline chart (`weekday-baseline`)** — Daily totals compared with a rolling same-weekday median and interquartile band
5. **Component waterfall (`component-waterfall`)** — Fare-media arithmetic contributions to the selected day's difference from its same-weekday reference

### Adding a new chart

1. Add a `Chart` enum variant
2. Add a `ChartDefinition` to `CHARTS`
3. Implement an SSR markup function (returns HTML string)
4. Add payload struct(s) to `ChartPayload`
5. Add a D3 render function to `CLIENT_SCRIPT`
6. Add CSS rules in `src/style.rs` if needed

### Calendar heatmap

- SSR: CSS grid of `<button>` cells with percentile-based band classes
- D3: SVG with animated cell transitions, horizontal scroll for wide ranges, month separator lines
- Percentile bands: 10 buckets from `< p2` to `> p98`, each with a CSS custom property `--band-N`
- Missing days: crossed cells with diagonal lines
- Default range when opened: **Last 6 months**

### Commute vs Casual by Weekday

- SSR: Static SVG with two series (commute, casual)
- D3: Animated area/line drawing, interactive points with tooltips
- Insight line: auto-generated text describing peak days for each series

### Unusual for its Weekday?

- SSR: Static SVG with the observed daily series, rolling same-weekday median, IQR band, point status colours, and an accessible data table
- D3: Time-scaled SVG with keyboard-focusable points and tooltips
- Baseline window: up to six earlier and six later published reports with the same weekday; the current day is excluded
- Missing reports are omitted rather than treated as zero; the observed daily line breaks across gaps longer than seven days

### What Accounts for the Difference?

- SSR: Static SVG waterfall with reference/selected total anchors, floating component deltas, and an accessible data table
- D3: Animated floating bars with keyboard-focusable tooltips
- Reference construction: medians of the same nearby same-weekday reports across all five published parent categories, requiring complete Smart Card, NCMC, Token, QR, and Group Ticket values; leaf/provider sums are deliberately not substituted because QR provider splits do not always reconcile with `Total QR`
- The reference total is the sum of category medians, so reference + deltas = selected-day total exactly

## Client-side architecture

The client JS is a single IIFE embedded in the page. It:

1. Reads the JSON payload from `#chart-data`
2. Dynamically imports `wasm/metro_wasm.js`, fetches the CSV from `payload.dataset.sourceUrl`, and calls `loadDataset` — the dataset then lives in WASM memory
3. Renders the active chart with D3 (data card needs no D3 enhancement)
4. Syncs date range controls (sliders ↔ date pickers)
5. On date range change, calls `chartPayload(start, end)` locally in WASM; if the WASM core is unavailable (e.g. self-hosted server build), falls back to `GET /api/chart`
6. Keeps chart and range state in `sessionStorage`
7. Handles chart navigation (previous/next controls, jump menu, arrow keys, and swipe)
8. Resets `sessionStorage` on every page load so the user always lands on the latest datapoint
9. Respects `prefers-reduced-motion`

### Progressive enhancement

- SSR markup is the baseline (visible without JS)
- D3 scenes are prepended to `.chart-shell` and SSR content is clipped via CSS
- If D3 fails to load (CDN issue), SSR content remains visible

## Date range controls

### UI

- **Dual range sliders**: Start and end date sliders over available dates
- **Date picker**: Custom day/month/year `<select>` elements styled as calendar cells with monospace font
- **Action links**: Quick filters such as **Last 3 months**, **Last 6 months**, **Last 9 months**, **Last one year**, and **Use all available data**, with dynamic `aria-disabled`

### Range logic

- `choose_range()` snaps requested dates to nearest available records
- If start > end, they are swapped
- Default first-visit range: last 90 days of dataset (configurable via `FIRST_VISIT_WINDOW_DAYS` in `src/data.rs`)
- Calendar default on first open: **Last 6 months**

## Testing

### Rust unit tests (`src/tests.rs`, `#[cfg(all(test, feature = "server"))]`)

- CSV parsing: aliases, numeric values, dates, duplicates, missing components
- Statistics: quantile interpolation, sample SD, weekday ordering, centred same-weekday baselines
- Range logic: clamping, snapping, link generation
- Calendar markup: leap day, partial weeks, month boundaries
- Payload: chart readiness, JSON serialization
- Client script: presence of key functions
- Integration: `/api/chart` response, failure view rendering, security headers

### WASM crate tests (`wasm/src/lib.rs`)

- `load_dataset` → `dataset_info` / `chart_payload` round-trip against the shared CSV fixture
- Default-range behaviour and error-before-load paths

### Static export

```sh
cargo run --release --bin export -- --out site   # writes site/index.html
```

The exporter drives the real topcoat `Router` in-process, so the emitted HTML is byte-identical in structure to what the server would render.

### E2E tests (`tests/e2e.py`)

- Playwright-based browser automation
- Tests chart navigation, tooltip visibility, date range interaction
- Checks for console errors
- Captures desktop and mobile screenshots
- Tests mobile viewport overflow

### Running the checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

With the dev server running:

```sh
uv run tests/e2e.py
```

The first run may also need `uv run --with playwright playwright install chromium`.

## Configuration

| Variable | Default | Purpose |
|---|---:|---|
| `HOST` | `127.0.0.1` | HTTP bind host. Use `0.0.0.0` in a container. |
| `PORT` | `3000` | HTTP port. |
| `METRO_CACHE_PATH` | `.cache/namma-metro-ridership.csv` | Last-known-good CSV cache path. The refresh timestamp is stored beside it as `namma-metro-ridership.refreshed-at`. |
| `METRO_REFRESH_SECONDS` | `21600` (6 hours) | Background refresh interval. Values below 60 seconds are clamped to 60. |

## Source-field mapping

Header matching is case-insensitive and ignores spaces and punctuation. The current live CSV maps as follows:

| Derived field | Current source header | Accepted aliases |
|---|---|---|
| Date | `Record Date` | `date`, `journey date` |
| Smart Card | `Total Smart Cards` | `smart card`, `smart cards` |
| NCMC | `Total NCMC` | `NCMC` |
| Token | `Total Tokens` | `token`, `tokens` |
| QR | `Total QR` | `QR`, `QR tickets` |
| Group tickets | `Group Ticket` | `group tickets` |
| Supplied daily total | not currently present | `Total Ridership`, `Total Daily Ridership`, `Total Journeys`, `Total` |

The mapping is explicit in `FieldMapping::from_headers` in `src/data.rs`.

## Methodology

### Normalisation

- Dates are parsed into `chrono::NaiveDate` values, then records are sorted chronologically.
- Numbers are trimmed, commas are removed, non-finite/invalid values become missing, and blank values remain missing—not zero.
- Raw source fields are retained on each parsed record.
- Records with unusable dates are excluded and counted in diagnostics.
- Duplicate dates are resolved deterministically by keeping the last valid row in source order; the decision is counted and logged.

### Derived measures

- `total_ridership`: use the supplied total when it exists and parses successfully; otherwise require all five fare-media components and sum Smart Card + NCMC + Token + QR + Group tickets.
- `commuter_ridership`: Smart Card + NCMC.
- `casual_ridership`: Token + QR + Group tickets.

If any component is missing, that grouped metric is incomplete for the date and excluded from its weekday statistics.

### Percentiles

The heatmap uses the selected range's valid total-ridership observations. Quantiles use linear interpolation between adjacent sorted values at position:

```text
p × (n − 1)
```

The complete ordered bands are `< p2`, `p2–p5`, `p5–p10`, `p10–p25`, `p25–p50`, `p50–p75`, `p75–p90`, `p90–p95`, `p95–p98`, and `> p98`. Comparisons are defined so equal boundaries and ties are always classified. Fewer than 10 valid totals produce an explicit insufficient-data state rather than misleading colour distinctions.

### Weekday statistics

Weekdays are always Monday through Sunday. For each weekday and series the app calculates count, arithmetic mean, minimum, maximum, and sample standard deviation:

```text
sqrt(sum((x − mean)²) / (n − 1))
```

Standard deviation is unavailable for `n < 2`. Missing series observations render as gaps, not zeroes.

## Deployment

### Cloudflare Pages (primary — zero VM)

The production deployment is a static site. Build and publish:

```sh
bash scripts/build-site.sh   # produces site/ (index.html + wasm/ + _headers)
```

Deploy `site/` to a Cloudflare Pages project (direct upload or git integration). `site/_headers` mirrors the security headers the Rust server sends — including a CSP with `'wasm-unsafe-eval'` (required for WASM compilation) and `connect-src` allowing `raw.githubusercontent.com` (the live CSV fetch).

There is no server, no volume, and no refresh webhook: every page load fetches the CSV straight from GitHub, so data is as fresh as the sister repo's last push. The `POST /api/refresh` step was removed from `namma-metro-ridership-tracker`'s workflow accordingly.

A good release gate is:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all-targets` and `cargo test` in `wasm/`
4. `bash scripts/build-site.sh`
5. Smoke test the deployed URL (charts render, console clean, range slider recomputes)

### Docker (self-hosted server)

A multi-stage `Dockerfile` builds a release binary and runs it as a non-root user under `tini`. The container exposes port 3000 and includes a health check against `/`.

```sh
docker build -t metrodash .
docker run --rm -p 3000:3000 \
  -e HOST=0.0.0.0 \
  -e PORT=3000 \
  -v metro-cache:/app/.cache \
  metrodash
```

Any OCI host works if it supplies outbound HTTPS access to GitHub, sets `HOST=0.0.0.0`, routes to `PORT`, and optionally mounts persistent storage at `/app/.cache`.

### Fly.io (self-hosted server, legacy)

`fly.toml` still deploys the Rust server to Fly.io (app `metrodash`, region `sin`, `min_machines_running = 1`) for anyone who wants the always-on `/api/chart` + `/api/refresh` behaviour. The volume `metro_cache` is mounted at `/app/.cache` so the last-known-good CSV survives deploys.

```sh
# One-time app + volume setup, run from a machine that has the fly CLI
# logged in as the deploy principal:
fly launch --copy-config --no-deploy --name metrodash --region sin
fly volumes create metro_cache --size 1 --region sin

# Then on every release:
fly deploy --remote-only --wait-timeout 300
```

A good manual release gate is:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all-targets`
4. `flyctl deploy --remote-only --strategy rolling`
5. Home-page smoke test (`GET https://metrodash.fly.dev/` → 200)
6. `/api/chart` smoke test (verifies SSR-computed payload is being served)

### Region selection

The primary region is `sin` because Fly is phasing out `bom` (Mumbai). If you need to swap regions, edit `fly.toml`'s `primary_region` (and any region list in `[vm]` blocks) before deploying. Latency from Bangalore to `sin` is around 30–40 ms over the public backbone, which is fine for the app's mostly-static traffic; a Tokyo (`nrt`) fallback is a one-line addition if you want a second region in the pool.

## Local development

Requirements:

- Rust 1.95 or newer (required by Topcoat 0.3.1)
- `topcoat-cli` 0.3.x (`cargo install topcoat-cli`)
- `wasm32-unknown-unknown` target + `wasm-bindgen-cli` matching the `wasm-bindgen` crate version (only needed for the static build: `rustup target add wasm32-unknown-unknown && cargo install wasm-bindgen-cli --version 0.2.130`)

```sh
cargo test --all-targets
topcoat dev
```

Open <http://127.0.0.1:3000>. Topcoat binds to `127.0.0.1:3000` by default.

For a production-style local run of the server:

```sh
cargo build --release
HOST=127.0.0.1 PORT=3000 ./target/release/metro-dash
```

For the static site:

```sh
bash scripts/build-site.sh
python3 -m http.server -d site 3000   # or any static file server
```

For a note on the `topcoat dev` crash caused by shared foreground process groups, see [TOPCOAT-DEV-CRASH.md](TOPCOAT-DEV-CRASH.md).

## Key design decisions

1. **Embedded CSS/JS**: No build step, no CDN for app code (only D3/GSAP from CDN). Trade-off: no HMR, but simplifies deployment.
2. **SSR + D3 dual rendering**: Accessibility-first; SSR is the baseline, D3 is enhancement. Trade-off: duplicated rendering logic.
3. **Sample SD (n−1)**: Standard for descriptive statistics; matches what most users expect from spreadsheet functions.
4. **Missing ≠ zero**: Blank source values remain `None` so averages and sums don't undercount. `sum_complete()` propagates `None` if any component is missing.
5. **Clean URL**: The URL is always `/` and state is `sessionStorage`-only. This removes shareable permalinks but keeps navigation clean for a live-updating dataset.
6. **Rust→WASM compute core**: The chart math lives once in Rust, compiled both for the server (`server` feature) and for the browser (`wasm/` crate). One source of truth means the cargo test suite validates exactly the code that runs in production — no TypeScript port to drift.
7. **Module split before chart 3**: Prevents the single file from becoming unmaintainable as more charts are added.
