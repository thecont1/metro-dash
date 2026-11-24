use std::{sync::Arc, time::Duration};

use reqwest::Client;
use serde::Serialize;
use tokio::sync::RwLock;
use topcoat::{
    Result,
    asset::AssetBundle,
    context::{Cx, CxBuilder},
    router::{
        Body, IntoResponse, Json, Next, Response, Router, RouterBuilderDiscoverExt, layer, page,
        parts, route,
    },
};

use crate::charts::Chart;
use crate::data::{Dataset, choose_range, fetch_dataset, load_cached_dataset};
use crate::payload::{ChartPayload, chart_payload_for};
use crate::render::{QueryState, failure_view, parse_query, render_view};

#[derive(Debug, Clone)]
pub struct AppState {
    pub dataset: Arc<RwLock<Option<Dataset>>>,
    pub client: Client,
}

/// Build the application router. Shared by the HTTP server (`main`) and the
/// static exporter (`bin/export`) so both render byte-identical pages.
pub fn app_router(state: AppState) -> Router {
    Router::builder()
        .discover()
        .app_context(state)
        .app_context(AssetBundle::empty())
        .build()
}

/// Fetch the dataset, falling back to the last-known-good cache. Returns
/// `None` when neither source has usable data.
pub async fn load_initial_dataset(client: &Client) -> Option<Dataset> {
    match fetch_dataset(client).await {
        Ok(dataset) => {
            println!("Loaded {} ridership records", dataset.records.len());
            Some(dataset)
        }
        Err(error) => {
            eprintln!("Initial ridership fetch failed: {error}");
            match load_cached_dataset() {
                Ok(dataset) => {
                    eprintln!(
                        "Serving {} records from the last-known-good cache",
                        dataset.records.len()
                    );
                    Some(dataset)
                }
                Err(cache_error) => {
                    eprintln!("Last-known-good cache unavailable: {cache_error}");
                    None
                }
            }
        }
    }
}

/// Spawn the background refresh loop that re-fetches the dataset every
/// `METRO_REFRESH_SECONDS` (default 6h, clamped to >= 60s).
pub fn spawn_refresh_loop(dataset_slot: Arc<RwLock<Option<Dataset>>>, client: Client) {
    let interval_secs: u64 = std::env::var("METRO_REFRESH_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(21600) // 6 hours
        .max(60);
    let interval = Duration::from_secs(interval_secs);
    if interval_secs < 21600 {
        eprintln!("Refresh interval clamped below 6h: {interval_secs}s");
    }
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.tick().await; // skip the immediate first tick
        loop {
            ticker.tick().await;
            match fetch_dataset(&client).await {
                Ok(dataset) => {
                    *dataset_slot.write().await = Some(dataset.clone());
                    println!(
                        "Background refresh: {} records (latest: {})",
                        dataset.records.len(),
                        dataset.max_date
                    );
                }
                Err(error) => {
                    eprintln!("Background refresh failed: {error}");
                }
            }
        }
    });
}

