# NammaMetro in Charts — Technical Specification

## Overview

**NammaMetro in Charts** is a data-analytical view into Bangalore's metro ridership data, designed for non-technical users who want to explore visualisations. The app fetches a CSV dataset from a public GitHub repository, computes statistics server-side, and renders interactive charts with D3.js progressive enhancement over an SSR fallback.

## Product Vision

- **Audience**: Non-technical public-data consumers
- **Scale**: 5–10 interactive charts
- **Data source**: Fresh CSV pulled from GitHub on page load, with a background refresh loop
- **Interaction model**: One chart per view; left/right arrows navigate between charts; date range controls recalculate all visualisations
- **Accessibility**: SSR fallback for every chart, keyboard navigation, ARIA labels, reduced-motion support

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
3. **Chart navigation**: Left/right arrows or arrow keys switch the active chart client-side (URL updated via `pushState`).

## Framework choice: [Topcoat](https://github.com/tokio-rs/topcoat/)

### Why **topcoat** works for this app

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
- **Single-file friction**: At 2700+ lines for 2 charts, the file becomes unwieldy (addressed by module split — see below)

### Alternatives considered

- **Axum + askama/maud**: Larger ecosystem, similar type-safe SSR, but requires a separate template files and asset pipeline
- **Leptos / Dioxus**: Full-stack Rust with reactive client-side, but overkill for an app with thin client JS

## Module structure

The application is split into the following modules:

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

### Chart registry

Charts are defined as a `const` array of `ChartDefinition` structs:

```rust
const CHARTS: [ChartDefinition; 2] = [
    ChartDefinition { chart: Chart::Calendar, slug: "calendar", title: "Daily Total Ridership", ... },
    ChartDefinition { chart: Chart::CommuteCasual, slug: "commute-casual", title: "Commute vs Casual by Weekday", ... },
];
```

Adding a new chart requires:
1. Add a `Chart` enum variant
2. Add a `ChartDefinition` to `CHARTS`
3. Implement SSR markup function (returns HTML string)
4. Add payload struct(s) to `ChartPayload`
5. Add D3 render function to `CLIENT_SCRIPT`

### Calendar heatmap

- SSR: CSS grid of `<button>` cells with percentile-based band classes
- D3: SVG with animated cell transitions, horizontal scroll for wide ranges
- Percentile bands: 10 buckets from `< p2` to `> p98`, each with a CSS custom property `--band-N`
- Missing days: crossed cells with diagonal background pattern

### Weekday line chart

- SSR: Static SVG with two series (commute, casual), whisker lines for ±1 SD
- D3: Animated line drawing, interactive points with tooltips
- Insight line: Auto-generated text describing peak days for each series

## Client-side architecture

The client JS is a single IIFE embedded in the page. It:

1. Reads the JSON payload from `#chart-data`
2. Renders the active chart with D3 (calendar or line)
3. Syncs date range controls (sliders ↔ date pickers)
4. Fetches fresh data from `/api/chart` on date range change
5. Updates URL via `pushState` for shareable links
6. Handles keyboard navigation (arrow keys for chart switching)
7. Respects `prefers-reduced-motion`

### Progressive enhancement

- SSR markup is the baseline (visible without JS)
- D3 scenes are prepended to `.chart-shell` and SSR content is clipped via CSS
- If D3 fails to load (CDN issue), SSR content remains visible

## Date range controls

### UI

- **Dual range sliders**: Start and end date sliders over available dates
- **Date picker**: Custom day/month/year `<select>` elements styled as calendar cells with monospace font
- **Action links**: "Reset to Jan–Jun 2026" and "Use all available data" with dynamic `aria-disabled`

### Range logic

- `choose_range()` snaps requested dates to nearest available records
- If start > end, they are swapped
- Default first-visit range: last 90 days of dataset (configurable via `FIRST_VISIT_WINDOW_DAYS`)
- Reset range: `DEFAULT_START` to `DEFAULT_END` (Jan–Jun 2026), snapped to available dates

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

## Deployment

- **Platform**: Fly.io (or any container host)
- **Build**: `cargo build --release`
- **Runtime**: Single binary, no external assets (CSS/JS embedded)
- **Environment variables**:
  - `METRO_CACHE_PATH`: Override cache file location
  - `METRO_REFRESH_SECONDS`: Override refresh interval (minimum 60)

## Key design decisions

1. **Embedded CSS/JS**: No build step, no CDN for app code (only D3/GSAP from CDN). Trade-off: no HMR, but simplifies deployment.
2. **SSR + D3 dual rendering**: Accessibility-first; SSR is the baseline, D3 is enhancement. Trade-off: duplicated rendering logic.
3. **Sample SD (n−1)**: Standard for descriptive statistics; matches what most users expect from spreadsheet functions.
4. **Missing ≠ zero**: Blank source values remain `None` so averages and sums don't undercount. `sum_complete()` propagates `None` if any component is missing.
5. **Module split before chart 3**: Prevents the single file from becoming unmaintainable at 5–10 charts.
