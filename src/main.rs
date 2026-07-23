use std::{collections::HashMap, fmt::Write as _, fs, path::PathBuf, sync::Arc, time::Duration};

use chrono::{Datelike, NaiveDate, Weekday};
use csv::StringRecord;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use topcoat::{
    Result,
    asset::AssetBundle,
    context::Cx,
    router::{Json, Router, RouterBuilderDiscoverExt, page, parts, route},
    view::{Unescaped, view},
};

const SOURCE_URL: &str = "https://raw.githubusercontent.com/thecont1/namma-metro-ridership-tracker/main/NammaMetro_Ridership_Dataset.csv";
const SOURCE_PAGE_URL: &str = "https://github.com/thecont1/namma-metro-ridership-tracker/blob/main/NammaMetro_Ridership_Dataset.csv";
const DEFAULT_START: &str = "2026-01-01";
const DEFAULT_END: &str = "2026-06-30";
const DEFAULT_CACHE_PATH: &str = ".cache/namma-metro-ridership.csv";

#[derive(Debug, Clone, PartialEq)]
pub struct RidershipRecord {
    pub date: NaiveDate,
    pub raw_fields: HashMap<String, String>,
    pub total_ridership: Option<f64>,
    pub smart_card: Option<f64>,
    pub ncmc: Option<f64>,
    pub token: Option<f64>,
    pub qr: Option<f64>,
    pub group_ticket: Option<f64>,
    pub commuter_ridership: Option<f64>,
    pub casual_ridership: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dataset {
    pub records: Vec<RidershipRecord>,
    pub min_date: NaiveDate,
    pub max_date: NaiveDate,
    pub invalid_date_count: usize,
    pub duplicate_count: usize,
    pub field_mapping: FieldMapping,
    pub refreshed_at: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldMapping {
    pub date: String,
    pub total: Option<String>,
    pub smart_card: Option<String>,
    pub ncmc: Option<String>,
    pub token: Option<String>,
    pub qr: Option<String>,
    pub group_ticket: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeSummary {
    pub calendar_days: usize,
    pub observation_days: usize,
    pub missing_days: usize,
    pub percentiles: Vec<(u8, f64)>,
    pub valid_total_count: usize,
    pub total_min: Option<f64>,
    pub total_max: Option<f64>,
    pub commute: Vec<WeekdayMetric>,
    pub casual: Vec<WeekdayMetric>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeekdayMetric {
    pub weekday: Weekday,
    pub n: usize,
    pub mean: Option<f64>,
    pub standard_deviation: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChartPayload {
    range: PayloadRange,
    dataset: PayloadDataset,
    summary: PayloadSummary,
    charts: PayloadCharts,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PayloadRange {
    start: String,
    end: String,
    label: String,
    start_index: usize,
    end_index: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PayloadDataset {
    min_date: String,
    max_date: String,
    row_count: usize,
    refreshed_at: String,
    available_dates: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PayloadSummary {
    calendar_days: usize,
    observation_days: usize,
    missing_days: usize,
    valid_total_count: usize,
    total_min: Option<f64>,
    total_max: Option<f64>,
    total_min_label: Option<String>,
    total_max_label: Option<String>,
    insufficient_percentiles: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PayloadCharts {
    calendar: CalendarPayload,
    commute_casual: CommuteCasualPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarPayload {
    cells: Vec<CalendarCellPayload>,
    legend: Vec<LegendItemPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarCellPayload {
    date: String,
    label: String,
    weekday: usize,
    week: usize,
    month_label: Option<String>,
    total: Option<f64>,
    total_label: Option<String>,
    band_label: Option<String>,
    band_index: Option<usize>,
    missing: bool,
    breakdown: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegendItemPayload {
    label: &'static str,
    band_index: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommuteCasualPayload {
    weekdays: Vec<WeekdayPayload>,
    series: Vec<SeriesPayload>,
    insight: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WeekdayPayload {
    index: usize,
    short: &'static str,
    name: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SeriesPayload {
    key: &'static str,
    label: &'static str,
    points: Vec<WeekdayPointPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WeekdayPointPayload {
    weekday: &'static str,
    weekday_short: &'static str,
    index: usize,
    n: usize,
    mean: Option<f64>,
    mean_label: Option<String>,
    standard_deviation: Option<f64>,
    standard_deviation_label: Option<String>,
    min: Option<f64>,
    min_label: Option<String>,
    max: Option<f64>,
    max_label: Option<String>,
    tooltip: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub dataset: Arc<RwLock<Option<Dataset>>>,
    pub client: Client,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct QueryState {
    start: Option<String>,
    end: Option<String>,
    chart: Option<String>,
    retry: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Chart {
    Calendar,
    CommuteCasual,
}

#[derive(Debug, Clone, Copy)]
struct ChartDefinition {
    chart: Chart,
    slug: &'static str,
    title: &'static str,
    deck: &'static str,
    short_name: &'static str,
}

const CHARTS: [ChartDefinition; 2] = [
    ChartDefinition {
        chart: Chart::Calendar,
        slug: "calendar",
        title: "Daily ridership calendar",
        deck: "Each square is one calendar day. Deeper mauve means higher total ridership within the selected period.",
        short_name: "Calendar",
    },
    ChartDefinition {
        chart: Chart::CommuteCasual,
        slug: "commute-casual",
        title: "Average daily ridership: Commute vs casual",
        deck: "Average journeys by day of week, using the dates currently selected above.",
        short_name: "Commute vs casual",
    },
];

impl Chart {
    fn from_query(value: Option<&str>) -> Self {
        CHARTS
            .iter()
            .find(|definition| Some(definition.slug) == value)
            .map(|definition| definition.chart)
            .unwrap_or(Self::Calendar)
    }
    fn slug(self) -> &'static str {
        self.definition().slug
    }
    fn definition(self) -> &'static ChartDefinition {
        CHARTS
            .iter()
            .find(|definition| definition.chart == self)
            .expect("every chart variant is registered")
    }
    fn index(self) -> usize {
        CHARTS
            .iter()
            .position(|definition| definition.chart == self)
            .expect("every chart variant is registered")
    }
}

#[tokio::main]
async fn main() {
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("NammaMetro-in-Charts/0.1")
        .build()
        .expect("HTTP client should build");
    let initial_dataset = match fetch_dataset(&client).await {
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
    };
    let state = AppState {
        dataset: Arc::new(RwLock::new(initial_dataset)),
        client,
    };
    let refresh_state = state.clone();
    tokio::spawn(async move {
        let refresh_seconds = std::env::var("METRO_REFRESH_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(21_600)
            .max(60);
        let mut interval = tokio::time::interval(Duration::from_secs(refresh_seconds));
        interval.tick().await;
        loop {
            interval.tick().await;
            match fetch_dataset(&refresh_state.client).await {
                Ok(dataset) => {
                    println!("Refreshed {} ridership records", dataset.records.len());
                    *refresh_state.dataset.write().await = Some(dataset);
                }
                Err(error) => {
                    eprintln!("Ridership refresh failed; retaining last-known-good data: {error}");
                }
            }
        }
    });
    let router = Router::builder()
        .discover()
        .app_context(state)
        .app_context(AssetBundle::empty())
        .build();
    topcoat::start(router).await.unwrap();
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    let state: &AppState = topcoat::context::app_context(cx);
    let query = parse_query(parts(cx).uri.query().unwrap_or_default());
    let mut snapshot = state.dataset.read().await.clone();
    if snapshot.is_none() && query.retry {
        // The visitor asked to retry after an initial-load failure and we have
        // no last-known-good data yet, so attempt a fresh live fetch inline.
        if let Ok(dataset) = fetch_dataset(&state.client).await {
            *state.dataset.write().await = Some(dataset.clone());
            snapshot = Some(dataset);
        }
    }
    let chart = Chart::from_query(query.chart.as_deref());
    match snapshot {
        Some(dataset) => render_view(cx, dataset, query, chart).await,
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

async fn render_view(cx: &Cx, dataset: Dataset, query: QueryState, chart: Chart) -> Result {
    let range = choose_range(&dataset, query.start.as_deref(), query.end.as_deref());
    let summary = summarise(&dataset, range);
    let payload_json = serde_json::to_string(&chart_payload_for(&dataset, range))
        .expect("chart payload should serialize")
        .replace("</", "<\\/");
    let range_label = format_range(range);
    let previous_href = format!("?start={}&end={}&chart=calendar", range.start, range.end);
    let next_href = format!(
        "?start={}&end={}&chart=commute-casual",
        range.start, range.end
    );
    let active_calendar = matches!(chart, Chart::Calendar);
    let definition = chart.definition();
    let chart_title = definition.title;
    let chart_deck = definition.deck;
    let chart_body = if active_calendar {
        calendar_markup(&dataset, range, &summary)
    } else {
        line_chart_markup(&summary)
    };
    let prior_disabled = if active_calendar { "true" } else { "false" };
    let next_disabled = if active_calendar { "false" } else { "true" };
    let data_note = format!(
        "{} – {} · {} rows · latest {} · retrieved {}",
        dataset.min_date.format("%-d %b %Y"),
        dataset.max_date.format("%-d %b %Y"),
        dataset.records.len(),
        dataset.max_date.format("%-d %b %Y"),
        dataset.refreshed_at
    );
    let default_status = default_range_note(&dataset);
    let reset_range = default_range(&dataset);
    let reset_disabled = range == reset_range;
    let all_disabled = range.start == dataset.min_date && range.end == dataset.max_date;
    let available_dates = dataset
        .records
        .iter()
        .map(|record| record.date.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let mapping_note = format!(
        "Mapping: date={} · total={} · commute={} + {} · casual={} + {} + {}",
        dataset.field_mapping.date,
        dataset
            .field_mapping
            .total
            .as_deref()
            .unwrap_or("calculated from fare media"),
        dataset
            .field_mapping
            .smart_card
            .as_deref()
            .unwrap_or("missing"),
        dataset.field_mapping.ncmc.as_deref().unwrap_or("missing"),
        dataset.field_mapping.token.as_deref().unwrap_or("missing"),
        dataset.field_mapping.qr.as_deref().unwrap_or("missing"),
        dataset
            .field_mapping
            .group_ticket
            .as_deref()
            .unwrap_or("missing"),
    );
    let style = Unescaped::new_unchecked(STYLE);
    let payload_script = Unescaped::new_unchecked(format!(
        r#"<script type="application/json" id="chart-data">{payload_json}</script>"#
    ));
    let d3_script = Unescaped::new_unchecked(
        r#"<script src="https://cdn.jsdelivr.net/npm/d3@7.9.0/dist/d3.min.js"></script>"#,
    );
    let gsap_script = Unescaped::new_unchecked(
        r#"<script src="https://cdn.jsdelivr.net/npm/gsap@3.12.5/dist/gsap.min.js"></script>"#,
    );
    let script = Unescaped::new_unchecked(WRAPPER_OPEN);
    let script_body = Unescaped::new_unchecked(CLIENT_SCRIPT);
    let script_close = Unescaped::new_unchecked(WRAPPER_CLOSE);
    view! {
        cx =>
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta
                    name="description"
                    content="A transparent public view of Bengaluru Metro ridership."
                >
                <title>"NammaMetro in Charts"</title>
                (style)
            </head>
            <body>
                <main class="site-shell">
                    <header class="site-header">
                        <div>
                            <p class="eyebrow">"PUBLIC DATA STORY · BENGALURU"</p>
                            <h1>"NammaMetro in Charts"</h1>
                            <p class="subtitle">
                                "A public view of Bengaluru Metro ridership"
                            </p>
                        </div>
                        <a
                            class="source-link"
                            href=(SOURCE_PAGE_URL)
                            target="_blank"
                            rel="noreferrer"
                        >
                            "Source data on GitHub ↗"
                        </a>
                    </header>
                    <section class="prepare-band" aria-labelledby="prepare-title">
                        <div class="band-heading">
                            <p class="eyebrow">"01 · SCOPE"</p>
                            <h2 id="prepare-title">"Prepare the data"</h2>
                            <p>
                                "Choose a window. Every summary and chart below recalculates from these inclusive dates."
                            </p>
                        </div>
                        <div
                            class="range-controls"
                            data-available-dates=(available_dates)
                        >
                            <div class="range-inputs">
                                <label>
                                    <span>"From"</span>
                                    <input
                                        id="start-date"
                                        type="date"
                                        min=(dataset.min_date.to_string())
                                        max=(dataset.max_date.to_string())
                                        value=(range.start.to_string())
                                    >
                                </label>
                                <span class="range-dash">"—"</span>
                                <label>
                                    <span>"To"</span>
                                    <input
                                        id="end-date"
                                        type="date"
                                        min=(dataset.min_date.to_string())
                                        max=(dataset.max_date.to_string())
                                        value=(range.end.to_string())
                                    >
                                </label>
                            </div>
                            <div class="range-readable" id="range-readable">
                                (range_label.clone())
                            </div>
                            <div class="range-slider" aria-label="Date range slider">
                                <div class="slider-track"></div>
                                <input
                                    id="range-start"
                                    type="range"
                                    min="0"
                                    max=((dataset.records.len() - 1).to_string())
                                    value=(record_index(&dataset, range.start)
                                        .to_string())
                                    aria-label="Start date"
                                >
                                <input
                                    id="range-end"
                                    type="range"
                                    min="0"
                                    max=((dataset.records.len() - 1).to_string())
                                    value=(record_index(&dataset, range.end).to_string())
                                    aria-label="End date"
                                >
                            </div>
                            <div class="range-actions">
                                <a
                                    class="text-button"
                                    href=(default_range_link(&dataset, chart))
                                    aria-disabled=(reset_disabled.to_string())
                                    tabindex=(if reset_disabled { "-1" } else { "0" })
                                >
                                    "Reset to Jan–Jun 2026"
                                </a>
                                <a
                                    class="text-button"
                                    href=(all_data_link(&dataset, chart))
                                    aria-disabled=(all_disabled.to_string())
                                    tabindex=(if all_disabled { "-1" } else { "0" })
                                >
                                    "Use all available data"
                                </a>
                            </div>
                            <p class="range-status" id="range-status">
                                (default_status)
                            </p>
                        </div>
                    </section>
                    <section class="chart-stage" aria-labelledby="chart-title">
                        <div class="stage-topline">
                            <p class="eyebrow">
                                "02 · CHART "
                                ((chart.index() + 1).to_string())
                            </p>
                            <p class="selection-label">(range_label.clone())</p>
                        </div>
                        <h2 id="chart-title">(chart_title)</h2>
                        <p class="chart-deck">(chart_deck)</p>
                        <div class="chart-shell">
                            (Unescaped::new_unchecked(chart_body))
                        </div>
                        <div class="chart-method">
                            (if active_calendar {
                                "Percentile bands use interpolated quantiles of valid totals in this selection. Missing dates stay visible as crossed cells."
                            } else {
                                "Lines show weekday pattern, not a chronological time series. Averages omit dates missing any component of the selected fare-media group."
                            })
                        </div>
                        <div class="navigation-row">
                            <a
                                class="arrow-button"
                                aria-label="Previous chart: Daily ridership calendar"
                                href=(previous_href.clone())
                                aria-disabled=(prior_disabled)
                                tabindex=(if active_calendar { "-1" } else { "0" })
                            >
                                <span class="arrow-glyph">"←"</span>
                                <span>"Previous"</span>
                            </a>
                            <nav class="chart-nav" aria-label="Chart navigation">
                                <a
                                    class=(if active_calendar {
                                        "chart-tab active"
                                    } else {
                                        "chart-tab"
                                    })
                                    href=(previous_href)
                                >
                                    "01 · "
                                    (CHARTS[0].short_name)
                                </a>
                                <span class="chart-position">
                                    ((chart.index() + 1).to_string())
                                    " / "
                                    (CHARTS.len().to_string())
                                </span>
                                <a
                                    class=(if !active_calendar {
                                        "chart-tab active"
                                    } else {
                                        "chart-tab"
                                    })
                                    href=(next_href.clone())
                                >
                                    "02 · "
                                    (CHARTS[1].short_name)
                                </a>
                            </nav>
                            <a
                                class="arrow-button"
                                aria-label="Next chart: Commute vs casual"
                                href=(next_href)
                                aria-disabled=(next_disabled)
                                tabindex=(if active_calendar { "0" } else { "-1" })
                            >
                                <span>"Next"</span>
                                <span class="arrow-glyph">"→"</span>
                            </a>
                        </div>
                    </section>
                    <footer class="site-footer">
                        <div>
                            <p class="eyebrow">"DATA NOTE"</p>
                            <p>(data_note)</p>
                            <p>
                                "Selected coverage: "
                                (summary.observation_days)
                                " observed days of "
                                (summary.calendar_days)
                                ". "
                                (summary.missing_days)
                                " calendar days are missing."
                            </p>
                        </div>
                        <details>
                            <summary>"Methodology & field mapping"</summary>
                            <p>
                                "Total ridership uses the supplied total when valid; otherwise it is the sum of valid fare-media fields. Commuter is Smart Card + NCMC. Casual is Token + QR + Group tickets. Blank source values remain missing, not zero. Sample standard deviation uses n − 1."
                            </p>
                            <p>(mapping_note)</p>
                            <p>
                                "Invalid dates: "
                                (dataset.invalid_date_count)
                                " · Duplicate dates resolved: "
                                (dataset.duplicate_count)
                                "."
                            </p>
                        </details>
                    </footer>
                </main>
                <div
                    id="chart-tooltip"
                    class="chart-tooltip"
                    role="tooltip"
                    hidden="hidden"
                ></div>
                (payload_script)
                (d3_script)
                (gsap_script)
                (script)
                (script_body)
                (script_close)
            </body>
        </html>
    }
}

async fn failure_view(cx: &Cx) -> Result {
    view! {
        cx =>
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"NammaMetro in Charts"</title>
                (Unescaped::new_unchecked(STYLE))
            </head>
            <body>
                <main class="error-page">
                    <p class="eyebrow">"NAMMAMETRO IN CHARTS"</p>
                    <h1>"The source data could not be loaded."</h1>
                    <p>
                        "The canonical GitHub CSV was unavailable when this page started. Use retry to attempt a fresh fetch now; a hosted deployment can also serve a last-known-good cache here."
                    </p>
                    <a class="button" href="/?retry=1">"Retry now"</a>
                </main>
            </body>
        </html>
    }
}

fn parse_query(query: &str) -> QueryState {
    let mut state = QueryState::default();
    for pair in query.split('&') {
        let mut pieces = pair.splitn(2, '=');
        let Some(key) = pieces.next() else { continue };
        let value = pieces.next().unwrap_or_default();
        match key {
            "start" => state.start = Some(value.to_string()),
            "end" => state.end = Some(value.to_string()),
            "chart" => state.chart = Some(value.to_string()),
            "retry" => state.retry = matches!(value, "1" | "true" | "yes"),
            _ => {}
        }
    }
    state
}

fn default_range(dataset: &Dataset) -> DateRange {
    choose_range(dataset, Some(DEFAULT_START), Some(DEFAULT_END))
}

fn default_range_link(dataset: &Dataset, chart: Chart) -> String {
    let range = default_range(dataset);
    format!(
        "?start={}&end={}&chart={}",
        range.start,
        range.end,
        chart.slug()
    )
}

fn all_data_link(dataset: &Dataset, chart: Chart) -> String {
    format!(
        "?start={}&end={}&chart={}",
        dataset.min_date,
        dataset.max_date,
        chart.slug()
    )
}

fn choose_range(dataset: &Dataset, start: Option<&str>, end: Option<&str>) -> DateRange {
    let requested_start = start.and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
    let requested_end = end.and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
    let mut start = requested_start.unwrap_or_else(|| default_date(dataset, DEFAULT_START));
    let mut end = requested_end.unwrap_or_else(|| default_date(dataset, DEFAULT_END));
    start = nearest_available(dataset, start);
    end = nearest_available(dataset, end);
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    DateRange { start, end }
}

fn default_date(dataset: &Dataset, value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|date| nearest_available(dataset, date))
        .unwrap_or(dataset.min_date)
}

fn nearest_available(dataset: &Dataset, target: NaiveDate) -> NaiveDate {
    dataset
        .records
        .iter()
        .min_by_key(|record| (record.date - target).num_days().unsigned_abs())
        .map(|record| record.date)
        .unwrap_or(dataset.min_date)
}

fn record_index(dataset: &Dataset, date: NaiveDate) -> usize {
    dataset
        .records
        .iter()
        .position(|record| record.date == date)
        .unwrap_or(0)
}

fn format_range(range: DateRange) -> String {
    format!(
        "{} – {}",
        range.start.format("%-d %b %Y"),
        range.end.format("%-d %b %Y")
    )
}

fn default_range_note(dataset: &Dataset) -> String {
    let desired = default_range(dataset);
    if desired.start.to_string() == DEFAULT_START && desired.end.to_string() == DEFAULT_END {
        "Default window available: 1 Jan 2026 – 30 Jun 2026.".to_string()
    } else {
        format!(
            "Default window clamped to available records: {}.",
            format_range(desired)
        )
    }
}

async fn fetch_dataset(client: &Client) -> std::result::Result<Dataset, String> {
    let response = client
        .get(SOURCE_URL)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("source returned HTTP {}", response.status()));
    }
    let body = response.text().await.map_err(|error| error.to_string())?;
    if body.trim().is_empty() {
        return Err("source returned an empty response".to_string());
    }
    let refreshed_at = chrono::Utc::now().to_rfc3339();
    let dataset = parse_dataset(&body, refreshed_at.clone())?;
    if let Err(error) = write_dataset_cache(&body, &refreshed_at) {
        eprintln!("Could not update ridership cache: {error}");
    }
    Ok(dataset)
}

fn cache_path() -> PathBuf {
    std::env::var("METRO_CACHE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_CACHE_PATH))
}

fn write_dataset_cache(csv_text: &str, refreshed_at: &str) -> std::result::Result<(), String> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, csv_text).map_err(|error| error.to_string())?;
    fs::write(path.with_extension("refreshed-at"), refreshed_at).map_err(|error| error.to_string())
}

fn load_cached_dataset() -> std::result::Result<Dataset, String> {
    let path = cache_path();
    let csv_text = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let refreshed_at = fs::read_to_string(path.with_extension("refreshed-at"))
        .map_err(|error| error.to_string())?;
    parse_dataset(&csv_text, refreshed_at.trim().to_string())
}

fn parse_dataset(csv_text: &str, refreshed_at: String) -> std::result::Result<Dataset, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(csv_text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|error| format!("CSV header error: {error}"))?
        .clone();
    let mapping = FieldMapping::from_headers(&headers)?;
    let mut by_date: HashMap<NaiveDate, RidershipRecord> = HashMap::new();
    let mut invalid_date_count = 0;
    let mut duplicate_count = 0;
    for row in reader.records() {
        let row = row.map_err(|error| format!("CSV row error: {error}"))?;
        let raw_date = field(&headers, &row, &mapping.date).unwrap_or_default();
        let Some(date) = parse_date(raw_date) else {
            invalid_date_count += 1;
            continue;
        };
        let record = normalise_record(&headers, &row, &mapping, date);
        if by_date.insert(date, record).is_some() {
            duplicate_count += 1;
            eprintln!("duplicate date {date}: keeping the latest valid row");
        }
    }
    let mut records: Vec<_> = by_date.into_values().collect();
    records.sort_by_key(|record| record.date);
    let Some(first) = records.first() else {
        return Err("CSV contained no usable dated records".to_string());
    };
    let min_date = first.date;
    let max_date = records.last().expect("first exists").date;
    Ok(Dataset {
        records,
        min_date,
        max_date,
        invalid_date_count,
        duplicate_count,
        field_mapping: mapping,
        refreshed_at,
    })
}

impl FieldMapping {
    fn from_headers(headers: &StringRecord) -> std::result::Result<Self, String> {
        let names: Vec<(String, String)> = headers
            .iter()
            .map(|header| (normalise_header(header), header.to_string()))
            .collect();
        let find = |aliases: &[&str]| {
            names
                .iter()
                .find(|(normalised, _)| aliases.iter().any(|alias| normalised == *alias))
                .map(|(_, original)| original.clone())
        };
        let date = find(&["recorddate", "date", "journeydate"])
            .ok_or_else(|| "CSV has no recognisable date column".to_string())?;
        Ok(Self {
            date,
            total: find(&[
                "totalridership",
                "totaldailyridership",
                "totaldaily ridership",
                "totaljourneys",
                "total",
            ]),
            smart_card: find(&["totalsmartcards", "smartcard", "smartcards"]),
            ncmc: find(&["totalncmc", "ncmc"]),
            token: find(&["totaltokens", "token", "tokens"]),
            qr: find(&["totalqr", "qr", "qrtickets"]),
            group_ticket: find(&["groupticket", "grouptickets"]),
        })
    }
}

fn normalise_header(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

fn field<'a>(headers: &'a StringRecord, row: &'a StringRecord, header: &str) -> Option<&'a str> {
    headers
        .iter()
        .position(|candidate| candidate == header)
        .and_then(|index| row.get(index))
        .map(str::trim)
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    ["%d-%m-%Y", "%Y-%m-%d", "%d/%m/%Y", "%Y/%m/%d"]
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(value.trim(), format).ok())
}

fn clean_number(value: Option<&str>) -> Option<f64> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    value
        .replace(',', "")
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

fn normalise_record(
    headers: &StringRecord,
    row: &StringRecord,
    mapping: &FieldMapping,
    date: NaiveDate,
) -> RidershipRecord {
    let smart_card = clean_number(
        mapping
            .smart_card
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let ncmc = clean_number(
        mapping
            .ncmc
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let token = clean_number(
        mapping
            .token
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let qr = clean_number(
        mapping
            .qr
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let group_ticket = clean_number(
        mapping
            .group_ticket
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let total_ridership = clean_number(
        mapping
            .total
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    )
    .or_else(|| sum_complete([smart_card, ncmc, token, qr, group_ticket]));
    let commuter_ridership = sum_complete([smart_card, ncmc]);
    let casual_ridership = sum_complete([token, qr, group_ticket]);
    let raw_fields = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            (
                header.to_string(),
                row.get(index).unwrap_or_default().trim().to_string(),
            )
        })
        .collect();
    RidershipRecord {
        date,
        raw_fields,
        total_ridership,
        smart_card,
        ncmc,
        token,
        qr,
        group_ticket,
        commuter_ridership,
        casual_ridership,
    }
}

fn sum_complete<const N: usize>(values: [Option<f64>; N]) -> Option<f64> {
    values
        .iter()
        .copied()
        .collect::<Option<Vec<_>>>()
        .map(|values| values.iter().sum())
}

fn summarise(dataset: &Dataset, range: DateRange) -> RangeSummary {
    let selected: Vec<_> = dataset
        .records
        .iter()
        .filter(|record| record.date >= range.start && record.date <= range.end)
        .collect();
    let calendar_days = (range.end - range.start).num_days() as usize + 1;
    let totals: Vec<f64> = selected
        .iter()
        .filter_map(|record| record.total_ridership)
        .collect();
    let mut percentiles = Vec::new();
    for percentile in [2, 5, 10, 25, 50, 75, 90, 95, 98] {
        if let Some(value) = quantile(&totals, percentile as f64 / 100.0) {
            percentiles.push((percentile, value));
        }
    }
    RangeSummary {
        calendar_days,
        observation_days: selected.len(),
        missing_days: calendar_days.saturating_sub(selected.len()),
        percentiles,
        valid_total_count: totals.len(),
        total_min: totals.iter().copied().reduce(f64::min),
        total_max: totals.iter().copied().reduce(f64::max),
        commute: weekday_metrics(&selected, |record| record.commuter_ridership),
        casual: weekday_metrics(&selected, |record| record.casual_ridership),
    }
}

fn weekday_metrics(
    records: &[&RidershipRecord],
    value: impl Fn(&RidershipRecord) -> Option<f64>,
) -> Vec<WeekdayMetric> {
    [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ]
    .iter()
    .map(|&weekday| {
        let values: Vec<f64> = records
            .iter()
            .filter(|record| record.date.weekday() == weekday)
            .filter_map(|record| value(record))
            .collect();
        let n = values.len();
        let mean = (!values.is_empty()).then(|| values.iter().sum::<f64>() / n as f64);
        let standard_deviation = (n > 1).then(|| {
            let average = mean.expect("n > 1 has a mean");
            (values
                .iter()
                .map(|value| (value - average).powi(2))
                .sum::<f64>()
                / (n - 1) as f64)
                .sqrt()
        });
        WeekdayMetric {
            weekday,
            n,
            mean,
            standard_deviation,
            min: values.iter().copied().reduce(f64::min),
            max: values.iter().copied().reduce(f64::max),
        }
    })
    .collect()
}

fn quantile(values: &[f64], probability: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    Some(sorted[lower] + (sorted[upper] - sorted[lower]) * (position - lower as f64))
}

fn chart_payload_for(dataset: &Dataset, range: DateRange) -> ChartPayload {
    let summary = summarise(dataset, range);
    ChartPayload {
        range: PayloadRange {
            start: range.start.to_string(),
            end: range.end.to_string(),
            label: format_range(range),
            start_index: record_index(dataset, range.start),
            end_index: record_index(dataset, range.end),
        },
        dataset: PayloadDataset {
            min_date: dataset.min_date.to_string(),
            max_date: dataset.max_date.to_string(),
            row_count: dataset.records.len(),
            refreshed_at: dataset.refreshed_at.clone(),
            available_dates: dataset
                .records
                .iter()
                .map(|record| record.date.to_string())
                .collect(),
        },
        summary: PayloadSummary {
            calendar_days: summary.calendar_days,
            observation_days: summary.observation_days,
            missing_days: summary.missing_days,
            valid_total_count: summary.valid_total_count,
            total_min: summary.total_min,
            total_max: summary.total_max,
            total_min_label: summary.total_min.map(format_number_full),
            total_max_label: summary.total_max.map(format_number_full),
            insufficient_percentiles: summary.valid_total_count < 10,
        },
        charts: PayloadCharts {
            calendar: CalendarPayload {
                cells: calendar_payload_cells(dataset, range, &summary),
                legend: legend_payload(),
            },
            commute_casual: CommuteCasualPayload {
                weekdays: weekday_payloads(),
                series: vec![
                    series_payload("commute", "Commute · Smart Card + NCMC", &summary.commute),
                    series_payload("casual", "Casual · Token + QR + Group", &summary.casual),
                ],
                insight: insight_line(&summary),
            },
        },
    }
}

fn calendar_payload_cells(
    dataset: &Dataset,
    range: DateRange,
    summary: &RangeSummary,
) -> Vec<CalendarCellPayload> {
    let first_monday =
        range.start - chrono::Duration::days(range.start.weekday().num_days_from_monday() as i64);
    let map: HashMap<NaiveDate, &RidershipRecord> = dataset
        .records
        .iter()
        .filter(|record| record.date >= range.start && record.date <= range.end)
        .map(|record| (record.date, record))
        .collect();
    let mut cells = Vec::new();
    let mut date = range.start;
    while date <= range.end {
        let record = map.get(&date).copied();
        let value = record.and_then(|record| record.total_ridership);
        let band = value.and_then(|value| band_label(value, summary));
        let breakdown = record.map(fare_media_breakdown).unwrap_or_default();
        let label = calendar_cell_label(date, value, band, &breakdown);
        let week = ((date - first_monday).num_days() / 7) as usize;
        let previous = date - chrono::Duration::days(1);
        let month_label = (date == range.start || date.month() != previous.month())
            .then(|| date.format("%b %Y").to_string());
        cells.push(CalendarCellPayload {
            date: date.to_string(),
            label,
            weekday: date.weekday().num_days_from_monday() as usize,
            week,
            month_label,
            total: value,
            total_label: value.map(format_number_full),
            band_label: band.map(str::to_string),
            band_index: band.and_then(band_index),
            missing: value.is_none(),
            breakdown,
        });
        date += chrono::Duration::days(1);
    }
    cells
}

fn calendar_cell_label(
    date: NaiveDate,
    value: Option<f64>,
    band: Option<&'static str>,
    breakdown: &str,
) -> String {
    match (value, band) {
        (Some(value), Some(band)) => format!(
            "{}: {} total journeys, {}{}",
            date.format("%A, %-d %B %Y"),
            format_number_full(value),
            band,
            breakdown
        ),
        (Some(value), None) => format!(
            "{}: {} total journeys; insufficient data for percentile bands{}",
            date.format("%A, %-d %B %Y"),
            format_number_full(value),
            breakdown
        ),
        _ => format!("{}: Missing data", date.format("%A, %-d %B %Y")),
    }
}

fn legend_payload() -> Vec<LegendItemPayload> {
    [
        "< p2",
        "p2 – p5",
        "p5 – p10",
        "p10 – p25",
        "p25 – p50",
        "p50 – p75",
        "p75 – p90",
        "p90 – p95",
        "p95 – p98",
        "> p98",
    ]
    .iter()
    .enumerate()
    .map(|(band_index, label)| LegendItemPayload { label, band_index })
    .collect()
}

fn weekday_payloads() -> Vec<WeekdayPayload> {
    [
        ("Mon", Weekday::Mon),
        ("Tue", Weekday::Tue),
        ("Wed", Weekday::Wed),
        ("Thu", Weekday::Thu),
        ("Fri", Weekday::Fri),
        ("Sat", Weekday::Sat),
        ("Sun", Weekday::Sun),
    ]
    .iter()
    .enumerate()
    .map(|(index, (short, weekday))| WeekdayPayload {
        index,
        short,
        name: weekday_name(*weekday),
    })
    .collect()
}

fn series_payload(
    key: &'static str,
    label: &'static str,
    metrics: &[WeekdayMetric],
) -> SeriesPayload {
    let shorts = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    SeriesPayload {
        key,
        label,
        points: metrics
            .iter()
            .enumerate()
            .map(|(index, metric)| weekday_point_payload(metric, shorts[index], index))
            .collect(),
    }
}

fn weekday_point_payload(
    metric: &WeekdayMetric,
    weekday_short: &'static str,
    index: usize,
) -> WeekdayPointPayload {
    let tooltip = metric.mean.map_or_else(
        || format!("{weekday_short}: no complete observations"),
        |mean| {
            format!(
                "{}: mean {}, SD {}, n {}, range {}–{}",
                weekday_short,
                format_number_full(mean),
                metric
                    .standard_deviation
                    .map(format_number_full)
                    .unwrap_or_else(|| "—".to_string()),
                metric.n,
                metric
                    .min
                    .map(format_number_full)
                    .unwrap_or_else(|| "—".to_string()),
                metric
                    .max
                    .map(format_number_full)
                    .unwrap_or_else(|| "—".to_string())
            )
        },
    );
    WeekdayPointPayload {
        weekday: weekday_name(metric.weekday),
        weekday_short,
        index,
        n: metric.n,
        mean: metric.mean,
        mean_label: metric.mean.map(format_number_full),
        standard_deviation: metric.standard_deviation,
        standard_deviation_label: metric.standard_deviation.map(format_number_full),
        min: metric.min,
        min_label: metric.min.map(format_number_full),
        max: metric.max,
        max_label: metric.max.map(format_number_full),
        tooltip,
    }
}

fn band_index(label: &str) -> Option<usize> {
    match label {
        "< p2" => Some(0),
        "p2 – p5" => Some(1),
        "p5 – p10" => Some(2),
        "p10 – p25" => Some(3),
        "p25 – p50" => Some(4),
        "p50 – p75" => Some(5),
        "p75 – p90" => Some(6),
        "p90 – p95" => Some(7),
        "p95 – p98" => Some(8),
        "> p98" => Some(9),
        _ => None,
    }
}

fn calendar_markup(dataset: &Dataset, range: DateRange, summary: &RangeSummary) -> String {
    let mut svg = String::from(
        r#"<div class="calendar-wrap"><div class="calendar-grid" role="img" aria-label="Daily ridership calendar heatmap"><div class="weekday-labels"><span>Mon</span><span>Tue</span><span>Wed</span><span>Thu</span><span>Fri</span><span>Sat</span><span>Sun</span></div><div class="calendar-canvas">"#,
    );
    let first_monday =
        range.start - chrono::Duration::days(range.start.weekday().num_days_from_monday() as i64);
    let last_sunday =
        range.end + chrono::Duration::days(6 - range.end.weekday().num_days_from_monday() as i64);
    let mut cursor = first_monday;
    let map: HashMap<NaiveDate, &RidershipRecord> = dataset
        .records
        .iter()
        .filter(|record| record.date >= range.start && record.date <= range.end)
        .map(|record| (record.date, record))
        .collect();
    let mut week_index = 0;
    let mut last_labelled_month: Option<(i32, u32)> = None;
    while cursor <= last_sunday {
        svg.push_str(&format!(
            r#"<div class="calendar-week" style="--week:{week_index}" aria-label="Week beginning {}">"#,
            cursor.format("%-d %b")
        ));
        for day_offset in 0..7 {
            let date = cursor + chrono::Duration::days(day_offset as i64);
            if date < range.start || date > range.end {
                svg.push_str(
                    r#"<span class="calendar-cell structural" aria-hidden="true"></span>"#,
                );
                continue;
            }
            let record = map.get(&date).copied();
            let value = record.and_then(|record| record.total_ridership);
            let band = value.and_then(|value| band_label(value, summary));
            let breakdown = record.map(fare_media_breakdown).unwrap_or_default();
            let label = match (value, band) {
                (Some(value), Some(band)) => format!(
                    "{}: {} total journeys, {}{}",
                    date.format("%A, %-d %B %Y"),
                    format_number_full(value),
                    band,
                    breakdown
                ),
                (Some(value), None) => format!(
                    "{}: {} total journeys; insufficient data for percentile bands{}",
                    date.format("%A, %-d %B %Y"),
                    format_number_full(value),
                    breakdown
                ),
                _ => format!("{}: Missing data", date.format("%A, %-d %B %Y")),
            };
            let safe = escape_attr(&label);
            let classes = match (value, band) {
                (None, _) => "calendar-cell missing",
                (_, Some(band)) => match band {
                    "< p2" => "calendar-cell band-0",
                    "p2 – p5" => "calendar-cell band-1",
                    "p5 – p10" => "calendar-cell band-2",
                    "p10 – p25" => "calendar-cell band-3",
                    "p25 – p50" => "calendar-cell band-4",
                    "p50 – p75" => "calendar-cell band-5",
                    "p75 – p90" => "calendar-cell band-6",
                    "p90 – p95" => "calendar-cell band-7",
                    "p95 – p98" => "calendar-cell band-8",
                    "> p98" => "calendar-cell band-9",
                    _ => "calendar-cell neutral",
                },
                (_, None) => "calendar-cell neutral",
            };
            let dow = date.weekday().num_days_from_monday();
            svg.push_str(&format!(
                r#"<button class="{classes}" title="{safe}" aria-label="{safe}" data-tooltip="{safe}" data-date="{date}" style="--day:{dow}"><span class="sr-only">{safe}</span></button>"#
            ));
        }
        svg.push_str("</div>");
        let first_visible = (0..7)
            .map(|offset| cursor + chrono::Duration::days(offset))
            .find(|date| *date >= range.start && *date <= range.end);
        let month_start = (0..7)
            .map(|offset| cursor + chrono::Duration::days(offset))
            .find(|date| {
                *date >= range.start
                    && *date <= range.end
                    && Some((date.year(), date.month())) != last_labelled_month
            })
            .or(first_visible.filter(|_| last_labelled_month.is_none()));
        if let Some(label_date) = month_start {
            svg.push_str(&format!(
                r#"<span class="month-label" style="--week:{week_index}">{}</span>"#,
                label_date.format("%b %Y")
            ));
            last_labelled_month = Some((label_date.year(), label_date.month()));
        }
        cursor += chrono::Duration::days(7);
        week_index += 1;
    }
    svg.push_str("</div></div>");
    svg.push_str(&legend_markup(summary));
    svg.push_str(&calendar_table(dataset, range, summary));
    svg
}

fn legend_markup(summary: &RangeSummary) -> String {
    let labels = [
        "< p2",
        "p2 – p5",
        "p5 – p10",
        "p10 – p25",
        "p25 – p50",
        "p50 – p75",
        "p75 – p90",
        "p90 – p95",
        "p95 – p98",
        "> p98",
    ];
    let mut html = String::from(r#"<div class="legend" aria-label="Percentile bands">"#);
    if summary.valid_total_count < 10 {
        html.push_str(r#"<p class="notice">Insufficient data for percentile bands; colour distinctions are intentionally muted.</p>"#);
    }
    for (index, label) in labels.iter().enumerate() {
        html.push_str(&format!(
            r#"<span><i class="swatch band-{index}"></i>{label}</span>"#
        ));
    }
    html.push_str(
        r#"<span><i class="swatch missing-swatch"></i>Crossed cells: missing data</span>"#,
    );
    if let (Some(min), Some(max)) = (summary.total_min, summary.total_max) {
        html.push_str(&format!(
            r#"<span class="legend-range">Observed: {} – {}</span>"#,
            compact_number(min),
            compact_number(max)
        ));
    }
    html.push_str("</div>");
    html
}

fn band_label(value: f64, summary: &RangeSummary) -> Option<&'static str> {
    if summary.valid_total_count < 10 {
        return None;
    }
    let bounds: Vec<f64> = [2, 5, 10, 25, 50, 75, 90, 95, 98]
        .iter()
        .filter_map(|percentile| {
            summary
                .percentiles
                .iter()
                .find(|(key, _)| key == percentile)
                .map(|(_, value)| *value)
        })
        .collect();
    if bounds.len() != 9 {
        return None;
    }
    Some(match bounds.iter().position(|bound| value < *bound) {
        Some(0) => "< p2",
        Some(1) => "p2 – p5",
        Some(2) => "p5 – p10",
        Some(3) => "p10 – p25",
        Some(4) => "p25 – p50",
        Some(5) => "p50 – p75",
        Some(6) => "p75 – p90",
        Some(7) => "p90 – p95",
        Some(8) => "p95 – p98",
        _ => "> p98",
    })
}

fn calendar_table(dataset: &Dataset, range: DateRange, summary: &RangeSummary) -> String {
    let mut table = String::from(
        r#"<details class="accessible-data"><summary>View calendar data table</summary><div class="table-scroll"><table><caption>Calendar values in the selected range</caption><thead><tr><th>Date</th><th>Total ridership</th><th>Band</th></tr></thead><tbody>"#,
    );
    let mut date = range.start;
    while date <= range.end {
        let record = dataset.records.iter().find(|record| record.date == date);
        let value = record.and_then(|record| record.total_ridership);
        let display = value
            .map(format_number_full)
            .unwrap_or_else(|| "Missing data".to_string());
        let band = value
            .and_then(|value| band_label(value, summary))
            .unwrap_or("—");
        table.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            date.format("%A, %-d %B %Y"),
            display,
            band
        ));
        date += chrono::Duration::days(1);
    }
    table.push_str("</tbody></table></div></details>");
    table
}

fn line_chart_markup(summary: &RangeSummary) -> String {
    const WIDTH: f64 = 900.0;
    const HEIGHT: f64 = 410.0;
    const LEFT: f64 = 66.0;
    const RIGHT: f64 = 28.0;
    const TOP: f64 = 24.0;
    const BOTTOM: f64 = 66.0;
    let max_value = summary
        .commute
        .iter()
        .chain(summary.casual.iter())
        .filter_map(|metric| {
            metric
                .mean
                .map(|mean| mean + metric.standard_deviation.unwrap_or(0.0))
        })
        .reduce(f64::max)
        .unwrap_or(1.0)
        .max(1.0);
    let plot_width = WIDTH - LEFT - RIGHT;
    let plot_height = HEIGHT - TOP - BOTTOM;
    let x = |index: usize| LEFT + index as f64 * plot_width / 6.0;
    let y = |value: f64| TOP + plot_height - value / max_value * plot_height;
    let mut svg = format!(
        r#"<div class="line-chart-wrap"><svg class="line-chart" viewBox="0 0 {WIDTH} {HEIGHT}" role="img" aria-label="Average daily ridership by weekday for commute and casual fare media"><g class="grid">"#
    );
    for tick in 0..=4 {
        let value = max_value * tick as f64 / 4.0;
        let yy = y(value);
        svg.push_str(&format!(
            r#"<line x1="{LEFT}" x2="{}" y1="{yy:.1}" y2="{yy:.1}"></line><text x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            WIDTH - RIGHT,
            LEFT - 10.0,
            yy + 4.0,
            compact_number(value)
        ));
    }
    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    for (index, weekday) in weekdays.iter().enumerate() {
        svg.push_str(&format!(
            r#"<text class="x-label" x="{:.1}" y="{}" text-anchor="middle">{weekday}</text>"#,
            x(index),
            HEIGHT - 26.0
        ));
    }
    for (metrics, class, label) in [
        (&summary.commute, "commute", "Commute · Smart Card + NCMC"),
        (&summary.casual, "casual", "Casual · Token + QR + Group"),
    ] {
        let mut path = String::new();
        let mut segment_open = false;
        for (index, metric) in metrics.iter().enumerate() {
            if let Some(mean) = metric.mean {
                let command = if segment_open { 'L' } else { 'M' };
                write!(path, "{command} {:.1} {:.1} ", x(index), y(mean)).unwrap();
                segment_open = true;
            } else {
                segment_open = false;
            }
        }
        svg.push_str(&format!(
            r#"<path class="series {class}" d="{path}" fill="none"></path>"#
        ));
        for (index, metric) in metrics.iter().enumerate() {
            let Some(mean) = metric.mean else {
                continue;
            };
            let xx = x(index);
            let yy = y(mean);
            if let Some(sd) = metric.standard_deviation {
                let top = y((mean + sd).min(max_value));
                let bottom = y((mean - sd).max(0.0));
                svg.push_str(&format!(
                    r#"<line class="whisker {class}" x1="{xx:.1}" x2="{xx:.1}" y1="{top:.1}" y2="{bottom:.1}"></line>"#
                ));
            }
            let label_text = format!(
                "{}: mean {}, SD {}, n {}, range {}–{}",
                weekdays[index],
                format_number_full(mean),
                metric
                    .standard_deviation
                    .map(format_number_full)
                    .unwrap_or_else(|| "—".to_string()),
                metric.n,
                metric
                    .min
                    .map(format_number_full)
                    .unwrap_or_else(|| "—".to_string()),
                metric
                    .max
                    .map(format_number_full)
                    .unwrap_or_else(|| "—".to_string())
            );
            let safe = escape_attr(&label_text);
            let compact = compact_number(mean);
            svg.push_str(&format!(
                r#"<circle class="point {class}" cx="{xx:.1}" cy="{yy:.1}" r="6" tabindex="0" role="img" aria-label="{safe}" data-tooltip="{safe}"></circle><text class="point-label {class}" x="{xx:.1}" y="{:.1}" text-anchor="middle">{compact}</text>"#,
                yy - 14.0
            ));
        }
        svg.push_str(&format!(
            r#"<text class="series-label {class}" x="{}" y="{}">{label}</text>"#,
            WIDTH - 210.0,
            if class == "commute" { 25.0 } else { 48.0 }
        ));
    }
    svg.push_str("</g></svg>");
    let insight = insight_line(summary);
    if !insight.is_empty() {
        svg.push_str(&format!(r#"<p class="insight">{insight}</p>"#));
    }
    svg.push_str(&line_table(summary));
    svg.push_str("</div>");
    svg
}

fn insight_line(summary: &RangeSummary) -> String {
    fn peaks(metrics: &[WeekdayMetric]) -> Option<(f64, Vec<Weekday>)> {
        let values: Vec<(Weekday, f64)> = metrics
            .iter()
            .filter_map(|metric| metric.mean.map(|mean| (metric.weekday, mean)))
            .collect();
        let max = values.iter().map(|(_, mean)| *mean).reduce(f64::max)?;
        let days: Vec<Weekday> = values
            .iter()
            .filter(|(_, mean)| (mean - max).abs() < 1e-6)
            .map(|(day, _)| *day)
            .collect();
        Some((max, days))
    }
    fn describe(days: &[Weekday]) -> String {
        match days {
            [] => String::new(),
            [only] => weekday_name(*only).to_string(),
            [first, second] => format!("{} and {}", weekday_name(*first), weekday_name(*second)),
            _ => {
                let names: Vec<&str> = days.iter().map(|day| weekday_name(*day)).collect();
                let (last, rest) = names.split_last().expect("non-empty");
                format!("{} and {}", rest.join(", "), last)
            }
        }
    }
    let (Some((commute_max, commute_days)), Some((casual_max, casual_days))) =
        (peaks(&summary.commute), peaks(&summary.casual))
    else {
        return String::new();
    };
    let commute_clause = if commute_days.len() > 1 {
        format!(
            "Commute traffic ties across {} ({} average)",
            describe(&commute_days),
            compact_number(commute_max)
        )
    } else {
        format!(
            "Commute traffic peaks on {} ({} average)",
            describe(&commute_days),
            compact_number(commute_max)
        )
    };
    let casual_clause = if casual_days.len() > 1 {
        format!(
            "casual traffic ties across {} ({} average)",
            describe(&casual_days),
            compact_number(casual_max)
        )
    } else {
        format!(
            "casual traffic is highest on {} ({} average)",
            describe(&casual_days),
            compact_number(casual_max)
        )
    };
    format!("{commute_clause}, while {casual_clause}.")
}

fn line_table(summary: &RangeSummary) -> String {
    let mut table = String::from(
        r#"<details class="accessible-data"><summary>View weekday data table</summary><div class="table-scroll"><table><caption>Weekday averages and variation</caption><thead><tr><th>Day</th><th>Commute mean</th><th>Commute SD</th><th>Commute n</th><th>Casual mean</th><th>Casual SD</th><th>Casual n</th></tr></thead><tbody>"#,
    );
    for index in 0..7 {
        let commute = summary.commute[index];
        let casual = summary.casual[index];
        table.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            weekday_name(commute.weekday),
            commute.mean.map(format_number_full).unwrap_or_else(|| "—".to_string()),
            commute.standard_deviation.map(format_number_full).unwrap_or_else(|| "—".to_string()),
            commute.n,
            casual.mean.map(format_number_full).unwrap_or_else(|| "—".to_string()),
            casual.standard_deviation.map(format_number_full).unwrap_or_else(|| "—".to_string()),
            casual.n
        ));
    }
    table.push_str("</tbody></table></div></details>");
    table
}

fn fare_media_breakdown(record: &RidershipRecord) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut push = |label: &str, value: Option<f64>| {
        if let Some(value) = value {
            parts.push(format!("{label} {}", format_number_full(value)));
        }
    };
    push("Smart Card", record.smart_card);
    push("NCMC", record.ncmc);
    push("Token", record.token);
    push("QR", record.qr);
    push("Group", record.group_ticket);
    if parts.is_empty() {
        String::new()
    } else {
        format!(" — {}", parts.join(", "))
    }
}

fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

fn format_number_full(value: f64) -> String {
    let rounded = value.round() as i64;
    let negative = rounded < 0;
    let digits = rounded.unsigned_abs().to_string();
    let mut grouped = String::new();
    let len = digits.len();
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (len - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(character);
    }
    if negative {
        format!("-{grouped}")
    } else {
        grouped
    }
}

fn compact_number(value: f64) -> String {
    if value.abs() >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("{:.0}k", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

const WRAPPER_OPEN: &str = "<script>";
const WRAPPER_CLOSE: &str = "</script>";
const CLIENT_SCRIPT: &str = r#"(() => {
  const D3 = window.d3;
  const GSAP = window.gsap;
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const canAnimate = !reducedMotion;
  const chartRegistry = {
    calendar: {
      index: 0,
      title: 'Daily ridership calendar',
      deck: 'Each square is one calendar day. Deeper mauve means higher total ridership within the selected period.'
    },
    'commute-casual': {
      index: 1,
      title: 'Average daily ridership: Commute vs casual',
      deck: 'Average journeys by day of week, using the dates currently selected above.'
    }
  };

  const startInput = document.querySelector('#start-date');
  const endInput = document.querySelector('#end-date');
  const startSlider = document.querySelector('#range-start');
  const endSlider = document.querySelector('#range-end');
  const readable = document.querySelector('#range-readable');
  const status = document.querySelector('#range-status');
  const tooltip = document.querySelector('#chart-tooltip');
  const chartShell = document.querySelector('.chart-shell');
  const chartTitle = document.querySelector('#chart-title');
  const chartDeck = document.querySelector('.chart-deck');
  const chartEyebrow = document.querySelector('.stage-topline .eyebrow');
  const selectionLabel = document.querySelector('.selection-label');
  const chartPosition = document.querySelector('.chart-position');
  const priorButton = document.querySelector('.arrow-button[aria-label^="Previous"]');
  const nextButton = document.querySelector('.arrow-button[aria-label^="Next"]');
  const tabs = [...document.querySelectorAll('.chart-tab')];
  const payloadNode = document.querySelector('#chart-data');
  if (!startInput || !endInput || !startSlider || !endSlider || !payloadNode || !chartShell) return;

  let payload = JSON.parse(payloadNode.textContent || '{}');
  let dates = payload.dataset?.availableDates || (document.querySelector('.range-controls')?.dataset.availableDates || '').split(',').filter(Boolean);
  if (!dates.length) return;
  let activeChart = chartRegistry[new URLSearchParams(window.location.search).get('chart')] ? new URLSearchParams(window.location.search).get('chart') : 'calendar';
  let rangeTimer;

  const dateFormat = (value) => new Intl.DateTimeFormat('en-IN', {day: 'numeric', month: 'short', year: 'numeric'}).format(new Date(`${value}T00:00:00`));
  const nearestIndex = (value, fallback) => {
    if (!value) return fallback;
    const wanted = new Date(`${value}T00:00:00`).getTime();
    if (!Number.isFinite(wanted)) return fallback;
    let best = 0;
    let distance = Infinity;
    dates.forEach((date, index) => {
      const next = Math.abs(new Date(`${date}T00:00:00`).getTime() - wanted);
      if (next < distance) { best = index; distance = next; }
    });
    return best;
  };
  const compact = (value) => {
    if (value == null || Number.isNaN(value)) return '—';
    const abs = Math.abs(value);
    if (abs >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
    if (abs >= 1_000) return `${Math.round(value / 1_000)}k`;
    return `${Math.round(value)}`;
  };
  const colourFor = (cell) => cell.missing ? 'var(--surface)' : cell.bandIndex == null ? 'var(--grid)' : `var(--band-${cell.bandIndex})`;
  const setDisabled = (el, disabled) => {
    if (!el) return;
    el.setAttribute('aria-disabled', String(disabled));
    el.setAttribute('tabindex', disabled ? '-1' : '0');
  };
  const currentParams = () => new URLSearchParams(window.location.search);
  const updateUrl = ({start, end, chart = activeChart, replace = false}) => {
    const params = currentParams();
    params.set('start', start);
    params.set('end', end);
    params.set('chart', chart);
    const next = `${window.location.pathname}?${params.toString()}`;
    (replace ? history.replaceState : history.pushState).call(history, {start, end, chart}, '', next);
  };
  const showTooltip = (target) => {
    const text = target.dataset.tooltip || target.getAttribute('aria-label');
    if (!tooltip || !text) return;
    const bounds = target.getBoundingClientRect();
    tooltip.textContent = text;
    tooltip.hidden = false;
    tooltip.style.left = `${Math.min(window.innerWidth - 16, Math.max(16, bounds.left + bounds.width / 2))}px`;
    tooltip.style.top = `${Math.max(12, bounds.top - 10)}px`;
  };
  const hideTooltip = () => { if (tooltip) tooltip.hidden = true; };
  const bindTooltip = (selection) => {
    selection
      .attr('tabindex', 0)
      .attr('role', 'img')
      .on('mouseenter focus click', function () { showTooltip(this); })
      .on('mouseleave blur', hideTooltip);
  };
  const choreograph = (node) => {
    if (canAnimate && GSAP && node) GSAP.fromTo(node, {autoAlpha: 0, y: 8}, {autoAlpha: 1, y: 0, duration: 0.32, ease: 'power2.out'});
  };

  const syncControls = (nextPayload) => {
    payload = nextPayload;
    dates = payload.dataset.availableDates;
    startSlider.max = `${dates.length - 1}`;
    endSlider.max = `${dates.length - 1}`;
    startSlider.value = payload.range.startIndex;
    endSlider.value = payload.range.endIndex;
    startInput.value = payload.range.start;
    endInput.value = payload.range.end;
    readable.textContent = payload.range.label;
    if (selectionLabel) selectionLabel.textContent = payload.range.label;
  };

  const syncChartChrome = () => {
    const def = chartRegistry[activeChart];
    if (chartTitle) chartTitle.textContent = def.title;
    if (chartDeck) chartDeck.textContent = def.deck;
    if (chartEyebrow) chartEyebrow.textContent = `02 · CHART ${def.index + 1}`;
    if (chartPosition) chartPosition.textContent = `${def.index + 1} / 2`;
    setDisabled(priorButton, activeChart === 'calendar');
    setDisabled(nextButton, activeChart === 'commute-casual');
    tabs.forEach((tab, index) => tab.classList.toggle('active', index === def.index));
  };

  const ensureScene = (className) => {
    chartShell.classList.add('d3-enhanced');
    let scene = chartShell.querySelector(`.${className}`);
    if (!scene) {
      scene = document.createElement('div');
      scene.className = `d3-scene ${className}`;
      chartShell.prepend(scene);
    }
    chartShell.querySelectorAll('.d3-scene').forEach((candidate) => {
      candidate.hidden = candidate !== scene;
    });
    return D3.select(scene);
  };

  const renderCalendar = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-calendar-scene');
    const cells = payload.charts.calendar.cells;
    const maxWeek = D3.max(cells, d => d.week) || 0;
    const cell = 16, gap = 2, left = 44, top = 28;
    const width = Math.max(720, left + (maxWeek + 1) * (cell + gap) + 24);
    const height = 190;
    const svg = scene.selectAll('svg').data([null]).join('svg')
      .attr('class', 'd3-calendar-svg')
      .attr('viewBox', `0 0 ${width} ${height}`)
      .attr('role', 'img')
      .attr('aria-label', 'Daily ridership calendar heatmap, enhanced with D3');

    svg.selectAll('.weekday-label').data(['Mon','Tue','Wed','Thu','Fri','Sat','Sun']).join('text')
      .attr('class', 'weekday-label')
      .attr('x', 0)
      .attr('y', (_, i) => top + i * (cell + gap) + 12)
      .text(d => d);

    svg.selectAll('.d3-month-label').data(cells.filter(d => d.monthLabel), d => d.date).join(
      enter => enter.append('text').attr('class', 'd3-month-label').attr('opacity', 0).text(d => d.monthLabel),
      update => update.text(d => d.monthLabel),
      exit => exit.remove()
    )
      .attr('x', d => left + d.week * (cell + gap))
      .attr('y', 13)
      .transition().duration(canAnimate && animate ? 220 : 0)
      .attr('opacity', 1);

    const joined = svg.selectAll('.d3-cell').data(cells, d => d.date).join(
      enter => {
        const g = enter.append('g').attr('class', d => `d3-cell ${d.missing ? 'missing' : `band-${d.bandIndex ?? 'neutral'}`}`).attr('opacity', 0);
        g.append('rect').attr('width', cell).attr('height', cell);
        g.append('line').attr('class', 'missing-cross a').attr('x1', 2).attr('y1', 2).attr('x2', cell - 2).attr('y2', cell - 2);
        g.append('line').attr('class', 'missing-cross b').attr('x1', cell - 2).attr('y1', 2).attr('x2', 2).attr('y2', cell - 2);
        return g;
      },
      update => update,
      exit => exit.transition().duration(canAnimate && animate ? 160 : 0).attr('opacity', 0).remove()
    );

    joined
      .attr('class', d => `d3-cell ${d.missing ? 'missing' : `band-${d.bandIndex ?? 'neutral'}`}`)
      .attr('aria-label', d => d.label)
      .attr('data-tooltip', d => d.label)
      .transition().duration(canAnimate && animate ? 260 : 0)
      .attr('opacity', 1)
      .attr('transform', d => `translate(${left + d.week * (cell + gap)},${top + d.weekday * (cell + gap)})`);
    joined.select('rect').transition().duration(canAnimate && animate ? 260 : 0).attr('fill', colourFor);
    joined.selectAll('.missing-cross').attr('display', d => d.missing ? null : 'none');
    bindTooltip(joined);

    const legend = scene.selectAll('.d3-calendar-legend').data([payload.charts.calendar.legend]).join('div').attr('class', 'd3-calendar-legend legend');
    legend.selectAll('.d3-legend-item').data(d => d).join('span')
      .attr('class', 'd3-legend-item')
      .html(d => `<i class="swatch band-${d.bandIndex}"></i>${d.label}`);
    const rangeText = payload.summary.totalMinLabel && payload.summary.totalMaxLabel ? `Observed: ${compact(payload.summary.totalMin)} – ${compact(payload.summary.totalMax)}` : '';
    legend.selectAll('.d3-legend-range').data(rangeText ? [rangeText] : []).join('span').attr('class', 'legend-range d3-legend-range').text(d => d);
    choreograph(scene.node());
  };

  const renderLine = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-line-scene');
    const width = 900, height = 410, left = 66, right = 28, top = 24, bottom = 66;
    const plotWidth = width - left - right;
    const plotHeight = height - top - bottom;
    const allPoints = payload.charts.commuteCasual.series.flatMap(series => series.points);
    const maxValue = Math.max(1, D3.max(allPoints, p => (p.mean || 0) + (p.standardDeviation || 0)) || 1);
    const x = D3.scalePoint().domain(payload.charts.commuteCasual.weekdays.map(d => d.index)).range([left, left + plotWidth]);
    const y = D3.scaleLinear().domain([0, maxValue]).nice(4).range([top + plotHeight, top]);
    const svg = scene.selectAll('svg').data([null]).join('svg')
      .attr('class', 'd3-line-svg line-chart')
      .attr('viewBox', `0 0 ${width} ${height}`)
      .attr('role', 'img')
      .attr('aria-label', 'Average daily ridership by weekday, enhanced with D3');

    svg.selectAll('.d3-grid-line').data(y.ticks(4)).join('line')
      .attr('class', 'd3-grid-line')
      .attr('x1', left).attr('x2', width - right)
      .transition().duration(canAnimate && animate ? 220 : 0)
      .attr('y1', d => y(d)).attr('y2', d => y(d));
    svg.selectAll('.d3-y-label').data(y.ticks(4)).join('text')
      .attr('class', 'd3-y-label')
      .attr('x', left - 10).attr('text-anchor', 'end')
      .transition().duration(canAnimate && animate ? 220 : 0)
      .attr('y', d => y(d) + 4)
      .text(d => compact(d));
    svg.selectAll('.d3-x-label').data(payload.charts.commuteCasual.weekdays, d => d.index).join('text')
      .attr('class', 'd3-x-label')
      .attr('x', d => x(d.index)).attr('y', height - 26).attr('text-anchor', 'middle')
      .text(d => d.short);

    const line = D3.line().defined(d => d.mean != null).x(d => x(d.index)).y(d => y(d.mean));
    const series = svg.selectAll('.d3-series').data(payload.charts.commuteCasual.series, d => d.key).join('g').attr('class', d => `d3-series ${d.key}`);
    series.selectAll('.d3-series-path').data(d => [d]).join('path')
      .attr('class', d => `d3-series-path series ${d.key}`)
      .attr('fill', 'none')
      .transition().duration(canAnimate && animate ? 320 : 0)
      .attr('d', d => line(d.points));

    const pointGroups = series.selectAll('.d3-point-group').data(d => d.points.map(point => ({...point, seriesKey: d.key})), d => `${d.seriesKey}-${d.index}`).join('g')
      .attr('class', d => `d3-point-group ${d.seriesKey}`)
      .attr('aria-label', d => d.tooltip)
      .attr('data-tooltip', d => d.tooltip);
    pointGroups.transition().duration(canAnimate && animate ? 320 : 0)
      .attr('opacity', d => d.mean == null ? 0 : 1)
      .attr('transform', d => `translate(${x(d.index)},${d.mean == null ? y(0) : y(d.mean)})`);
    pointGroups.selectAll('line').data(d => d.mean != null && d.standardDeviation != null ? [d] : []).join('line')
      .attr('class', d => `d3-whisker whisker ${d.seriesKey}`)
      .attr('x1', 0).attr('x2', 0)
      .transition().duration(canAnimate && animate ? 320 : 0)
      .attr('y1', d => y(Math.min(maxValue, d.mean + d.standardDeviation)) - y(d.mean))
      .attr('y2', d => y(Math.max(0, d.mean - d.standardDeviation)) - y(d.mean));
    pointGroups.selectAll('circle').data(d => d.mean == null ? [] : [d]).join('circle')
      .attr('class', d => `d3-point point ${d.seriesKey}`)
      .attr('r', 6);
    pointGroups.selectAll('text').data(d => d.mean == null ? [] : [d]).join('text')
      .attr('class', d => `d3-point-label point-label ${d.seriesKey}`)
      .attr('text-anchor', 'middle')
      .attr('y', -14)
      .text(d => compact(d.mean));
    bindTooltip(pointGroups);

    svg.selectAll('.d3-series-label').data(payload.charts.commuteCasual.series, d => d.key).join('text')
      .attr('class', d => `d3-series-label series-label ${d.key}`)
      .attr('x', width - 210)
      .attr('y', d => d.key === 'commute' ? 25 : 48)
      .text(d => d.label);
    scene.selectAll('.d3-insight').data(payload.charts.commuteCasual.insight ? [payload.charts.commuteCasual.insight] : []).join('p')
      .attr('class', 'd3-insight insight')
      .text(d => d);
    choreograph(scene.node());
  };

  const renderActiveChart = (animate = true) => {
    syncChartChrome();
    if (activeChart === 'calendar') renderCalendar(animate);
    else renderLine(animate);
  };

  const fetchRange = async (start, end, {replace = false, animate = true} = {}) => {
    status.textContent = 'Updating the chart…';
    const params = new URLSearchParams({start, end});
    const response = await fetch(`/api/chart?${params.toString()}`, {headers: {'Accept': 'application/json'}});
    if (!response.ok) throw new Error(`chart payload failed: ${response.status}`);
    const nextPayload = await response.json();
    syncControls(nextPayload);
    updateUrl({start: nextPayload.range.start, end: nextPayload.range.end, replace});
    renderActiveChart(animate);
    status.textContent = `Showing ${nextPayload.range.label}.`;
  };

  const sync = (source, delay = 280) => {
    let startIndex = Number(startSlider.value);
    let endIndex = Number(endSlider.value);
    if (startIndex > endIndex) {
      if (source === 'start') endSlider.value = startIndex;
      else startSlider.value = endIndex;
    }
    const start = dates[Number(startSlider.value)];
    const end = dates[Number(endSlider.value)];
    startInput.value = start;
    endInput.value = end;
    readable.textContent = `${dateFormat(start)} – ${dateFormat(end)}`;
    status.textContent = delay ? 'Release the slider to update the chart.' : 'Updating the chart…';
    clearTimeout(rangeTimer);
    rangeTimer = setTimeout(() => fetchRange(start, end, {replace: true}).catch(error => { status.textContent = error.message; }), delay);
  };
  const syncDateInput = (source) => {
    if (source === 'start') startSlider.value = nearestIndex(startInput.value, 0);
    else endSlider.value = nearestIndex(endInput.value, dates.length - 1);
    sync(source, 0);
  };

  startSlider.addEventListener('input', () => sync('start'));
  endSlider.addEventListener('input', () => sync('end'));
  startInput.addEventListener('change', () => syncDateInput('start'));
  endInput.addEventListener('change', () => syncDateInput('end'));

  document.querySelectorAll('[aria-disabled="true"]').forEach((control) => {
    control.addEventListener('click', (event) => event.preventDefault());
  });
  document.querySelector('.range-actions')?.addEventListener('click', (event) => {
    const link = event.target.closest('a[href]');
    if (!link) return;
    event.preventDefault();
    if (link.getAttribute('aria-disabled') === 'true') return;
    const params = new URLSearchParams(new URL(link.href).search);
    fetchRange(params.get('start'), params.get('end'), {replace: false}).catch(error => { status.textContent = error.message; });
  });
  document.querySelector('.navigation-row')?.addEventListener('click', (event) => {
    const link = event.target.closest('a[href]');
    if (!link) return;
    event.preventDefault();
    if (link.getAttribute('aria-disabled') === 'true') return;
    const params = new URLSearchParams(new URL(link.href).search);
    activeChart = chartRegistry[params.get('chart')] ? params.get('chart') : activeChart;
    updateUrl({start: payload.range.start, end: payload.range.end, chart: activeChart});
    renderActiveChart(true);
  });
  document.addEventListener('keydown', (event) => {
    if (['INPUT', 'TEXTAREA', 'SELECT', 'BUTTON'].includes(document.activeElement?.tagName)) return;
    if (event.key === 'ArrowLeft' && activeChart !== 'calendar') {
      activeChart = 'calendar';
      updateUrl({start: payload.range.start, end: payload.range.end, chart: activeChart});
      renderActiveChart(true);
    }
    if (event.key === 'ArrowRight' && activeChart !== 'commute-casual') {
      activeChart = 'commute-casual';
      updateUrl({start: payload.range.start, end: payload.range.end, chart: activeChart});
      renderActiveChart(true);
    }
  });
  window.addEventListener('popstate', () => {
    const params = currentParams();
    activeChart = chartRegistry[params.get('chart')] ? params.get('chart') : 'calendar';
    renderActiveChart(false);
  });

  if (D3) {
    syncControls(payload);
    renderActiveChart(false);
  }
})();"#;
const STYLE: &str = r#"<style>
:root{--bg:#f7f5f8;--surface:#fcfbfc;--ink:#171417;--muted:#68636a;--grid:#d3cdd5;--mauve-strong:#7138a8;--mauve-mid:#8c5bbc;--ochre:#a9831e;--missing:#bf705d;--band-0:#f0eaf3;--band-1:#e6d9ed;--band-2:#dac6e5;--band-3:#ccb0dc;--band-4:#bb95d0;--band-5:#a979c1;--band-6:#955fb4;--band-7:#874baa;--band-8:#793da2;--band-9:#642a96;--serif:Georgia,'Times New Roman',serif;--sans:ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif}*{box-sizing:border-box}html{background:var(--bg);color:var(--ink);font-family:var(--sans)}body{margin:0;min-width:320px}.site-shell{max-width:1360px;margin:0 auto;padding:32px clamp(20px,5vw,72px) 40px}.site-header{display:flex;justify-content:space-between;align-items:flex-end;gap:32px;padding-bottom:28px;border-bottom:1px solid var(--grid)}h1,h2,p{margin-top:0}.site-header h1{margin-bottom:7px;font:600 clamp(2.2rem,5vw,4.8rem)/.95 var(--serif);letter-spacing:-.055em}.subtitle{color:var(--muted);font-size:1.05rem}.eyebrow{margin-bottom:10px;color:var(--mauve-strong);font-size:.68rem;font-weight:800;letter-spacing:.16em}.source-link,.text-button{color:var(--mauve-strong);font-size:.84rem;font-weight:700;text-underline-offset:4px}.prepare-band{display:grid;grid-template-columns:minmax(230px,.8fr) 2fr;gap:48px;padding:28px 0 32px;border-bottom:1px solid var(--grid)}.band-heading h2{font:600 2rem/1 var(--serif);margin-bottom:12px}.band-heading p:not(.eyebrow){max-width:36ch;color:var(--muted);font-size:.9rem;line-height:1.5}.range-controls{display:grid;grid-template-columns:auto 1fr;align-items:center;gap:12px 22px}.range-inputs{display:flex;align-items:end;gap:12px}.range-inputs label{display:grid;gap:6px;color:var(--muted);font-size:.7rem;font-weight:800;letter-spacing:.08em;text-transform:uppercase}.range-inputs input{border:1px solid var(--grid);background:var(--surface);padding:10px;color:var(--ink);font:600 .9rem var(--sans)}.range-dash{color:var(--muted);padding-bottom:10px}.range-readable{grid-column:2;font:600 1.2rem var(--serif)}.range-slider{grid-column:1/-1;position:relative;height:30px}.slider-track{position:absolute;top:13px;left:0;right:0;height:4px;background:var(--grid)}.range-slider input{position:absolute;top:0;left:0;width:100%;height:30px;margin:0;pointer-events:none;appearance:none;background:none}.range-slider input::-webkit-slider-thumb{width:22px;height:22px;border:3px solid var(--surface);border-radius:50%;background:var(--mauve-strong);box-shadow:0 0 0 1px var(--mauve-strong);appearance:none;pointer-events:auto;cursor:grab}.range-slider input::-moz-range-thumb{width:16px;height:16px;border:3px solid var(--surface);border-radius:50%;background:var(--mauve-strong);box-shadow:0 0 0 1px var(--mauve-strong);pointer-events:auto;cursor:grab}.range-actions{grid-column:1/-1;display:flex;gap:22px;flex-wrap:wrap}.range-status{grid-column:1/-1;margin:0;color:var(--muted);font-size:.78rem}.chart-stage{padding:34px 0 20px}.stage-topline{display:flex;justify-content:space-between;gap:20px}.selection-label{margin:0;color:var(--muted);font-size:.8rem}.chart-stage>h2{margin:12px 0 8px;font:600 clamp(2rem,4vw,3.6rem)/1 var(--serif);letter-spacing:-.04em}.chart-deck{max-width:65ch;color:var(--muted);font-size:1rem}.chart-shell{min-height:410px;margin:22px 0 12px;padding:20px 0}.chart-shell.d3-enhanced .calendar-wrap,.chart-shell.d3-enhanced .line-chart-wrap{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap}.d3-scene{width:100%;overflow-x:auto}.d3-scene[hidden]{display:none}.d3-calendar-svg,.d3-line-svg{display:block;width:100%;min-width:700px;height:auto;overflow:visible}.d3-calendar-legend{margin-top:18px}.weekday-label,.d3-month-label,.d3-y-label,.d3-x-label{fill:var(--muted);font:700 12px var(--sans)}.d3-month-label{font-size:11px}.d3-cell rect{stroke:transparent}.d3-cell:focus{outline:none}.d3-cell:focus rect,.d3-cell:hover rect{stroke:var(--ink);stroke-width:3}.missing-cross{stroke:var(--missing);stroke-width:1.8}.d3-grid-line{stroke:var(--grid);stroke-width:1}.d3-series-path{stroke-width:3}.d3-series-path.commute,.d3-point.commute,.d3-whisker.commute{stroke:var(--mauve-strong);fill:var(--mauve-strong)}.d3-series-path.casual,.d3-point.casual,.d3-whisker.casual{stroke:var(--ochre);fill:var(--ochre)}.d3-point{stroke:var(--surface);stroke-width:3;cursor:pointer}.d3-point-group:focus .d3-point{stroke:var(--ink);stroke-width:4}.d3-whisker{stroke-width:2;opacity:.45}.d3-point-label{font:700 12px var(--sans)}.d3-point-label.commute,.d3-series-label.commute{fill:var(--mauve-strong)}.d3-point-label.casual,.d3-series-label.casual{fill:var(--ochre)}.d3-series-label{font:700 12px var(--sans)}.chart-method{color:var(--muted);font-size:.78rem;line-height:1.5;border-left:3px solid var(--mauve-mid);padding-left:12px}.calendar-wrap{overflow-x:auto;padding:32px 0 6px}.calendar-grid{display:grid;grid-template-columns:42px 1fr;min-width:720px;position:relative}.weekday-labels{display:grid;grid-template-rows:repeat(7,16px);gap:2px;padding-top:1px;color:var(--muted);font-size:.67rem;font-weight:700}.weekday-labels span{align-self:center}.calendar-canvas{display:flex;gap:2px;position:relative;padding-top:24px}.calendar-week{display:grid;grid-template-rows:repeat(7,16px);gap:2px;min-width:16px}.calendar-cell{width:16px;height:16px;padding:0;border:0;background:var(--grid);position:relative;cursor:pointer}.calendar-cell:hover,.calendar-cell:focus-visible{outline:3px solid var(--ink);outline-offset:2px;z-index:2}.calendar-cell.structural{background:transparent;cursor:default}.calendar-cell.missing{background:var(--surface);border:1px solid var(--missing);background-image:linear-gradient(45deg,transparent 44%,var(--missing) 45%,var(--missing) 55%,transparent 56%),linear-gradient(-45deg,transparent 44%,var(--missing) 45%,var(--missing) 55%,transparent 56%)}.calendar-cell.neutral{background:var(--grid)}.band-0{background:var(--band-0)}.band-1{background:var(--band-1)}.band-2{background:var(--band-2)}.band-3{background:var(--band-3)}.band-4{background:var(--band-4)}.band-5{background:var(--band-5)}.band-6{background:var(--band-6)}.band-7{background:var(--band-7)}.band-8{background:var(--band-8)}.band-9{background:var(--band-9)}.month-label{position:absolute;top:0;color:var(--muted);font-size:.7rem;font-weight:800;transform:translateX(calc(var(--week) * 18px));white-space:nowrap}.legend{display:flex;gap:11px 18px;align-items:center;flex-wrap:wrap;margin:18px 0 0;color:var(--muted);font-size:.71rem}.legend span{display:inline-flex;gap:6px;align-items:center}.swatch{display:inline-block;width:13px;height:13px;background:var(--mauve-mid)}.missing-swatch{border:1px solid var(--missing);background:linear-gradient(45deg,transparent 43%,var(--missing) 44%,var(--missing) 56%,transparent 57%),linear-gradient(-45deg,transparent 43%,var(--missing) 44%,var(--missing) 56%,transparent 57%)}.legend-range{font-weight:800;color:var(--ink)}.notice{flex-basis:100%;margin:0;color:var(--missing);font-weight:700}.line-chart-wrap{width:100%;overflow-x:auto}.line-chart{display:block;width:100%;min-width:700px;height:auto;overflow:visible}.grid line{stroke:var(--grid);stroke-width:1}.grid text,.x-label{fill:var(--muted);font:12px var(--sans)}.series{stroke-width:3}.series.commute,.point.commute,.whisker.commute{stroke:var(--mauve-strong);fill:var(--mauve-strong)}.series.casual,.point.casual,.whisker.casual{stroke:var(--ochre);fill:var(--ochre)}.point{stroke:var(--surface);stroke-width:3;cursor:pointer}.point:focus{outline:none;stroke:var(--ink);stroke-width:4}.whisker{stroke-width:2;opacity:.45}.point-label{font:700 12px var(--sans)}.point-label.commute,.series-label.commute{fill:var(--mauve-strong)}.point-label.casual,.series-label.casual{fill:var(--ochre)}.series-label{font:700 12px var(--sans)}.insight{margin:4px 0 18px;color:var(--ink);font:600 1.1rem/1.35 var(--serif)}.accessible-data{margin-top:20px}.accessible-data summary{color:var(--mauve-strong);font-size:.82rem;font-weight:800;cursor:pointer}.table-scroll{overflow-x:auto}table{width:100%;border-collapse:collapse;font-size:.75rem}th,td{padding:8px 10px;border-bottom:1px solid var(--grid);text-align:left;white-space:nowrap}th{color:var(--muted);font-size:.68rem;letter-spacing:.06em;text-transform:uppercase}.navigation-row{display:grid;grid-template-columns:1fr auto 1fr;align-items:center;gap:20px;margin-top:32px}.arrow-button{display:inline-flex;align-items:center;gap:12px;width:max-content;padding:12px 0;border:0;background:transparent;color:var(--ink);font-weight:800;text-decoration:none}.arrow-button:last-child{justify-self:end}.arrow-button[aria-disabled="false"]:hover{color:var(--mauve-strong)}.arrow-button[aria-disabled="true"]{color:#aaa5ac;cursor:not-allowed;pointer-events:none}.arrow-glyph{font-size:2rem;font-weight:400;line-height:.5}.chart-nav{display:flex;align-items:center;gap:16px}.chart-tab{color:var(--muted);font-size:.75rem;font-weight:800;text-decoration:none}.chart-tab.active{color:var(--ink);border-bottom:2px solid var(--mauve-strong);padding-bottom:5px}.chart-position{font-weight:800}.site-footer{display:grid;grid-template-columns:1fr 1fr;gap:40px;padding-top:26px;border-top:1px solid var(--grid);color:var(--muted);font-size:.78rem;line-height:1.5}.site-footer p{margin-bottom:7px}.site-footer summary{color:var(--ink);font-weight:800;cursor:pointer}.site-footer details p{max-width:62ch}.button{display:inline-block;padding:12px 16px;background:var(--mauve-strong);color:#fff;text-decoration:none;font-weight:800}.error-page{max-width:650px;margin:15vh auto}.sr-only{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0}.chart-tooltip{position:fixed;z-index:20;max-width:300px;transform:translate(-50%,-100%);padding:8px 10px;background:var(--ink);color:#fff;font-size:.72rem;line-height:1.35;pointer-events:none;box-shadow:0 6px 22px rgba(23,20,23,.25)}.chart-tooltip[hidden]{display:none}:focus-visible{outline:3px solid var(--ink);outline-offset:3px}@media(max-width:760px){.site-shell{padding:22px 18px 30px}.site-header,.prepare-band,.site-footer{display:block}.source-link{display:inline-block;margin-top:18px}.prepare-band{padding-bottom:22px}.range-controls{display:block}.range-readable{margin:14px 0}.range-actions{margin:16px 0}.chart-stage{padding-top:24px}.stage-topline{display:block}.selection-label{margin-top:10px}.chart-shell{min-height:330px;padding-left:0}.navigation-row{grid-template-columns:1fr 1fr;gap:10px}.chart-nav{grid-column:1/-1;grid-row:1;justify-content:center;order:0}.arrow-button{grid-row:2}.arrow-button:last-child{justify-self:end}.site-footer{padding-top:22px}.site-footer details{margin-top:24px}.calendar-wrap{margin-right:-18px;padding-right:18px}.line-chart,.d3-calendar-svg,.d3-line-svg{min-width:680px}.point-label,.d3-point-label{font-size:10px}}@media(prefers-reduced-motion:reduce){*,*::before,*::after{scroll-behavior:auto!important;transition:none!important}}
</style>"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dataset() -> Dataset {
        parse_dataset(
            include_str!("../tests/fixtures/ridership_fixture.csv"),
            "fixture".into(),
        )
        .unwrap()
    }

    #[test]
    fn parses_aliases_numeric_values_and_dates() {
        let dataset = fixture_dataset();
        assert_eq!(dataset.records.len(), 20);
        assert_eq!(dataset.invalid_date_count, 1);
        assert_eq!(
            dataset.records[0].date,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
        assert_eq!(dataset.records[0].smart_card, Some(1000.0));
        assert_eq!(dataset.records[0].total_ridership, Some(1650.0));
    }

    #[test]
    fn keeps_latest_duplicate_without_adding_values() {
        let dataset = fixture_dataset();
        assert_eq!(dataset.duplicate_count, 1);
        let record = dataset
            .records
            .iter()
            .find(|record| record.date == NaiveDate::from_ymd_opt(2026, 1, 20).unwrap())
            .unwrap();
        assert_eq!(record.smart_card, Some(9999.0));
    }

    #[test]
    fn missing_component_does_not_become_zero() {
        let dataset = fixture_dataset();
        let record = dataset
            .records
            .iter()
            .find(|record| record.date == NaiveDate::from_ymd_opt(2026, 1, 3).unwrap())
            .unwrap();
        assert_eq!(record.group_ticket, None);
        assert_eq!(record.casual_ridership, None);
    }

    #[test]
    fn range_is_inclusive_and_defaults_clamp() {
        let dataset = fixture_dataset();
        let range = choose_range(&dataset, Some("2025-01-01"), Some("2027-01-01"));
        assert_eq!(range.start, dataset.min_date);
        assert_eq!(range.end, dataset.max_date);
        let summary = summarise(
            &dataset,
            DateRange {
                start: dataset.records[0].date,
                end: dataset.records[1].date,
            },
        );
        assert_eq!(summary.calendar_days, 2);
        assert_eq!(summary.observation_days, 2);
    }

    #[test]
    fn quantile_is_interpolated_and_ties_are_classifiable() {
        assert_eq!(quantile(&[1.0, 2.0, 3.0, 4.0], 0.25), Some(1.75));
        assert_eq!(quantile(&[1.0, 1.0, 1.0], 0.95), Some(1.0));
        assert_eq!(quantile(&[], 0.5), None);
    }

    #[test]
    fn standard_deviation_uses_sample_denominator() {
        let dataset = fixture_dataset();
        let summary = summarise(
            &dataset,
            DateRange {
                start: dataset.min_date,
                end: dataset.max_date,
            },
        );
        let monday = summary
            .commute
            .iter()
            .find(|metric| metric.weekday == Weekday::Mon)
            .unwrap();
        assert_eq!(monday.n, 3);
        assert!(monday.standard_deviation.unwrap() > 0.0);
    }

    #[test]
    fn weekday_order_is_monday_first() {
        let dataset = fixture_dataset();
        let summary = summarise(
            &dataset,
            DateRange {
                start: dataset.min_date,
                end: dataset.max_date,
            },
        );
        assert_eq!(
            summary
                .commute
                .iter()
                .map(|metric| metric.weekday)
                .collect::<Vec<_>>(),
            vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
                Weekday::Sat,
                Weekday::Sun
            ]
        );
    }

    #[test]
    fn query_parser_handles_malformed_values_without_panicking() {
        let query = parse_query("start=nope&end=2026-01-03&chart=commute-casual&ignored=yes");
        assert_eq!(query.start.as_deref(), Some("nope"));
        assert_eq!(
            Chart::from_query(query.chart.as_deref()),
            Chart::CommuteCasual
        );
        assert!(!query.retry);
    }

    #[test]
    fn query_parser_recognises_an_explicit_retry() {
        let query = parse_query("retry=1");
        assert!(query.retry);
    }

    #[test]
    fn numeric_cleaning_trims_commas_and_preserves_missing_values() {
        assert_eq!(clean_number(Some(" 1,234,567 ")), Some(1_234_567.0));
        assert_eq!(clean_number(Some("")), None);
        assert_eq!(clean_number(Some("   ")), None);
        assert_eq!(clean_number(Some("not a number")), None);
        assert_eq!(clean_number(None), None);
    }

    #[test]
    fn commuter_and_casual_derivations_use_only_complete_components() {
        let dataset = fixture_dataset();
        let complete = &dataset.records[0];
        assert_eq!(complete.commuter_ridership, Some(1_100.0));
        assert_eq!(complete.casual_ridership, Some(550.0));

        let incomplete = dataset
            .records
            .iter()
            .find(|record| record.date == NaiveDate::from_ymd_opt(2026, 1, 3).unwrap())
            .unwrap();
        assert_eq!(incomplete.commuter_ridership, Some(1_300.0));
        assert_eq!(incomplete.casual_ridership, None);
    }

    #[test]
    fn range_links_serialise_dates_and_preserve_chart() {
        let dataset = fixture_dataset();
        assert_eq!(
            default_range_link(&dataset, Chart::CommuteCasual),
            "?start=2026-01-01&end=2026-01-20&chart=commute-casual"
        );
        assert_eq!(
            all_data_link(&dataset, Chart::Calendar),
            "?start=2026-01-01&end=2026-01-20&chart=calendar"
        );
    }

    #[test]
    fn equal_percentile_boundaries_never_leave_values_unclassified() {
        let summary = RangeSummary {
            calendar_days: 10,
            observation_days: 10,
            missing_days: 0,
            percentiles: vec![
                (2, 100.0),
                (5, 100.0),
                (10, 100.0),
                (25, 100.0),
                (50, 100.0),
                (75, 100.0),
                (90, 100.0),
                (95, 100.0),
                (98, 100.0),
            ],
            valid_total_count: 10,
            total_min: Some(100.0),
            total_max: Some(100.0),
            commute: Vec::new(),
            casual: Vec::new(),
        };
        assert_eq!(band_label(99.0, &summary), Some("< p2"));
        assert_eq!(band_label(100.0, &summary), Some("> p98"));
    }

    #[test]
    fn calendar_handles_leap_day_partial_weeks_and_month_boundaries() {
        let dataset = parse_dataset(
            "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR\n28-02-2024,100,10,20,2,30\n05-03-2024,120,12,22,2,32\n",
            "fixture".into(),
        )
        .unwrap();
        // Range spans a partial February week, the leap day, and into a pure-March week.
        let range = DateRange {
            start: NaiveDate::from_ymd_opt(2024, 2, 28).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 3, 6).unwrap(),
        };
        let summary = summarise(&dataset, range);
        let markup = calendar_markup(&dataset, range, &summary);

        assert!(markup.contains("data-date=\"2024-02-28\""));
        assert!(markup.contains("data-date=\"2024-02-29\""));
        assert!(markup.contains("data-date=\"2024-03-01\""));
        assert!(markup.contains("data-date=\"2024-03-06\""));
        // The leap day has no observation, so it must render as a crossed missing cell.
        assert!(markup.contains("calendar-cell missing"));
        assert!(markup.contains("Thursday, 29 February 2024: Missing data"));
        // Both month blocks are labelled even though February starts mid-week.
        assert!(markup.contains("Feb 2024"));
        assert!(markup.contains("Mar 2024"));
        assert!(markup.contains("data-tooltip="));
        assert_eq!(summary.calendar_days, 8);
        assert_eq!(summary.observation_days, 2);
        assert_eq!(summary.missing_days, 6);
    }

    #[test]
    fn chart_payload_is_chart_ready_without_js_business_logic() {
        let dataset = fixture_dataset();
        let range = DateRange {
            start: dataset.min_date,
            end: dataset.max_date,
        };
        let payload = chart_payload_for(&dataset, range);
        assert_eq!(payload.range.start, "2026-01-01");
        assert_eq!(payload.range.end, "2026-01-20");
        assert_eq!(payload.summary.calendar_days, 20);
        assert_eq!(payload.charts.calendar.cells.len(), 20);
        assert_eq!(payload.charts.calendar.legend.len(), 10);
        assert!(
            payload
                .charts
                .calendar
                .cells
                .iter()
                .any(|cell| cell.missing || cell.band_index.is_some())
        );
        assert_eq!(payload.charts.commute_casual.weekdays[0].short, "Mon");
        assert_eq!(payload.charts.commute_casual.series.len(), 2);
        assert_eq!(payload.charts.commute_casual.series[0].key, "commute");
        assert_eq!(payload.charts.commute_casual.series[1].key, "casual");
        assert!(
            serde_json::to_string(&payload)
                .unwrap()
                .contains("commuteCasual")
        );
    }

    #[test]
    fn text_date_controls_snap_and_recover_when_cleared() {
        assert!(CLIENT_SCRIPT.contains("nearestIndex(startInput.value, 0)"));
        assert!(CLIENT_SCRIPT.contains("nearestIndex(endInput.value, dates.length - 1)"));
        assert!(CLIENT_SCRIPT.contains("/api/chart"));
        assert!(CLIENT_SCRIPT.contains("D3.select"));
        assert!(CLIENT_SCRIPT.contains("GSAP.fromTo"));
    }

    #[tokio::test]
    async fn chart_api_returns_rust_computed_payload() {
        let dataset = fixture_dataset();
        let request = topcoat::router::Request::builder()
            .uri("/api/chart?start=2026-01-01&end=2026-01-02")
            .body(topcoat::router::Body::empty())
            .unwrap();
        let state = AppState {
            dataset: Arc::new(RwLock::new(Some(dataset))),
            client: Client::new(),
        };
        let response = Router::builder()
            .discover()
            .app_context(state)
            .app_context(AssetBundle::empty())
            .build()
            .handle(request)
            .await;
        assert!(response.status().is_success());
        let body = topcoat::router::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["range"]["start"], "2026-01-01");
        assert_eq!(json["range"]["end"], "2026-01-02");
        assert_eq!(
            json["charts"]["calendar"]["cells"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            json["charts"]["commuteCasual"]["series"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn home_page_renders_failure_view_without_dataset() {
        let request = topcoat::router::Request::builder()
            .uri("/")
            .body(topcoat::router::Body::empty())
            .unwrap();
        let state = AppState {
            dataset: Arc::new(RwLock::new(None)),
            client: Client::new(),
        };
        let response = Router::builder()
            .discover()
            .app_context(state)
            .app_context(AssetBundle::empty())
            .build()
            .handle(request)
            .await;
        let status = response.status();
        let body = topcoat::router::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);
        assert!(status.is_success() || status == topcoat::router::StatusCode::OK);
        assert!(html.contains("The source data could not be loaded."));
    }
}
