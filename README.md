# NammaMetro in Charts

A server-rendered, single-page civic-data story about Bengaluru Metro ridership. It uses Rust and [Topcoat 0.3](https://github.com/tokio-rs/topcoat), fetches the canonical BMRCL ridership CSV server-side, and renders two accessible SVG/HTML chart slides:

1. Daily ridership calendar heatmap
2. Average daily commuter versus casual ridership by weekday

The browser never requests the source CSV directly.

## Local setup

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

## Configuration

| Variable | Default | Purpose |
|---|---:|---|
| `HOST` | `127.0.0.1` | HTTP bind host. Use `0.0.0.0` in a container. |
| `PORT` | `3000` | HTTP port. |
| `METRO_CACHE_PATH` | `.cache/namma-metro-ridership.csv` | Last-known-good CSV cache path. The refresh timestamp is stored beside it as `namma-metro-ridership.refreshed-at`. |
| `METRO_REFRESH_SECONDS` | `21600` (6 hours) | Background refresh interval. Values below 60 seconds are clamped to 60. |

## Data source and refresh behaviour

Canonical source:

<https://raw.githubusercontent.com/thecont1/namma-metro-ridership-tracker/main/NammaMetro_Ridership_Dataset.csv>

At process startup the server:

1. Fetches the source with a 20-second timeout.
2. Rejects non-success HTTP responses and empty bodies.
3. Parses and validates the CSV.
4. Writes a last-known-good CSV and retrieval timestamp to disk.
5. Serves the parsed dataset from shared in-memory application state.

If the initial fetch fails, the app parses and serves the disk cache. If neither source nor cache is usable, it renders an explicit failure page. A background task refreshes the source every six hours by default. A failed refresh never replaces the current dataset.

The footer reports the dataset's actual date extent, row count, latest date, successful retrieval timestamp, selected-range coverage, invalid dates, and deduplicated rows.

The cache directory is runtime state and is excluded from Git. For container hosting, mount a persistent volume at `/app/.cache` if last-known-good data must survive a container replacement.

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

### Date ranges

All calculations use the inclusive selected range. The initial target is 1 January–30 June 2026. Each boundary is snapped to the nearest available observation if the target date is unavailable. Slider thumbs snap only to observed dates; the calendar still renders every intervening calendar day and marks absent observations with a cross.

`start`, `end`, and `chart` are persisted in the URL, for example:

```text
?start=2026-01-01&end=2026-06-30&chart=commute-casual
```

Malformed dates fall back to the clamped default. Reversed bounds are reordered.

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

## Architecture

### Server side

`src/main.rs` contains the Topcoat router, application state, source fetch/cache pipeline, parsing and normalisation, date-range selection, statistical calculations, chart-ready JSON view-model generation, and SSR markup. Topcoat's `#[page("/")]` route returns complete semantic HTML plus an embedded `<script type="application/json" id="chart-data">` payload for the initial selection.

The same Rust payload builder powers `GET /api/chart?start=YYYY-MM-DD&end=YYYY-MM-DD`. Date-range changes fetch this endpoint so the browser can redraw without reloading the whole page while keeping Rust as the source of truth for clamping, missingness, percentiles, weekday aggregation, and labels.

The two chart slides are declared in the typed `CHARTS` registry. Adding another slide requires a `Chart` variant, one registry item, and a renderer branch; navigation labels and positions derive from the registry.

### Client side

A small inline JavaScript enhancement handles the dual range inputs, accessible text-date snapping, URL updates, visible hover/focus/touch tooltips, disabled-link behaviour, and left/right keyboard navigation. The analytical values are computed in Rust and serialized as chart-ready JSON; JavaScript does not recompute percentiles, standard deviations, missingness, or derived ridership categories.

D3 7 is loaded from jsDelivr and owns the enhanced SVG scenes: data joins for calendar cells and weekday line/point/whisker marks, plus transitions when `/api/chart` returns a new payload. The SSR calendar, line chart, legends, and data tables remain in the DOM as accessible fallbacks and are visually clipped only after D3 initializes successfully.

GSAP 3 is also loaded from jsDelivr but is intentionally limited to scene choreography and micro-interactions. It is gated by `prefers-reduced-motion`; D3 transition durations and GSAP choreography collapse when reduced motion is requested.

CSS is self-contained, uses custom properties, responds at 760px, and disables motion under `prefers-reduced-motion`. The app depends on CDN availability for enhanced D3/GSAP rendering, but not for basic access: if either script fails, the SSR chart fallbacks remain visible and usable.

## Tests

Tests use `tests/fixtures/ridership_fixture.csv`, never the live source.

```sh
cargo test --all-targets
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Coverage includes field aliases, date/numeric parsing, missing components, total/commuter/casual derivation, inclusive/clamped ranges, URL links, interpolated quantiles, tied percentile boundaries, sample standard deviation, Monday-first aggregation, deterministic deduplication, leap days, partial weeks, month boundaries, chart JSON payload serialisation, the `/api/chart` route, and failure-page rendering.

With the dev server running, execute browser checks with `uv`. The first run may also need `uv run --with playwright playwright install chromium`:

```sh
uv run tests/e2e.py
```

This verifies embedded chart JSON, D3 scene initialisation, calendar/line navigation, focus tooltips, URL restoration, text-date recovery through `/api/chart`, console errors, mobile overflow containment, and produces screenshots in `/tmp`.

## Deployment

A multi-stage `Dockerfile` builds a release binary and runs it as a non-root user under `tini`. The container exposes port 3000 and includes a health check against `/`.

```sh
docker build -t namma-metro-in-charts .
docker run --rm -p 3000:3000 \
  -e HOST=0.0.0.0 \
  -e PORT=3000 \
  -v metro-cache:/app/.cache \
  namma-metro-in-charts
```

### Fly.io

The included `fly.toml` deploys to Fly.io under the application name `namma-metro-in-charts`, with the primary machine region set to `sin` (Singapore) and `min_machines_running = 1` so a random visitor never hits a cold container.

A reusable named volume `metro_cache` is mounted at `/app/.cache` so the last-known-good CSV survives deploys and machine restarts.

```sh
# One-time app + volume setup, run from a machine that has the fly CLI
# logged in as the deploy principal:
fly launch --copy-config --no-deploy --name namma-metro-in-charts --region sin
fly volumes create metro_cache --size 1 --region sin

# Then on every release:
fly deploy --remote-only --wait-timeout 300
```

Continuous deploys run via `.github/workflows/deploy-fly.yml` on every push to `main`. The job gate is:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all-targets`
4. `flyctl deploy --remote-only --strategy rolling`
5. Home-page smoke test (`GET https://namma-metro-in-charts.fly.dev/` → 200)
6. `/api/chart` smoke test (verifies SSR-computed payload is being served)

The Fly deploy token must be stored as the `FLY_API_TOKEN` secret in the GitHub repository settings (`https://github.com/thecont1/metro-dash/settings/secrets/actions`). Generate it with `fly tokens create deploy`.

Any OCI host works if it supplies outbound HTTPS access to GitHub, sets `HOST=0.0.0.0`, routes to `PORT`, and optionally mounts persistent storage at `/app/.cache`.

### Region selection

The primary region is `sin` because Fly is phasing out `bom` (Mumbai). If you need to swap regions, edit `fly.toml`'s `primary_region` (and any region list in `[vm]` blocks) before deploying. Latency from Bangalore to `sin` is around 30–40 ms over the public backbone, which is fine for the app's mostly-static traffic; a Tokyo (`nrt`) fallback is a one-line addition if you want a second region in the pool.
