use chrono::{Datelike, FixedOffset, NaiveDate};
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    view::{Unescaped, view},
};

use crate::charts::{
    CHARTS, Chart, baseline_chart_markup, calendar_markup, data_card_markup, escape_attr,
    fare_mix_chart_markup, line_chart_markup, ncmc_share_chart_markup, qr_mix_chart_markup,
    smart_mix_chart_markup, waterfall_chart_markup, weekday_distribution_chart_markup,
    weekend_signature_chart_markup,
};
use crate::client::{CLIENT_SCRIPT, WRAPPER_CLOSE, WRAPPER_OPEN};
use crate::data::{
    Dataset, DateRange, SOURCE_PAGE_URL, choose_range, component_waterfall, fare_mix_series,
    ncmc_share_series, qr_mix_series, record_index, smart_mix_series, weekday_baseline_series,
    weekday_distribution_series, weekend_signature_series,
};
use crate::payload::{chart_payload_for, range_presets};
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

pub fn range_action_links(dataset: &Dataset, chart: Chart, current: &DateRange) -> String {
    range_presets(dataset)
        .iter()
        .map(|preset| {
            let range = preset.range;
            let disabled = current.start == range.start && current.end == range.end;
            format!(
                r#"<a class="text-button" href="/" data-preset="{preset}" data-start="{start}" data-end="{end}" data-chart="{chart}" aria-disabled="{disabled}" tabindex="{tabindex}">{label}</a>"#,
                preset = preset.slug,
                start = range.start,
                end = range.end,
                label = preset.label,
                chart = chart.slug(),
                disabled = disabled,
                tabindex = if disabled { "-1" } else { "0" },
            )
        })
        .collect()
}