/// Attach baseline security headers to every response.
///
/// The app is served directly from Fly.io (`namma.thecontrarian.in` is not
/// proxied through Cloudflare), so there is no edge tier to add these for
/// us — the origin is the only place they can come from.
///
/// `force_https = true` in fly.toml redirects HTTP to HTTPS, but a redirect
/// alone still leaves the first request open to interception; HSTS is what
/// tells the browser never to try plaintext again.
#[layer("/")]
async fn security_headers(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> {
    // An `Err` from a handler is converted into a response by the router
    // *after* this layer returns, so propagating it with `?` would skip the
    // headers entirely. Error responses — 404s, 500s, the failure view — are
    // exactly the ones a prober sees most, so convert here and header both
    // branches identically. `CxBuilder` derefs to `Cx`, which is what
    // `IntoResponse` needs.
    let mut response = match next.run(cx, body).await {
        Ok(response) => response,
        Err(error) => error.into_response(&*cx)?,
    };
    let headers = response.headers_mut();

    // 2 years, matching the preload-list minimum. No `preload` directive:
    // that is a one-way door for the whole apex domain and must be a
    // deliberate choice, not a side effect of a security pass.
    headers.insert(
        "strict-transport-security",
        "max-age=63072000; includeSubDomains".parse().unwrap(),
    );
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert(
        "referrer-policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    headers.insert(
        "permissions-policy",
        "geolocation=(), microphone=(), camera=(), interest-cohort=()"
            .parse()
            .unwrap(),
    );
    // The dashboard inlines its chart script and styles (see client.rs /
    // style.rs), so 'unsafe-inline' is load-bearing here. It also loads d3
    // and gsap as classic <script src> tags from jsdelivr, so those two
    // files — and nothing else — are allowed to execute. Pin the exact
    // versioned paths rather than the whole CDN origin: a bare
    // `https://cdn.jsdelivr.net` would let any package on that CDN run in
    // the page. Bumping d3/gsap means updating these two URLs to match.
    //
    // 'wasm-unsafe-eval' permits WebAssembly compilation (the chart compute
    // core shipped as a wasm module) without unlocking full 'unsafe-eval'.
    // connect-src additionally allows the one cross-origin fetch the client
    // makes: the public ridership CSV on raw.githubusercontent.com.
    headers.insert(
        "content-security-policy",
        "default-src 'self'; \
         script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval' \
             https://cdn.jsdelivr.net/npm/d3@7.9.0/dist/d3.min.js \
             https://cdn.jsdelivr.net/npm/gsap@3.12.5/dist/gsap.min.js; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data:; \
         font-src 'self' data:; \
         connect-src 'self' https://raw.githubusercontent.com; \
         object-src 'none'; \
         base-uri 'none'; \
         form-action 'self'; \
         frame-ancestors 'none'"
            .parse()
            .unwrap(),
    );

    Ok(response)
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    let state: &AppState = topcoat::context::app_context(cx);
    let query = parse_query(parts(cx).uri.query().unwrap_or_default());
    let mut snapshot = state.dataset.read().await.clone();
    if snapshot.is_none()
        && query.retry
        && let Ok(dataset) = fetch_dataset(&state.client).await
    {
        *state.dataset.write().await = Some(dataset.clone());
        snapshot = Some(dataset);
    }
    match snapshot {
        Some(dataset) => {
            // The client owns chart and range state (sessionStorage); the server
            // always renders the default landing view.
            let chart = Chart::DataCard;
            let query = QueryState {
                chart: Some(chart.slug().to_string()),
                ..Default::default()
            };
            render_view(cx, dataset, query, chart).await
        }
        None => failure_view(cx).await,
    }
}

#[route(GET "/api/chart")]
async fn chart_payload(cx: &Cx) -> Result<Json<ChartPayload>> {
    let state: &AppState = topcoat::context::app_context(cx);
    let query = parse_query(parts(cx).uri.query().unwrap_or_default());
    let snapshot = state.dataset.read().await.clone();
    let Some(dataset) = snapshot else {
        return Err(topcoat::Error::from(std::io::Error::other(
            "ridership dataset unavailable",
        )));
    };
    let range = choose_range(&dataset, query.start.as_deref(), query.end.as_deref());
    Ok(Json(chart_payload_for(&dataset, range)))
}

#[derive(Serialize)]
struct RefreshResponse {
    row_count: usize,
    max_date: String,
    refreshed_at: String,
}

/// Compare two byte strings in time independent of their contents.
///
/// A plain `!=` on secrets returns as soon as it hits the first differing
/// byte, so response latency leaks how many leading bytes an attacker got
/// right — enough to recover the secret one byte at a time over many
/// requests. This folds every byte into the accumulator before deciding.
///
/// The length check short-circuits, which leaks only the secret's length —
/// not its content — and that is already bounded by the configuration.
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    // `fold` over every byte: no early exit, so the loop runs the same
    // number of iterations regardless of where (or whether) they differ.
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[route(POST "/api/refresh")]
async fn refresh(cx: &Cx) -> Result<Json<RefreshResponse>> {
    let state: &AppState = topcoat::context::app_context(cx);

    let expected = std::env::var("REFRESH_SECRET").unwrap_or_default();
    if expected.is_empty() {
        return Err(topcoat::Error::from(std::io::Error::other(
            "refresh secret is not configured",
        )));
    }

    let provided = parts(cx)
        .headers
        .get("x-refresh-secret")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return Err(topcoat::Error::from(std::io::Error::other(
            "invalid refresh secret",
        )));
    }

    let dataset = fetch_dataset(&state.client)
        .await
        .map_err(|error| topcoat::Error::from(std::io::Error::other(error)))?;
    let row_count = dataset.records.len();
    let max_date = dataset.max_date.to_string();
    let refreshed_at = dataset.refreshed_at.clone();
    *state.dataset.write().await = Some(dataset);

    Ok(Json(RefreshResponse {
        row_count,
        max_date,
        refreshed_at,
    }))
}
