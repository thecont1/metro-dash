use chrono::{Datelike, FixedOffset, NaiveDate};
use serde::Deserialize;
use topcoat::{context::Cx, view::{Unescaped, view}, Result};

use crate::charts::{calendar_markup, data_card_markup, line_chart_markup, Chart, CHARTS};
use crate::client::{CLIENT_SCRIPT, WRAPPER_CLOSE, WRAPPER_OPEN};
use crate::data::{
    choose_range, default_range, record_index, Dataset, DEFAULT_END, DEFAULT_START,
    SOURCE_PAGE_URL,
};
use crate::payload::chart_payload_for;
use crate::style::STYLE;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct QueryState {
    pub start: Option<String>,
    pub end: Option<String>,
    pub chart: Option<String>,
    pub retry: bool,
}

pub fn parse_query(query: &str) -> QueryState {
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

pub fn default_range_link(dataset: &Dataset, chart: Chart) -> String {
    let range = default_range(dataset);
    format!(
        "?start={}&end={}&chart={}",
        range.start,
        range.end,
        chart.slug()
    )
}

pub fn all_data_link(dataset: &Dataset, chart: Chart) -> String {
    format!(
        "?start={}&end={}&chart={}",
        dataset.min_date,
        dataset.max_date,
        chart.slug()
    )
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub fn date_select_markup(id_prefix: &str, date: NaiveDate, min: NaiveDate, max: NaiveDate) -> String {
    let day = date.day();
    let month = date.month() as usize;
    let year = date.year();

    let min_year = min.year();
    let max_year = max.year();

    let mut day_opts = String::new();
    for d in 1..=31 {
        let selected = if d == day { " selected" } else { "" };
        day_opts.push_str(&format!("<option value=\"{d}\"{selected}>{d}</option>"));
    }

    let mut month_opts = String::new();
    for (i, name) in MONTHS.iter().enumerate() {
        let m = i + 1;
        let selected = if m == month { " selected" } else { "" };
        month_opts.push_str(&format!("<option value=\"{m}\"{selected}>{name}</option>"));
    }

    let mut year_opts = String::new();
    for y in min_year..=max_year {
        let selected = if y == year { " selected" } else { "" };
        year_opts.push_str(&format!("<option value=\"{y}\"{selected}>{y}</option>"));
    }

    format!(
        r#"<select id="{id_prefix}-day" class="date-select date-day" aria-label="Day">{day_opts}</select><select id="{id_prefix}-month" class="date-select date-month" aria-label="Month">{month_opts}</select><select id="{id_prefix}-year" class="date-select date-year" aria-label="Year">{year_opts}</select>"#
    )
}

pub async fn render_view(cx: &Cx, dataset: Dataset, query: QueryState, chart: Chart) -> Result {
    let range = choose_range(&dataset, query.start.as_deref(), query.end.as_deref());
    let summary = crate::data::summarise(&dataset, range);
    let payload_json = serde_json::to_string(&chart_payload_for(&dataset, range))
        .expect("chart payload should serialize")
        .replace("</", "<\\/");
    let chart_idx = chart.index();
    let prev_chart = if chart_idx == 0 { chart } else { CHARTS[chart_idx - 1].chart };
    let next_chart = if chart_idx + 1 < CHARTS.len() { CHARTS[chart_idx + 1].chart } else { chart };
    let previous_href = format!("?start={}&end={}&chart={}", range.start, range.end, prev_chart.slug());
    let next_href = format!(
        "?start={}&end={}&chart={}",
        range.start, range.end, next_chart.slug()
    );
    let definition = chart.definition();
    let chart_title = definition.title;
    let chart_deck = definition.deck;
    let chart_body = match chart {
        Chart::DataCard => data_card_markup(&dataset, range),
        Chart::Calendar => calendar_markup(&dataset, range, &summary),
        Chart::CommuteCasual => line_chart_markup(&summary),
    };
    let start_date_selects =
        date_select_markup("start-date", range.start, dataset.min_date, dataset.max_date);
    let end_date_selects =
        date_select_markup("end-date", range.end, dataset.min_date, dataset.max_date);
    let previous_definition = prev_chart.definition();
    let next_definition = next_chart.definition();
    let prior_disabled = chart_idx == 0;
    let next_disabled = chart_idx + 1 >= CHARTS.len();
    let ist = FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap();
    let refreshed_ist = chrono::DateTime::parse_from_rfc3339(&dataset.refreshed_at)
        .map(|dt| dt.with_timezone(&ist).format("%-d %b %Y, %-I:%M %p IST").to_string())
        .unwrap_or_else(|_| dataset.refreshed_at.clone());
    let data_note = format!(
        "{} – {} · {} rows · latest {} · retrieved {}",
        dataset.min_date.format("%-d %b %Y"),
        dataset.max_date.format("%-d %b %Y"),
        dataset.records.len(),
        dataset.max_date.format("%-d %b %Y"),
        refreshed_ist
    );
    let reset_range = choose_range(
        &dataset,
        Some(DEFAULT_START),
        Some(DEFAULT_END),
    );
    let reset_disabled = range == reset_range;
    let all_disabled = range.start == dataset.min_date && range.end == dataset.max_date;
    let chart_index_str = (chart.index() + 1).to_string();
    let chart_total_str = CHARTS.len().to_string();
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
            <body class=(format!("chart-{}", chart.slug()))>
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
                    <div class="prepare-band">
                        <div
                            class="range-controls"
                            data-available-dates=(available_dates)
                        >
                            <label class="range-input">
                                <span class="range-input-label">"From"</span>
                                <div class="date-picker" data-date-type="start">
                                    (Unescaped::new_unchecked(start_date_selects))
                                </div>
                            </label>
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
                            <label class="range-input">
                                <span class="range-input-label">"To"</span>
                                <div class="date-picker" data-date-type="end">
                                    (Unescaped::new_unchecked(end_date_selects))
                                </div>
                            </label>
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
                    </div>
                    <section class="chart-stage" aria-labelledby="chart-title">
                        <div class="stage-topline">
                            <p class="eyebrow">
                                "Chart "
                                <span class="chart-index">(chart_index_str)</span>
                                " of "
                                <span class="chart-total">(chart_total_str)</span>
                            </p>
                        </div>
                        <div class="chart-title-row">
                            <h2 id="chart-title">(chart_title)</h2>
                            <div class="chart-pager">
                                <a
                                    class="arrow-button prev"
                                    aria-label=(format!("Previous chart: {}", previous_definition.title))
                                    href=(previous_href.clone())
                                    aria-disabled=(if prior_disabled { "true" } else { "false" })
                                    tabindex=(if prior_disabled { "-1" } else { "0" })
                                >
                                    <span class="arrow-glyph">"←"</span>
                                    <span class="arrow-glyph-label">"Previous"</span>
                                </a>
                                <a
                                    class="arrow-button next"
                                    aria-label=(format!("Next chart: {}", next_definition.title))
                                    href=(next_href)
                                    aria-disabled=(if next_disabled { "true" } else { "false" })
                                    tabindex=(if next_disabled { "-1" } else { "0" })
                                >
                                    <span class="arrow-glyph-label">"Next"</span>
                                    <span class="arrow-glyph">"→"</span>
                                </a>
                            </div>
                        </div>
                        <p class="chart-deck">(chart_deck)</p>
                        <div class="chart-shell">
                            (Unescaped::new_unchecked(chart_body))
                        </div>
                        <div class="chart-method">
                            (match chart {
                                Chart::DataCard => "Shows the fare media breakdown for the end date of your selected range. The proportion bar and payment boxes mirror the official BMRCL daily ridership page.",
                                Chart::Calendar => "Each cell shows one day. The hue tells you how that day's total compares with the other days you've selected — lighter means lower, darker means higher. Crossed cells are days BMRCL didn't publish.",
                                Chart::CommuteCasual => "Lines show the weekday pattern for two rider groups: people who pay by Smart Card or NCMC (closed-loop), and everyone else (QR + token + group). Averages skip days missing any component of the relevant group.",
                            })
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

pub async fn failure_view(cx: &Cx) -> Result {
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