pub fn chart_jump_markup(chart: Chart) -> String {
    let options = CHARTS
        .iter()
        .map(|definition| {
            let selected = if definition.chart == chart {
                " selected"
            } else {
                ""
            };
            let slug = escape_attr(definition.slug);
            let title = escape_attr(definition.title);
            format!(r#"<option value="{slug}"{selected}>{title}</option>"#)
        })
        .collect::<String>();
    format!(r#"<select class="chart-jump" aria-label="Jump to chart">{options}</select>"#)
}

const GITHUB_ICON: &str = r#"<svg role="img" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><title>GitHub</title><path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/></svg>"#;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub fn date_select_markup(
    id_prefix: &str,
    date: NaiveDate,
    min: NaiveDate,
    max: NaiveDate,
) -> String {
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
    let prev_chart = if chart_idx == 0 {
        chart
    } else {
        CHARTS[chart_idx - 1].chart
    };
    let next_chart = if chart_idx + 1 < CHARTS.len() {
        CHARTS[chart_idx + 1].chart
    } else {
        chart
    };
    let previous_chart_slug = prev_chart.slug();
    let next_chart_slug = next_chart.slug();
    let definition = chart.definition();
    let chart_title = definition.title;
    let chart_deck = definition.deck;
    let chart_body = match chart {
        Chart::DataCard => data_card_markup(&dataset, range),
        Chart::Calendar => calendar_markup(&dataset, range, &summary),
        Chart::CommuteCasual => line_chart_markup(&summary),
        Chart::WeekdayBaseline => {
            baseline_chart_markup(&weekday_baseline_series(&dataset, range), range)
        }
        Chart::ComponentWaterfall => {
            waterfall_chart_markup(component_waterfall(&dataset, range.end).as_ref())
        }
        Chart::FareMediaMix => fare_mix_chart_markup(&fare_mix_series(&dataset, range)),
        Chart::NcmcShare => ncmc_share_chart_markup(&ncmc_share_series(&dataset, range)),
        Chart::QrMix => qr_mix_chart_markup(&qr_mix_series(&dataset, range)),
        Chart::SmartMix => smart_mix_chart_markup(&smart_mix_series(&dataset, range)),
        Chart::WeekdayDistribution => {
            weekday_distribution_chart_markup(&weekday_distribution_series(&dataset, range))
        }
        Chart::WeekendSignatures => {
            weekend_signature_chart_markup(&weekend_signature_series(&dataset, range))
        }
    };
    let start_date_selects = date_select_markup(
        "start-date",
        range.start,
        dataset.min_date,
        dataset.max_date,
    );
    let end_date_selects =
        date_select_markup("end-date", range.end, dataset.min_date, dataset.max_date);
    let previous_definition = prev_chart.definition();
    let next_definition = next_chart.definition();
    let prior_disabled = chart_idx == 0;
    let next_disabled = chart_idx + 1 >= CHARTS.len();
    let ist = FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap();
    let refreshed_ist = chrono::DateTime::parse_from_rfc3339(&dataset.refreshed_at)
        .map(|dt| {
            dt.with_timezone(&ist)
                .format("%-d %b %Y, %-I:%M %p IST")
                .to_string()
        })
        .unwrap_or_else(|_| dataset.refreshed_at.clone());
    let data_note = format!(
        "{} – {} · {} rows · latest {} · retrieved {}",
        dataset.min_date.format("%-d %b %Y"),
        dataset.max_date.format("%-d %b %Y"),
        dataset.records.len(),
        dataset.max_date.format("%-d %b %Y"),
        refreshed_ist
    );
    let range_actions = range_action_links(&dataset, chart, &range);
    let chart_index_str = (chart.index() + 1).to_string();
    let chart_total_str = CHARTS.len().to_string();
    let available_dates = dataset
        .records
        .iter()
        .map(|record| record.date.to_string())
        .collect::<Vec<_>>()
        .join(",");
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
                        <p class="eyebrow">"PUBLIC DATA STORY · BENGALURU"</p>
                        <h1>"NammaMetro in Charts"</h1>
                        <p class="subtitle">
                            "A public view of Bengaluru Metro ridership"
                        </p>
                        <div class="source-links">
                            (Unescaped::new_unchecked(GITHUB_ICON))
                            <span class="source-prefix">"Source"</span>
                            <a
                                href="https://github.com/thecont1/metro-dash"
                                target="_blank"
                                rel="noreferrer"
                                aria-label="NammaMetro in Charts source code on GitHub"
                            >
                                "Code"
                            </a>
                            <span class="source-separator" aria-hidden="true">"|"</span>
                            <a
                                href=(SOURCE_PAGE_URL)
                                target="_blank"
                                rel="noreferrer"
                                aria-label="Namma Metro ridership source data on GitHub"
                            >
                                "Data"
                            </a>
                        </div>
                    </header>
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
                                    href="/"
                                    data-chart=(previous_chart_slug)
                                    aria-disabled=(if prior_disabled { "true" } else { "false" })
                                    tabindex=(if prior_disabled { "-1" } else { "0" })
                                >
                                    <span class="arrow-glyph">"←"</span>
                                    <span class="arrow-glyph-label">"Previous"</span>
                                </a>
                                (Unescaped::new_unchecked(chart_jump_markup(chart)))
                                <a
                                    class="arrow-button next"
                                    aria-label=(format!("Next chart: {}", next_definition.title))
                                    href="/"
                                    data-chart=(next_chart_slug)
                                    aria-disabled=(if next_disabled { "true" } else { "false" })
                                    tabindex=(if next_disabled { "-1" } else { "0" })
                                >
                                    <span class="arrow-glyph-label">"Next"</span>
                                    <span class="arrow-glyph">"→"</span>
                                </a>
                            </div>
                        </div>
                        <p class="chart-deck">(chart_deck)</p>
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
                            <div class="range-actions" role="group" aria-label="Quick date range filters">
                                (Unescaped::new_unchecked(range_actions))
                            </div>
                        </div>
                        <div class="chart-shell">
                            (Unescaped::new_unchecked(chart_body))
                        </div>
                        <div class="chart-method">
                            (definition.method)
                        </div>
                    </section>
                    <footer class="site-footer">
                        <div>
                            <p class="eyebrow">"DATA NOTE"</p>
                            <p>(data_note)</p>
                            <p class="compute-note" id="compute-note" hidden="hidden">
                                "Live data is unavailable right now; the charts are showing the build-time snapshot."
                            </p>
                            <p class="copyright">"© Mahesh Shantaram 2026"</p>
                        </div>
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
