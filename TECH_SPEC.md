# NammaMetro in Charts — Technical Specification

## Overview

**NammaMetro in Charts** is a data-analytical view into Bangalore's metro ridership data, designed for non-technical users who want to explore visualisations. The app fetches a CSV dataset from a public GitHub repository, computes statistics server-side, and renders interactive charts with D3.js progressive enhancement over an SSR fallback.

## Product vision

- **Audience**: Non-technical public-data consumers
- **Scale**: Three interactive chart slides
- **Data source**: Fresh CSV pulled from GitHub on startup, with a background refresh loop
- **Interaction model**: One chart per view; left/right arrows navigate between charts; date range controls recalculate all visualisations
- **Accessibility**: SSR fallback for every chart, keyboard navigation, ARIA labels, reduced-motion support
- **Clean URL**: The URL is always `/`; chart and range state live in `sessionStorage` for the current pageview only and reset on refresh

## Live app

<https://metrodash.fly.dev>

## Architecture

```
Request ──▶ topcoat router ──▶ home() handler
                                   │
                    ┌──────────────┼──────────────┐
                    ▼              ▼              ▼
              fetch_dataset   choose_range   render_view
                    │              │              │
                    ▼              ▼              ├─▶ SSR HTML (view! macro)
              parse_dataset   summarise          │   ├─ CSS (embedded)
                    │              │              │   ├─ JSON payload (embedded)
                    ▼              ▼              │   ├─ D3 + GSAP (CDN)
              Dataset ◀────── RangeSummary       │   └─ Client JS (embedded)
                    │                             │
                    └─────────────────────────────┘
                                  │
                    Client fetches /api/chart on date change
                                  │
                                  ▼
                         Re-render with D3
```

### Request flow

1. **Page load (`GET /`)**: Server fetches (or serves cached) dataset, computes the requested date range and summary, renders full HTML with embedded JSON payload and SSR chart markup.
2. **Date range change (`GET /api/chart`)**: Client JS calls the JSON API, receives a fresh `ChartPayload`, updates sliders/date pickers, and re-renders the active chart with D3.
3. **Chart navigation**: Left/right arrows or arrow keys switch the active chart client-side. The state is stored in `sessionStorage`, not the URL.
4. **Page refresh**: `sessionStorage` is cleared at script start, so the app always resets to the default data-card view with the latest datapoint.

## Framework choice: [Topcoat](https://github.com/tokio-rs/topcoat/)

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
├── main.rs       Entry point, route handlers, AppState, integration tests
├── data.rs       Data types, CSV parsing, dataset loading, statistics
├── charts.rs     Chart registry, SSR markup generation, formatting helpers
├── payload.rs    JSON payload types and construction for /api/chart
├── render.rs     View rendering, query parsing, date range logic, date picker
├── client.rs     Client-side JavaScript (embedded constant)
└── style.rs      CSS (embedded constant)
```

### Dependency graph

```
main.rs ──▶ data.rs, charts.rs, payload.rs, render.rs, client.rs, style.rs
render.rs ──▶ data.rs, charts.rs, payload.rs, client.rs, style.rs
payload.rs ──▶ data.rs, charts.rs
charts.rs ──▶ data.rs
```

## Data pipeline

### Loading

1. On startup, `fetch_dataset()` pulls CSV from `SOURCE_URL` via reqwest
2. If the fetch fails, `load_cached_dataset()` reads from `.cache/`
3. A background task refreshes every 6 hours (configurable via `METRO_REFRESH_SECONDS`)
4. The dataset is stored in `Arc<RwLock<Option<Dataset>>>` for shared access

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
- `quantile()` uses linear interpolation
- Sample standard deviation uses n−1 denominator

## Chart system

Charts are defined as a `const` array of `ChartDefinition` structs in `src/charts.rs`:

1. **Data card (`data-card`)** — Today's ridership, broken down by fare media
2. **Calendar heatmap (`calendar`)** — Daily total ridership, one cell per day, percentile colour bands
3. **Weekday line chart (`commute-casual`)** — Average commuter vs casual ridership by weekday

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

## Client-side architecture

The client JS is a single IIFE embedded in the page. It:

1. Reads the JSON payload from `#chart-data`
2. Renders the active chart with D3 (data card needs no D3 enhancement)
3. Syncs date range controls (sliders ↔ date pickers)
4. Fetches fresh data from `/api/chart` on date range change
5. Keeps chart and range state in `sessionStorage`
6. Handles keyboard navigation (arrow keys for chart switching)
7. Resets `sessionStorage` on every page load so the user always lands on the latest datapoint
8. Respects `prefers-reduced-motion`

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

### Rust unit tests (`#[cfg(test)] mod tests`)

- CSV parsing: aliases, numeric values, dates, duplicates, missing components
- Statistics: quantile interpolation, sample SD, weekday ordering
- Range logic: clamping, snapping, link generation
- Calendar markup: leap day, partial weeks, month boundaries
- Payload: chart readiness, JSON serialization
- Client script: presence of key functions
- Integration: `/api/chart` response, failure view rendering

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

The mapping is explicit in `FieldMapping::from_headers` in `src/main.rs`.

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

### Docker

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

### Fly.io

The included `fly.toml` deploys to Fly.io under the application name `metrodash`, with the primary machine region set to `sin` (Singapore) and `min_machines_running = 1` so a random visitor never hits a cold container.

A reusable named volume `metro_cache` is mounted at `/app/.cache` so the last-known-good CSV survives deploys and machine restarts.

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

```sh
cargo test --all-targets
topcoat dev
```

Open <http://127.0.0.1:3000>. Topcoat binds to `127.0.0.1:3000` by default.

For a production-style local run:

```sh
cargo build --release
HOST=127.0.0.1 PORT=3000 ./target/release/metro-dash
```

For a note on the `topcoat dev` crash caused by shared foreground process groups, see [TOPCOAT-DEV-CRASH.md](TOPCOAT-DEV-CRASH.md).

## Key design decisions

1. **Embedded CSS/JS**: No build step, no CDN for app code (only D3/GSAP from CDN). Trade-off: no HMR, but simplifies deployment.
2. **SSR + D3 dual rendering**: Accessibility-first; SSR is the baseline, D3 is enhancement. Trade-off: duplicated rendering logic.
3. **Sample SD (n−1)**: Standard for descriptive statistics; matches what most users expect from spreadsheet functions.
4. **Missing ≠ zero**: Blank source values remain `None` so averages and sums don't undercount. `sum_complete()` propagates `None` if any component is missing.
5. **Clean URL**: The URL is always `/` and state is `sessionStorage`-only. This removes shareable permalinks but keeps navigation clean for a live-updating dataset.
6. **Module split before chart 3**: Prevents the single file from becoming unmaintainable as more charts are added.
