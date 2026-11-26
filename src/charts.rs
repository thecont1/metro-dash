use std::collections::HashMap;
use std::fmt::Write as _;

use chrono::{Datelike, NaiveDate, Weekday};

use crate::data::{
    BaselineStatus, ComponentWaterfall, Dataset, DateRange, FareMixMonth, NcmcShareWeek,
    QrMixMonth, RangeSummary, RidershipRecord, SmartMixMonth, WeekdayBaselinePoint,
    WeekdayDistribution, WeekdayMetric,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Chart {
    DataCard,
    Calendar,
    CommuteCasual,
    WeekdayBaseline,
    ComponentWaterfall,
    FareMediaMix,
    NcmcShare,
    QrMix,
    SmartMix,
    WeekdayDistribution,
}

#[derive(Debug, Clone, Copy)]
pub struct ChartDefinition {
    pub chart: Chart,
    pub slug: &'static str,
    pub title: &'static str,
    pub deck: &'static str,
    pub method: &'static str,
}

pub static CHARTS: [ChartDefinition; 10] = [
    ChartDefinition {
        chart: Chart::DataCard,
        slug: "data-card",
        title: "Today's Ridership",
        deck: "A single day's ridership at a glance, broken down by fare media.",
        method: "Shows the fare media breakdown for the end date of your selected range. The proportion bar and payment boxes mirror the official BMRCL daily ridership page.",
    },
    ChartDefinition {
        chart: Chart::Calendar,
        slug: "calendar",
        title: "Daily Total Ridership",
        deck: "Each cell is one calendar day. Deeper the purple, higher the total.",
        method: "Each cell shows one day. The hue tells you how that day's total compares with the other days you've selected — lighter means lower, darker means higher. Crossed cells are days BMRCL didn't publish.",
    },
    ChartDefinition {
        chart: Chart::CommuteCasual,
        slug: "commute-casual",
        title: "Commute vs Casual by Weekday",
        deck: "Average journeys by day of week, using the dates you've selected.",
        method: "Lines show the weekday pattern for two rider groups: people who pay by Smart Card or NCMC (closed-loop), and everyone else (QR + token + group). Averages skip days missing any component of the relevant group.",
    },
    ChartDefinition {
        chart: Chart::WeekdayBaseline,
        slug: "weekday-baseline",
        title: "Unusual for its Weekday?",
        deck: "Daily totals derived from fare media, compared with nearby reports from the same weekday.",
        method: "Each point is a daily total derived from published fare-media components. The band is the interquartile range of up to 6 earlier and 6 later published reports from the same weekday; the line is their median. At least 4 nearby reports are required. The current day is never included in its own baseline, and missing reports are not treated as zero.",
    },
    ChartDefinition {
        chart: Chart::ComponentWaterfall,
        slug: "component-waterfall",
        title: "What Accounts for the Difference?",
        deck: "Fare-media contributions to the selected day's difference from its same-weekday reference.",
        method: "Uses the end date and the published parent totals. Each reference is the median of up to 6 earlier and 6 later same-weekday reports with complete components; provider sums are not substituted because QR splits can diverge. Bars sum exactly to the total difference — arithmetic decomposition, not causal attribution. The axis is zoomed to make movements visible.",
    },
    ChartDefinition {
        chart: Chart::FareMediaMix,
        slug: "fare-media-mix",
        title: "How is the Fare-Media Mix Shifting?",
        deck: "Monthly share mix, with the observed absolute total shown separately underneath.",
        method: "Each month uses only published days with complete parent components. Upper bars are shares of the observed total; lower bars are the observed absolute total for those same complete days, not full-month totals. Missing reports and incomplete components are never zero-filled. Shares are aggregate fare-media composition, not evidence that individual riders switched payment method.",
    },
    ChartDefinition {
        chart: Chart::NcmcShare,
        slug: "ncmc-share",
        title: "How is NCMC’s Share Changing?",
        deck: "Weekly NCMC share of Smart Cards + NCMC, with the NCMC count shown separately underneath.",
        method: "Weeks aggregate only days where both Smart Card and NCMC are present, so the share and count use the same evidence base. Complete/published day counts make uneven week coverage visible. Missing values are never zero-filled. This is fare-media arithmetic, not proof that Smart Card users converted to NCMC.",
    },
    ChartDefinition {
        chart: Chart::QrMix,
        slug: "qr-channel-mix",
        title: "Which QR Channels Make Up QR Use?",
        deck: "Monthly provider-level shares, with parent Total QR shown separately underneath.",
        method: "Months aggregate days where all four QR provider fields are present. Shares use the provider sum because provider splits do not always reconcile with parent Total QR; the lower strip shows parent Total QR for the same complete days. Zero is treated as reported zero, while missing fields exclude the day. This describes aggregate channel mix, not individual app use.",
    },
    ChartDefinition {
        chart: Chart::SmartMix,
        slug: "smart-card-mix",
        title: "What is Inside Smart Cards?",
        deck: "Monthly Smart Card subtype shares, with parent Smart Cards shown separately underneath.",
        method: "Months aggregate days where all four Smart Card subtype fields are present. Shares use the subtype sum; the lower strip shows the published parent Smart Card total for the same complete days. Missing fields exclude the day rather than becoming zero. These are fare-media counts, not unique people or trip purposes.",
    },
    ChartDefinition {
        chart: Chart::WeekdayDistribution,
        slug: "weekday-distributions",
        title: "How Predictable is Each Weekday?",
        deck: "Observed daily-total distributions for every weekday in the selected range.",
        method: "For each weekday, the box spans the interquartile range, the centre line is the median, and whiskers span the full observed range. Only published derived totals count; missing reports are omitted rather than zero-filled. A wide range is descriptive variability, not evidence of crowding or capacity pressure.",
    },
];

impl Chart {
    pub fn slug(self) -> &'static str {
        self.definition().slug
    }
    pub fn definition(self) -> &'static ChartDefinition {
        CHARTS
            .iter()
            .find(|definition| definition.chart == self)
            .expect("every chart variant is registered")
    }
    pub fn index(self) -> usize {
        CHARTS
            .iter()
            .position(|definition| definition.chart == self)
            .expect("every chart variant is registered")
    }
}

pub fn format_range(range: DateRange) -> String {
    format!(
        "{} – {}",
        range.start.format("%-d %b %Y"),
        range.end.format("%-d %b %Y")
    )
}

pub fn format_number_full(value: f64) -> String {
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

pub fn compact_number(value: f64) -> String {
    if value.abs() >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("{:.0}k", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

pub fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn weekday_name(day: Weekday) -> &'static str {
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

pub fn band_label(value: f64, summary: &RangeSummary) -> Option<&'static str> {
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

pub fn band_index(label: &str) -> Option<usize> {
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

pub fn calendar_markup(dataset: &Dataset, range: DateRange, summary: &RangeSummary) -> String {
    let map: HashMap<NaiveDate, &RidershipRecord> = dataset
        .records
        .iter()
        .filter(|record| record.date >= range.start && record.date <= range.end)
        .map(|record| (record.date, record))
        .collect();

    let mut col = 0usize;
    let mut prev_row: Option<usize> = None;
    let mut last_month = range.start.month();
    let mut date = range.start;
    let mut positions: Vec<(NaiveDate, usize, usize)> = Vec::new();
    while date <= range.end {
        let row = date.weekday().num_days_from_monday() as usize;
        if date != range.start {
            if date.month() != last_month {
                col += 1;
                last_month = date.month();
            } else if prev_row == Some(6) {
                col += 1;
            }
        }
        positions.push((date, col, row));
        prev_row = Some(row);
        date += chrono::Duration::days(1);
    }
    let max_col = positions.iter().map(|(_, c, _)| *c).max().unwrap_or(0);

    let mut month_spans: Vec<(String, usize, usize)> = Vec::new();
    let mut current_span: Option<(String, usize, usize)> = None;
    for (d, c, _) in &positions {
        let key = d.format("%b '%y").to_string();
        let is_new = current_span
            .as_ref()
            .map(|(label, _, _)| label != &key)
            .unwrap_or(true);
        if is_new {
            if let Some(span) = current_span.take() {
                month_spans.push(span);
            }
            current_span = Some((key, *c, *c));
        } else if let Some((_, _min, max)) = current_span.as_mut() {
            *max = (*max).max(*c);
        }
    }
    if let Some(span) = current_span {
        month_spans.push(span);
    }

    let mut html = format!(
        r#"<div class="calendar-wrap"><div class="calendar-grid" role="img" aria-label="Daily ridership calendar heatmap"><div class="weekday-labels"><span>Mon</span><span>Tue</span><span>Wed</span><span>Thu</span><span>Fri</span><span>Sat</span><span>Sun</span></div><div class="calendar-canvas" style="--max-col:{}">"#,
        max_col + 1
    );

    for (label, min_c, max_c) in &month_spans {
        let span = max_c - min_c + 1;
        html.push_str(&format!(
            r#"<span class="month-label" style="grid-column:{} / span {}">{label}</span>"#,
            min_c + 1,
            span
        ));
    }

    for (date, c, row) in &positions {
        let record = map.get(date).copied();
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
        html.push_str(&format!(
            r#"<button class="{classes}" title="{safe}" aria-label="{safe}" data-tooltip="{safe}" data-date="{date}" style="grid-column:{};grid-row:{}"><span class="sr-only">{safe}</span></button>"#,
            c + 1,
            row + 2
        ));
    }

    html.push_str("</div></div></div>");
    html.push_str(&legend_markup(summary));
    html.push_str(&calendar_table(dataset, range, summary));
    html
}

pub fn legend_markup(summary: &RangeSummary) -> String {
    let observed = match (summary.total_min, summary.total_max) {
        (Some(min), Some(max)) if (max - min).abs() > f64::EPSILON => format!(
            "Observed: {} – {}",
            compact_number(min),
            compact_number(max)
        ),
        _ => "Observed range unavailable for this selection".to_string(),
    };
    let buckets_note = match summary.valid_total_count {
        0 => "Buckets unavailable: no published totals in this range.".to_string(),
        1..=9 => format!(
            "Buckets are approximate ({num} day{plural} in range; tint carries limited signal).",
            num = summary.valid_total_count,
            plural = if summary.valid_total_count == 1 {
                ""
            } else {
                "s"
            }
        ),
        _ => "Each cell's shade maps to its percentile of daily ridership within your selection."
            .to_string(),
    };
    let pct_map: std::collections::HashMap<u8, f64> =
        summary.percentiles.iter().map(|(p, v)| (*p, *v)).collect();
    let pct_label = |p: u8| -> String {
        pct_map
            .get(&p)
            .map(|v| compact_number(*v))
            .unwrap_or_default()
    };
    let bands = [
        ("< p2", "band-0", format!("< p2  (< {})", pct_label(2))),
        (
            "p2 – p5",
            "band-1",
            format!("p2 – p5  ({} – {})", pct_label(2), pct_label(5)),
        ),
        (
            "p5 – p10",
            "band-2",
            format!("p5 – p10  ({} – {})", pct_label(5), pct_label(10)),
        ),
        (
            "p10 – p25",
            "band-3",
            format!("p10 – p25  ({} – {})", pct_label(10), pct_label(25)),
        ),
        (
            "p25 – p50",
            "band-4",
            format!("p25 – p50  ({} – {})", pct_label(25), pct_label(50)),
        ),
        (
            "p50 – p75",
            "band-5",
            format!("p50 – p75  ({} – {})", pct_label(50), pct_label(75)),
        ),
        (
            "p75 – p90",
            "band-6",
            format!("p75 – p90  ({} – {})", pct_label(75), pct_label(90)),
        ),
        (
            "p90 – p95",
            "band-7",
            format!("p90 – p95  ({} – {})", pct_label(90), pct_label(95)),
        ),
        (
            "p95 – p98",
            "band-8",
            format!("p95 – p98  ({} – {})", pct_label(95), pct_label(98)),
        ),
        ("> p98", "band-9", format!("> p98  (> {})", pct_label(98))),
    ];
    let swatches_html: String = bands
        .iter()
        .map(|(_, cls, title)| {
            format!(r#"<span class="legend-swatch {cls}" title="{title}"></span>"#)
        })
        .collect();
    let percentile_labels = ["p2", "p5", "p10", "p25", "p50", "p75", "p90", "p95", "p98"];
    let ticks_html: String = percentile_labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            format!(
                r#"<span class="legend-tick" style="left:{}%">{label}</span>"#,
                (i + 1) * 10
            )
        })
        .collect();
    format!(
        r#"<div class="legend" aria-label="Color scale"><div class="legend-gradient-wrap"><div class="legend-swatches" aria-hidden="true">{swatches_html}</div><div class="legend-ticks" aria-hidden="true">{ticks_html}</div></div><p class="legend-caption">{observed}</p><p class="legend-meta">{buckets_note}</p><p class="legend-meta legend-missing-note">Crossed cells = days BMRCL didn't publish a total for.</p></div>"#
    )
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

pub fn line_chart_markup(summary: &RangeSummary) -> String {
    const WIDTH: f64 = 900.0;
    const HEIGHT: f64 = 410.0;
    const LEFT: f64 = 66.0;
    const RIGHT: f64 = 10.0;
    const TOP: f64 = 24.0;
    const BOTTOM: f64 = 66.0;
    const X_INSET: f64 = 18.0;
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
    let x = |index: usize| LEFT + X_INSET + index as f64 * (plot_width - X_INSET * 2.0) / 6.0;
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
            r#"<path class="series {class}" d="{path}"></path>"#
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
                r#"<circle class="point-hit" cx="{xx:.1}" cy="{yy:.1}" r="12" fill="transparent" tabindex="0" role="img" aria-label="{safe}" data-tooltip="{safe}"></circle><circle class="point {class}" cx="{xx:.1}" cy="{yy:.1}" r="6" aria-hidden="true"></circle><text class="point-label {class}" x="{xx:.1}" y="{:.1}" text-anchor="middle" aria-hidden="true">{compact}</text>"#,
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

fn baseline_delta_pct(point: &WeekdayBaselinePoint) -> Option<f64> {
    point
        .median
        .filter(|median| *median != 0.0)
        .map(|median| (point.total - median) / median * 100.0)
}

fn baseline_point_label(point: &WeekdayBaselinePoint) -> String {
    let date = point.date.format("%A, %-d %B %Y");
    let total = format_number_full(point.total);
    match (point.median, point.lower_quartile, point.upper_quartile) {
        (Some(median), Some(lower), Some(upper)) => {
            let delta = baseline_delta_pct(point)
                .map(|value| format!("{value:+.1}%"))
                .unwrap_or_else(|| "—".to_string());
            format!(
                "{date}: {total} total journeys, {} the same-weekday baseline. Median {}, interquartile band {}–{}, delta {delta}, baseline n {}.",
                point.status.label().to_lowercase(),
                format_number_full(median),
                format_number_full(lower),
                format_number_full(upper),
                point.baseline_n
            )
        }
        _ => format!(
            "{date}: {total} total journeys. Baseline unavailable; only {} prior same-weekday reports.",
            point.baseline_n
        ),
    }
}

pub fn baseline_insight(points: &[WeekdayBaselinePoint]) -> String {
    let high = points
        .iter()
        .filter(|point| point.status == BaselineStatus::High)
        .count();
    let low = points
        .iter()
        .filter(|point| point.status == BaselineStatus::Low)
        .count();
    let insufficient = points
        .iter()
        .filter(|point| point.status == BaselineStatus::Insufficient)
        .count();
    let mut parts = Vec::new();
    if high > 0 {
        parts.push(format!("{high} above"));
    }
    if low > 0 {
        parts.push(format!("{low} below"));
    }
    if parts.is_empty() {
        "No published day in this range sits outside its weekday baseline band.".to_string()
    } else {
        let mut text = format!(
            "Reported days outside their weekday baseline band: {}",
            parts.join(" and ")
        );
        if insufficient > 0 {
            text.push_str(&format!("; {insufficient} lack enough nearby reports"));
        }
        text.push('.');
        text
    }
}

pub fn baseline_chart_markup(points: &[WeekdayBaselinePoint], range: DateRange) -> String {
    if points.is_empty() {
        return r#"<div class="baseline-wrap"><p class="baseline-empty">No published totals are available in this range.</p></div>"#.to_string();
    }
    const WIDTH: f64 = 1100.0;
    const HEIGHT: f64 = 360.0;
    const LEFT: f64 = 70.0;
    const RIGHT: f64 = 34.0;
    const TOP: f64 = 36.0;
    const BOTTOM: f64 = 52.0;
    let plot_width = WIDTH - LEFT - RIGHT;
    let plot_height = HEIGHT - TOP - BOTTOM;
    let span_days = (range.end - range.start).num_days().max(1) as f64;
    let x =
        |date: NaiveDate| LEFT + ((date - range.start).num_days() as f64 / span_days) * plot_width;
    let min_value = points
        .iter()
        .flat_map(|point| [Some(point.total), point.lower_quartile])
        .flatten()
        .reduce(f64::min)
        .unwrap_or(0.0);
    let max_value = points
        .iter()
        .flat_map(|point| [Some(point.total), point.upper_quartile])
        .flatten()
        .reduce(f64::max)
        .unwrap_or(1.0)
        .max(1.0);
    let spread = (max_value - min_value).max(1.0);
    let y_min = (min_value - spread * 0.08).max(0.0);
    let y_max = max_value + spread * 0.08;
    let y = |value: f64| TOP + plot_height - (value - y_min) / (y_max - y_min) * plot_height;
    let mut svg = format!(
        r#"<div class="baseline-wrap"><svg class="baseline-chart" viewBox="0 0 {WIDTH} {HEIGHT}" role="img" aria-label="Daily ridership compared with rolling same-weekday baseline"><g class="grid">"#
    );
    for tick in 0..=4 {
        let value = y_min + (y_max - y_min) * tick as f64 / 4.0;
        let yy = y(value);
        svg.push_str(&format!(
            r#"<line x1="{LEFT}" x2="{}" y1="{yy:.1}" y2="{yy:.1}"></line><text x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            WIDTH - RIGHT,
            LEFT - 10.0,
            yy + 4.0,
            compact_number(value)
        ));
    }
    for tick in 0..=5 {
        let offset = (span_days * tick as f64 / 5.0).round() as i64;
        let date = range.start + chrono::Duration::days(offset);
        let xx = x(date);
        svg.push_str(&format!(
            r#"<text class="x-label" x="{xx:.1}" y="{}" text-anchor="middle">{}</text>"#,
            HEIGHT - 26.0,
            date.format("%d %b '%y")
        ));
    }

    let band_points: Vec<_> = points
        .iter()
        .filter(|point| point.lower_quartile.is_some() && point.upper_quartile.is_some())
        .collect();
    if band_points.len() >= 2 {
        let mut path = String::new();
        for (index, point) in band_points.iter().enumerate() {
            let command = if index == 0 { 'M' } else { 'L' };
            write!(
                path,
                "{command} {:.1} {:.1} ",
                x(point.date),
                y(point.upper_quartile.expect("band point has upper quartile"))
            )
            .unwrap();
        }
        for point in band_points.iter().rev() {
            write!(
                path,
                "L {:.1} {:.1} ",
                x(point.date),
                y(point.lower_quartile.expect("band point has lower quartile"))
            )
            .unwrap();
        }
        svg.push_str(&format!(
            r#"<path class="baseline-band" d="{path}Z"></path>"#
        ));
    }

    let median_points: Vec<_> = points
        .iter()
        .filter(|point| point.median.is_some())
        .collect();
    if median_points.len() >= 2 {
        let mut path = String::new();
        for (index, point) in median_points.iter().enumerate() {
            let command = if index == 0 { 'M' } else { 'L' };
            write!(
                path,
                "{command} {:.1} {:.1} ",
                x(point.date),
                y(point.median.expect("median point has median"))
            )
            .unwrap();
        }
        svg.push_str(&format!(
            r#"<path class="baseline-median" d="{path}" fill="none"></path>"#
        ));
    }

    if points.len() >= 2 {
        let mut path = String::new();
        for (index, point) in points.iter().enumerate() {
            let gap = index
                .checked_sub(1)
                .map(|previous| (point.date - points[previous].date).num_days())
                .unwrap_or(0);
            let command = if index == 0 || gap > 7 { 'M' } else { 'L' };
            write!(
                path,
                "{command} {:.1} {:.1} ",
                x(point.date),
                y(point.total)
            )
            .unwrap();
        }
        svg.push_str(&format!(
            r#"<path class="baseline-actual" d="{path}" fill="none"></path>"#
        ));
    }

    for point in points {
        let label = baseline_point_label(point);
        let safe = escape_attr(&label);
        let xx = x(point.date);
        let yy = y(point.total);
        let radius = match point.status {
            BaselineStatus::High | BaselineStatus::Low => 5.5,
            BaselineStatus::Within => 4.0,
            BaselineStatus::Insufficient => 3.5,
        };
        svg.push_str(&format!(
            r#"<circle class="baseline-point-hit" cx="{xx:.1}" cy="{yy:.1}" r="12" fill="transparent" tabindex="0" role="img" aria-label="{safe}" data-tooltip="{safe}"></circle><circle class="baseline-point {}" cx="{xx:.1}" cy="{yy:.1}" r="{radius}" aria-hidden="true"></circle>"#,
            point.status.as_str()
        ));
    }
    svg.push_str("</g></svg>");
    svg.push_str(
        r#"<div class="baseline-legend" aria-label="Baseline chart legend"><span><i class="legend-swatch-band"></i>IQR band</span><span><i class="legend-line-median"></i>Median</span><span><i class="legend-dot within"></i>Within</span><span><i class="legend-dot high"></i>Above</span><span><i class="legend-dot low"></i>Below</span></div>"#,
    );
    let insight = baseline_insight(points);
    if !insight.is_empty() {
        svg.push_str(&format!(r#"<p class="insight">{insight}</p>"#));
    }
    svg.push_str(&baseline_table(points));
    svg.push_str("</div>");
    svg
}

fn signed_full(value: f64) -> String {
    if value > 0.0 {
        format!("+{}", format_number_full(value))
    } else {
        format_number_full(value)
    }
}

pub fn smart_mix_insight(months: &[SmartMixMonth]) -> String {
    let usable: Vec<&SmartMixMonth> = months.iter().filter(|month| month.has_mix()).collect();
    if usable.len() < 2 {
        return "Not enough complete months in the selected range to compare Smart Card subtypes."
            .to_string();
    }
    let first = usable.first().expect("nonempty");
    let last = usable.last().expect("nonempty");
    let share =
        |month: &SmartMixMonth, index: usize| month.subtypes[index] / month.subtype_total * 100.0;
    format!(
        "Stored Value {:.1}% → {:.1}%; One Day Pass {:.1}% → {:.1}% of Smart Card subtypes.",
        share(first, 0),
        share(last, 0),
        share(first, 1),
        share(last, 1)
    )
}

pub fn smart_mix_chart_markup(months: &[SmartMixMonth]) -> String {
    let mut html = String::from(r#"<div class="smart-ssr-wrap"><div class="smart-ssr-months">"#);
    for month in months {
        html.push_str(&format!(
            r#"<div class="smart-ssr-month"><span class="smart-ssr-label">{}</span>"#,
            month.month.format("%b ’%y")
        ));
        if month.has_mix() {
            html.push_str(r#"<span class="smart-ssr-bar">"#);
            for (index, (key, label)) in crate::data::SMART_SUBTYPE_CATEGORIES.iter().enumerate() {
                let share = month.subtypes[index] / month.subtype_total * 100.0;
                html.push_str(&format!(
                    r#"<i class="smart-segment smart-{key}" style="width:{share:.3}%" title="{}: {:.1}%"></i>"#,
                    label, share
                ));
            }
            html.push_str(r#"</span>"#);
        } else {
            html.push_str(r#"<span class="smart-ssr-gap">No complete subtype days</span>"#);
        }
        html.push_str(&format!(
            r#"<span class="smart-ssr-meta">{}/{}d · Smart {}</span></div>"#,
            month.complete_days,
            month.published_days,
            if month.has_mix() {
                format_number_full(month.parent_smart_total)
            } else {
                "—".into()
            }
        ));
    }
    html.push_str(r#"</div><div class="mix-legend"><span><i class="mix-key smart-stored-value"></i>Stored Value</span><span><i class="mix-key smart-one-day-pass"></i>One Day Pass</span><span><i class="mix-key smart-three-day-pass"></i>Three Day Pass</span><span><i class="mix-key smart-five-day-pass"></i>Five Day Pass</span></div>"#);
    html.push_str(&format!(
        r#"<p class="insight">{}</p>"#,
        smart_mix_insight(months)
    ));
    html.push_str(&smart_mix_table(months));
    html.push_str("</div>");
    html
}

fn smart_mix_table(months: &[SmartMixMonth]) -> String {
    let mut table = String::from(
        r#"<details class="accessible-data"><summary>View Smart Card subtype data table</summary><div class="table-scroll"><table><caption>Monthly Smart Card subtype counts and shares</caption><thead><tr><th scope="col">Month</th><th scope="col">Complete days</th><th scope="col">Parent Smart Cards</th><th scope="col">Stored Value</th><th scope="col">One Day Pass</th><th scope="col">Three Day Pass</th><th scope="col">Five Day Pass</th></tr></thead><tbody>"#,
    );
    for month in months {
        let share = |index: usize| {
            if month.has_mix() {
                format!(
                    "{:.1}%",
                    month.subtypes[index] / month.subtype_total * 100.0
                )
            } else {
                "—".into()
            }
        };
        table.push_str(&format!(
            "<tr><th scope=\"row\">{}</th><td>{}/{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            month.month.format("%b %Y"), month.complete_days, month.published_days,
            if month.has_mix() { format_number_full(month.parent_smart_total) } else { "—".into() },
            share(0), share(1), share(2), share(3)
        ));
    }
    table.push_str("</tbody></table></div></details>");
    table
}

pub fn weekday_distribution_insight(days: &[WeekdayDistribution]) -> String {
    let usable: Vec<&WeekdayDistribution> = days
        .iter()
        .filter(|day| day.n >= 2 && day.lower_quartile.is_some() && day.upper_quartile.is_some())
        .collect();
    if usable.is_empty() {
        return "Not enough observations in the selected range to compare weekday distributions."
            .to_string();
    }
    let width = |day: &WeekdayDistribution| {
        day.upper_quartile.unwrap_or(0.0) - day.lower_quartile.unwrap_or(0.0)
    };
    let widest = usable
        .iter()
        .copied()
        .max_by(|a, b| width(a).total_cmp(&width(b)))
        .expect("nonempty");
    let narrowest = usable
        .iter()
        .copied()
        .min_by(|a, b| width(a).total_cmp(&width(b)))
        .expect("nonempty");
    format!(
        "{} has the widest interquartile range ({}–{}); {} is the tightest ({}–{}).",
        weekday_name(widest.weekday),
        format_number_full(widest.lower_quartile.unwrap_or(0.0)),
        format_number_full(widest.upper_quartile.unwrap_or(0.0)),
        weekday_name(narrowest.weekday),
        format_number_full(narrowest.lower_quartile.unwrap_or(0.0)),
        format_number_full(narrowest.upper_quartile.unwrap_or(0.0))
    )
}

fn distribution_day_label(day: &WeekdayDistribution) -> String {
    if day.n == 0 {
        return format!(
            "{}: no published totals in the selected range.",
            weekday_name(day.weekday)
        );
    }
    format!(
        "{}: median {}, interquartile range {}–{}, full range {}–{}, n {}.",
        weekday_name(day.weekday),
        day.median
            .map(format_number_full)
            .unwrap_or_else(|| "—".into()),
        day.lower_quartile
            .map(format_number_full)
            .unwrap_or_else(|| "—".into()),
        day.upper_quartile
            .map(format_number_full)
            .unwrap_or_else(|| "—".into()),
        day.minimum
            .map(format_number_full)
            .unwrap_or_else(|| "—".into()),
        day.maximum
            .map(format_number_full)
            .unwrap_or_else(|| "—".into()),
        day.n
    )
}

pub fn weekday_distribution_chart_markup(days: &[WeekdayDistribution]) -> String {
    const WIDTH: f64 = 900.0;
    const HEIGHT: f64 = 380.0;
    const LEFT: f64 = 66.0;
    const RIGHT: f64 = 24.0;
    const TOP: f64 = 28.0;
    const BOTTOM: f64 = 64.0;
    let max_value = days
        .iter()
        .filter_map(|day| day.maximum)
        .reduce(f64::max)
        .unwrap_or(1.0)
        .max(1.0);
    let plot_width = WIDTH - LEFT - RIGHT;
    let plot_height = HEIGHT - TOP - BOTTOM;
    let x = |index: usize| LEFT + plot_width * (index as f64 + 0.5) / 7.0;
    let y = |value: f64| TOP + plot_height - value / max_value * plot_height;
    let mut svg = format!(
        r#"<div class="dist-ssr-wrap"><svg class="dist-chart" viewBox="0 0 {WIDTH} {HEIGHT}" role="img" aria-label="Weekday distributions of observed daily totals"><g class="grid">"#
    );
    for tick in 0..=4 {
        let value = max_value * tick as f64 / 4.0;
        let yy = y(value);
        svg.push_str(&format!(
            r#"<line x1="{LEFT}" x2="{}" y1="{yy:.1}" y2="{yy:.1}"></line><text x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            WIDTH - RIGHT, LEFT - 10.0, yy + 4.0, compact_number(value)
        ));
    }
    for (index, day) in days.iter().enumerate() {
        let xx = x(index);
        svg.push_str(&format!(
            r#"<text class="x-label" x="{xx:.1}" y="{}" text-anchor="middle">{}</text>"#,
            HEIGHT - 25.0,
            weekday_name(day.weekday).get(..3).unwrap_or("")
        ));
        if day.n == 0 {
            svg.push_str(&format!(
                r#"<text class="dist-empty" x="{xx:.1}" y="{:.1}" text-anchor="middle">n 0</text>"#,
                TOP + plot_height / 2.0
            ));
            continue;
        }
        let label = escape_attr(&distribution_day_label(day));
        let min_y = y(day.minimum.unwrap_or(0.0));
        let max_y = y(day.maximum.unwrap_or(0.0));
        let q1_y = y(day.lower_quartile.unwrap_or(0.0));
        let q3_y = y(day.upper_quartile.unwrap_or(0.0));
        let med_y = y(day.median.unwrap_or(0.0));
        svg.push_str(&format!(
            r#"<g class="dist-day" tabindex="0" role="img" aria-label="{label}" data-tooltip="{label}"><line class="dist-whisker" x1="{xx:.1}" x2="{xx:.1}" y1="{max_y:.1}" y2="{min_y:.1}"></line><line class="dist-cap" x1="{:.1}" x2="{:.1}" y1="{max_y:.1}" y2="{max_y:.1}"></line><line class="dist-cap" x1="{:.1}" x2="{:.1}" y1="{min_y:.1}" y2="{min_y:.1}"></line><rect class="dist-box" x="{:.1}" y="{q3_y:.1}" width="42" height="{:.1}"></rect><line class="dist-median" x1="{:.1}" x2="{:.1}" y1="{med_y:.1}" y2="{med_y:.1}"></line><text class="dist-n" x="{xx:.1}" y="{}" text-anchor="middle">n {}</text></g>"#,
            xx - 18.0, xx + 18.0, xx - 18.0, xx + 18.0,
            xx - 21.0, (q1_y - q3_y).max(1.0), xx - 21.0, xx + 21.0,
            HEIGHT - 8.0, day.n
        ));
    }
    svg.push_str("</g></svg>");
    svg.push_str(r#"<div class="dist-legend" aria-label="Distribution legend"><span><i class="dist-key-box"></i>Interquartile range</span><span><i class="dist-key-median"></i>Median</span><span><i class="dist-key-whisker"></i>Observed range</span></div>"#);
    svg.push_str(&format!(
        r#"<p class="insight">{}</p>"#,
        weekday_distribution_insight(days)
    ));
    svg.push_str(&weekday_distribution_table(days));
    svg.push_str("</div>");
    svg
}

fn weekday_distribution_table(days: &[WeekdayDistribution]) -> String {
    let mut table = String::from(
        r#"<details class="accessible-data"><summary>View weekday distribution data table</summary><div class="table-scroll"><table><caption>Observed total-ridership distributions by weekday</caption><thead><tr><th scope="col">Weekday</th><th scope="col">n</th><th scope="col">Minimum</th><th scope="col">Q1</th><th scope="col">Median</th><th scope="col">Q3</th><th scope="col">Maximum</th></tr></thead><tbody>"#,
    );
    for day in days {
        let value =
            |value: Option<f64>| value.map(format_number_full).unwrap_or_else(|| "—".into());
        table.push_str(&format!(
            "<tr><th scope=\"row\">{}</th><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            weekday_name(day.weekday), day.n, value(day.minimum), value(day.lower_quartile), value(day.median), value(day.upper_quartile), value(day.maximum)
        ));
    }
    table.push_str("</tbody></table></div></details>");
    table
}

pub fn qr_mix_insight(months: &[QrMixMonth]) -> String {
    let usable: Vec<&QrMixMonth> = months.iter().filter(|month| month.has_mix()).collect();
    if usable.len() < 2 {
        return "Not enough complete months in the selected range to compare QR channels."
            .to_string();
    }
    let first = usable.first().expect("nonempty");
    let last = usable.last().expect("nonempty");
    let share =
        |month: &QrMixMonth, index: usize| month.providers[index] / month.provider_total * 100.0;
    format!(
        "Namma Metro {:.1}% → {:.1}%; ONDC {:.1}% → {:.1}% of provider-sum QR.",
        share(first, 0),
        share(last, 0),
        share(first, 3),
        share(last, 3)
    )
}

pub fn qr_mix_chart_markup(months: &[QrMixMonth]) -> String {
    let mut html = String::from(r#"<div class="qr-ssr-wrap"><div class="qr-ssr-months">"#);
    for month in months {
        html.push_str(&format!(
            r#"<div class="qr-ssr-month"><span class="qr-ssr-label">{}</span>"#,
            month.month.format("%b ’%y")
        ));
        if month.has_mix() {
            html.push_str(r#"<span class="qr-ssr-bar">"#);
            for (index, (key, label)) in crate::data::QR_PROVIDER_CATEGORIES.iter().enumerate() {
                let share = month.providers[index] / month.provider_total * 100.0;
                html.push_str(&format!(
                    r#"<i class="qr-segment qr-{key}" style="width:{share:.3}%" title="{}: {:.1}%"></i>"#,
                    label, share
                ));
            }
            html.push_str(r#"</span>"#);
        } else {
            html.push_str(r#"<span class="qr-ssr-gap">No complete provider days</span>"#);
        }
        html.push_str(&format!(
            r#"<span class="qr-ssr-meta">{}/{}d · QR {}</span></div>"#,
            month.complete_days,
            month.published_days,
            if month.has_mix() {
                format_number_full(month.parent_qr_total)
            } else {
                "—".into()
            }
        ));
    }
    html.push_str(r#"</div><div class="mix-legend"><span><i class="mix-key qr-qr-namma"></i>Namma Metro</span><span><i class="mix-key qr-qr-whatsapp"></i>WhatsApp</span><span><i class="mix-key qr-qr-paytm"></i>Paytm</span><span><i class="mix-key qr-qr-ondc"></i>ONDC</span></div>"#);
    html.push_str(&format!(
        r#"<p class="insight">{}</p>"#,
        qr_mix_insight(months)
    ));
    html.push_str(&qr_mix_table(months));
    html.push_str("</div>");
    html
}

fn qr_mix_table(months: &[QrMixMonth]) -> String {
    let mut table = String::from(
        r#"<details class="accessible-data"><summary>View QR channel data table</summary><div class="table-scroll"><table><caption>Monthly QR provider counts and shares</caption><thead><tr><th scope="col">Month</th><th scope="col">Complete days</th><th scope="col">Parent QR</th><th scope="col">Namma Metro</th><th scope="col">WhatsApp</th><th scope="col">Paytm</th><th scope="col">ONDC</th></tr></thead><tbody>"#,
    );
    for month in months {
        let share = |index: usize| {
            if month.has_mix() {
                format!(
                    "{:.1}%",
                    month.providers[index] / month.provider_total * 100.0
                )
            } else {
                "—".into()
            }
        };
        table.push_str(&format!(
            "<tr><th scope=\"row\">{}</th><td>{}/{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            month.month.format("%b %Y"), month.complete_days, month.published_days,
            if month.has_mix() { format_number_full(month.parent_qr_total) } else { "—".into() },
            share(0), share(1), share(2), share(3)
        ));
    }
    table.push_str("</tbody></table></div></details>");
    table
}

pub fn ncmc_share_insight(weeks: &[NcmcShareWeek]) -> String {
    let usable: Vec<&NcmcShareWeek> = weeks.iter().filter(|week| week.share().is_some()).collect();
    if usable.len() < 2 {
        return "Not enough complete weeks in the selected range to compare NCMC share."
            .to_string();
    }
    let first = usable.first().expect("nonempty");
    let last = usable.last().expect("nonempty");
    format!(
        "NCMC share of Smart Card + NCMC: {:.1}% → {:.1}% across complete observed days.",
        first.share().unwrap_or(0.0) * 100.0,
        last.share().unwrap_or(0.0) * 100.0
    )
}

pub fn ncmc_share_chart_markup(weeks: &[NcmcShareWeek]) -> String {
    if weeks.is_empty() {
        return r#"<div class="ncmc-wrap"><p class="ncmc-empty">No weeks are available in the selected range.</p></div>"#.to_string();
    }
    const WIDTH: f64 = 1100.0;
    const HEIGHT: f64 = 400.0;
    const LEFT: f64 = 48.0;
    const RIGHT: f64 = 22.0;
    const TOP: f64 = 42.0;
    const SHARE_H: f64 = 178.0;
    const GAP: f64 = 34.0;
    const TOTAL_H: f64 = 58.0;
    const LABEL_Y: f64 = 366.0;
    let plot_width = WIDTH - LEFT - RIGHT;
    let step = plot_width / weeks.len().max(1) as f64;
    let bar_width = (step * 0.52).clamp(10.0, 42.0);
    let max_ncmc = weeks
        .iter()
        .map(|week| week.ncmc)
        .reduce(f64::max)
        .unwrap_or(1.0)
        .max(1.0);
    let total_base = TOP + SHARE_H + GAP + TOTAL_H;
    let x = |index: usize| LEFT + step * (index as f64 + 0.5);
    let share_y = |share: f64| TOP + SHARE_H - share * SHARE_H;
    let mut svg = format!(
        r#"<div class="ncmc-wrap"><svg class="ncmc-chart" viewBox="0 0 {WIDTH} {HEIGHT}" role="img" aria-label="Weekly NCMC share of Smart Cards plus NCMC"><g class="grid">"#
    );
    for (value, label) in [(0.0, "0%"), (0.5, "50%"), (1.0, "100%")] {
        let yy = share_y(value);
        svg.push_str(&format!(
            r#"<line x1="{LEFT}" x2="{}" y1="{yy:.1}" y2="{yy:.1}"></line><text x="{}" y="{:.1}" text-anchor="end">{label}</text>"#,
            WIDTH - RIGHT,
            LEFT - 9.0,
            yy + 4.0
        ));
    }
    svg.push_str(&format!(
        r#"<line x1="{LEFT}" x2="{}" y1="{total_base:.1}" y2="{total_base:.1}"></line><text x="{}" y="{:.1}" text-anchor="end">{}</text><text x="{}" y="{:.1}" text-anchor="end">0</text>"#,
        WIDTH - RIGHT,
        LEFT - 9.0,
        total_base - TOTAL_H + 4.0,
        compact_number(max_ncmc),
        LEFT - 9.0,
        total_base + 4.0
    ));
    svg.push_str("</g>");
    let points: Vec<(usize, f64, f64)> = weeks
        .iter()
        .enumerate()
        .filter_map(|(index, week)| week.share().map(|share| (index, x(index), share_y(share))))
        .collect();
    for window in points.windows(2) {
        let (left_index, x1, y1) = window[0];
        let (right_index, x2, y2) = window[1];
        if right_index == left_index + 1 {
            svg.push_str(&format!(
                r#"<line class="ncmc-share-line" x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}"></line>"#
            ));
        }
    }
    for (index, week) in weeks.iter().enumerate() {
        let xx = x(index);
        if let Some(share) = week.share() {
            let yy = share_y(share);
            let tooltip = escape_attr(&format!(
                "Week of {}: NCMC {} of Smart Card + NCMC {} ({:.1}%), {} complete of {} published days",
                week.week.format("%d %b %Y"),
                format_number_full(week.ncmc),
                format_number_full(week.commuter_total),
                share * 100.0,
                week.complete_days,
                week.published_days
            ));
            svg.push_str(&format!(
                r#"<circle class="ncmc-share-point" cx="{xx:.1}" cy="{yy:.1}" r="5" tabindex="0" role="img" aria-label="{tooltip}" data-tooltip="{tooltip}"></circle>"#
            ));
            let bar_height = week.ncmc / max_ncmc * TOTAL_H;
            let bar_y = total_base - bar_height;
            let total_tooltip = escape_attr(&format!(
                "Week of {}: NCMC {} across {} complete days",
                week.week.format("%d %b %Y"),
                format_number_full(week.ncmc),
                week.complete_days
            ));
            svg.push_str(&format!(
                r#"<rect class="ncmc-count-bar" x="{:.1}" y="{bar_y:.1}" width="{bar_width}" height="{bar_height:.1}" aria-hidden="true"></rect><rect class="ncmc-count-hit" x="{:.1}" y="{bar_y:.1}" width="{bar_width}" height="{bar_height:.1}" fill="transparent" tabindex="0" role="img" aria-label="{total_tooltip}" data-tooltip="{total_tooltip}"></rect>"#,
                xx - bar_width / 2.0,
                xx - bar_width / 2.0
            ));
            if step >= 46.0 {
                svg.push_str(&format!(
                    r#"<text class="mix-coverage-label" x="{xx:.1}" y="{:.1}" text-anchor="middle">{}/{}d</text>"#,
                    total_base + 17.0,
                    week.complete_days,
                    week.published_days
                ));
            }
        } else {
            svg.push_str(&format!(
                r#"<rect class="mix-gap" x="{:.1}" y="{TOP}" width="{bar_width}" height="{SHARE_H}"></rect>"#,
                xx - bar_width / 2.0
            ));
        }
        let label_every = (weeks.len() / 10).max(1);
        if index % label_every == 0 || index + 1 == weeks.len() {
            svg.push_str(&format!(
                r#"<text class="x-label mix-x-label" x="{xx:.1}" y="{LABEL_Y}" text-anchor="middle">{}</text>"#,
                week.week.format("%d %b")
            ));
        }
    }
    svg.push_str("</svg>");
    svg.push_str(r#"<div class="mix-legend" aria-label="NCMC chart legend"><span><i class="ncmc-line-key"></i>NCMC share</span><span><i class="ncmc-count-key"></i>NCMC count</span></div>"#);
    svg.push_str(&format!(
        r#"<p class="insight">{}</p>"#,
        ncmc_share_insight(weeks)
    ));
    svg.push_str(&ncmc_share_table(weeks));
    svg.push_str("</div>");
    svg
}

fn ncmc_share_table(weeks: &[NcmcShareWeek]) -> String {
    let mut table = String::from(
        r#"<details class="accessible-data"><summary>View NCMC share data table</summary><div class="table-scroll"><table><caption>Weekly NCMC count and share of Smart Cards plus NCMC</caption><thead><tr><th scope="col">Week</th><th scope="col">Complete days</th><th scope="col">NCMC</th><th scope="col">Smart + NCMC</th><th scope="col">NCMC share</th></tr></thead><tbody>"#,
    );
    for week in weeks {
        table.push_str(&format!(
            "<tr><th scope=\"row\">{}</th><td>{}/{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            week.week.format("%d %b %Y"),
            week.complete_days,
            week.published_days,
            format_number_full(week.ncmc),
            format_number_full(week.commuter_total),
            week.share()
                .map(|share| format!("{:.1}%", share * 100.0))
                .unwrap_or_else(|| "—".into())
        ));
    }
    table.push_str("</tbody></table></div></details>");
    table
}

fn mix_percent(value: f64) -> String {
    format!("{value:.0}%")
}

pub fn fare_mix_insight(months: &[FareMixMonth]) -> String {
    let usable: Vec<&FareMixMonth> = months
        .iter()
        .filter(|month| month.total.is_some())
        .collect();
    if usable.len() < 2 {
        return "Not enough complete months in the selected range to compare the mix.".to_string();
    }
    let first = usable.first().expect("nonempty");
    let last = usable.last().expect("nonempty");
    let share = |month: &FareMixMonth, index: usize| {
        month
            .total
            .filter(|total| *total > 0.0)
            .map(|total| month.components[index] / total * 100.0)
            .unwrap_or(0.0)
    };
    format!(
        "Smart Cards {:.0}% → {:.0}%; NCMC {:.0}% → {:.0}% across complete observed days.",
        share(first, 0),
        share(last, 0),
        share(first, 1),
        share(last, 1)
    )
}

pub fn fare_mix_chart_markup(months: &[FareMixMonth]) -> String {
    if months.is_empty() {
        return r#"<div class="mix-wrap"><p class="mix-empty">No months are available in the selected range.</p></div>"#.to_string();
    }
    const WIDTH: f64 = 1100.0;
    const HEIGHT: f64 = 400.0;
    const LEFT: f64 = 48.0;
    const RIGHT: f64 = 22.0;
    const TOP: f64 = 42.0;
    const SHARE_H: f64 = 178.0;
    const GAP: f64 = 34.0;
    const TOTAL_H: f64 = 58.0;
    const LABEL_Y: f64 = 366.0;
    let plot_width = WIDTH - LEFT - RIGHT;
    let step = plot_width / months.len().max(1) as f64;
    let bar_width = (step * 0.58).clamp(14.0, 52.0);
    let max_total = months
        .iter()
        .filter_map(|month| month.total)
        .reduce(f64::max)
        .unwrap_or(1.0)
        .max(1.0);
    let total_base = TOP + SHARE_H + GAP + TOTAL_H;
    let mut svg = format!(
        r#"<div class="mix-wrap"><svg class="mix-chart" viewBox="0 0 {WIDTH} {HEIGHT}" role="img" aria-label="Monthly fare-media share mix with observed totals"><g class="grid">"#
    );
    for (value, label) in [(0.0, "0%"), (0.5, "50%"), (1.0, "100%")] {
        let yy = TOP + SHARE_H - value * SHARE_H;
        svg.push_str(&format!(
            r#"<line x1="{LEFT}" x2="{}" y1="{yy:.1}" y2="{yy:.1}"></line><text x="{}" y="{:.1}" text-anchor="end">{label}</text>"#,
            WIDTH - RIGHT,
            LEFT - 9.0,
            yy + 4.0
        ));
    }
    svg.push_str(&format!(
        r#"<line x1="{LEFT}" x2="{}" y1="{total_base:.1}" y2="{total_base:.1}"></line><text x="{}" y="{:.1}" text-anchor="end">{}</text><text x="{}" y="{:.1}" text-anchor="end">0</text>"#,
        WIDTH - RIGHT,
        LEFT - 9.0,
        total_base - TOTAL_H + 4.0,
        compact_number(max_total),
        LEFT - 9.0,
        total_base + 4.0
    ));
    svg.push_str("</g>");

    for (index, month) in months.iter().enumerate() {
        let xx = LEFT + step * (index as f64 + 0.5);
        if let Some(total) = month.total.filter(|total| *total > 0.0) {
            let mut cumulative = 0.0;
            for (component_index, (key, label)) in
                crate::data::WATERFALL_CATEGORIES.iter().enumerate()
            {
                let value = month.components[component_index];
                let share = value / total;
                let y = TOP + cumulative * SHARE_H;
                let height = (share * SHARE_H).max(if value > 0.0 { 1.0 } else { 0.0 });
                let tooltip = escape_attr(&format!(
                    "{} · {}: {} ({:.1}%), {} complete of {} published days",
                    month.month.format("%b %Y"),
                    label,
                    format_number_full(value),
                    share * 100.0,
                    month.complete_days,
                    month.published_days
                ));
                svg.push_str(&format!(
                    r#"<rect class="mix-segment mix-{key}" x="{:.1}" y="{y:.1}" width="{bar_width}" height="{height:.1}" aria-hidden="true"></rect><rect class="mix-hit" x="{:.1}" y="{y:.1}" width="{bar_width}" height="{height:.1}" fill="transparent" tabindex="0" role="img" aria-label="{tooltip}" data-tooltip="{tooltip}"></rect>"#,
                    xx - bar_width / 2.0,
                    xx - bar_width / 2.0
                ));
                cumulative += share;
            }
            let total_height = total / max_total * TOTAL_H;
            let total_y = total_base - total_height;
            let total_tooltip = escape_attr(&format!(
                "{} observed total: {} across {} complete days",
                month.month.format("%b %Y"),
                format_number_full(total),
                month.complete_days
            ));
            svg.push_str(&format!(
                r#"<rect class="mix-total-bar" x="{:.1}" y="{total_y:.1}" width="{bar_width}" height="{total_height:.1}" aria-hidden="true"></rect><rect class="mix-total-hit" x="{:.1}" y="{total_y:.1}" width="{bar_width}" height="{total_height:.1}" fill="transparent" tabindex="0" role="img" aria-label="{total_tooltip}" data-tooltip="{total_tooltip}"></rect><text class="mix-coverage-label" x="{xx:.1}" y="{:.1}" text-anchor="middle">{}/{}d</text>"#,
                xx - bar_width / 2.0,
                xx - bar_width / 2.0,
                total_base + 17.0,
                month.complete_days,
                month.published_days
            ));
        } else {
            svg.push_str(&format!(
                r#"<rect class="mix-gap" x="{:.1}" y="{TOP}" width="{bar_width}" height="{SHARE_H}"></rect>"#,
                xx - bar_width / 2.0
            ));
        }
        svg.push_str(&format!(
            r#"<text class="x-label mix-x-label" x="{xx:.1}" y="{LABEL_Y}" text-anchor="middle">{}</text>"#,
            month.month.format("%b ’%y")
        ));
    }
    svg.push_str("</svg>");
    svg.push_str(r#"<div class="mix-legend" aria-label="Fare media legend"><span><i class="mix-key mix-smart-card"></i>Smart Cards</span><span><i class="mix-key mix-ncmc"></i>NCMC</span><span><i class="mix-key mix-token"></i>Tokens</span><span><i class="mix-key mix-qr"></i>QR</span><span><i class="mix-key mix-group-ticket"></i>Group</span></div>"#);
    svg.push_str(&format!(
        r#"<p class="insight">{}</p>"#,
        fare_mix_insight(months)
    ));
    svg.push_str(&fare_mix_table(months));
    svg.push_str("</div>");
    svg
}

fn fare_mix_table(months: &[FareMixMonth]) -> String {
    let mut table = String::from(
        r#"<details class="accessible-data"><summary>View fare-media mix data table</summary><div class="table-scroll"><table><caption>Monthly observed totals and shares by fare medium</caption><thead><tr><th scope="col">Month</th><th scope="col">Complete days</th><th scope="col">Observed total</th><th scope="col">Smart Cards</th><th scope="col">NCMC</th><th scope="col">Tokens</th><th scope="col">QR</th><th scope="col">Group</th></tr></thead><tbody>"#,
    );
    for month in months {
        let share = |index: usize| {
            month
                .total
                .filter(|total| *total > 0.0)
                .map(|total| mix_percent(month.components[index] / total * 100.0))
                .unwrap_or_else(|| "—".into())
        };
        table.push_str(&format!(
            "<tr><th scope=\"row\">{}</th><td>{}/{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            month.month.format("%b %Y"),
            month.complete_days,
            month.published_days,
            month.total.map(format_number_full).unwrap_or_else(|| "—".into()),
            share(0), share(1), share(2), share(3), share(4)
        ));
    }
    table.push_str("</tbody></table></div></details>");
    table
}

pub fn waterfall_insight(waterfall: &ComponentWaterfall) -> String {
    let Some(total_delta) = waterfall.delta_total else {
        return String::new();
    };
    let mut contributors = waterfall.components.clone();
    contributors.sort_by(|a, b| b.delta.abs().total_cmp(&a.delta.abs()));
    let top = contributors
        .iter()
        .take(2)
        .map(|component| format!("{} {}", component.label, signed_full(component.delta)))
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "Total difference {}. Largest component movements: {}.",
        signed_full(total_delta),
        top
    )
}

pub fn waterfall_chart_markup(waterfall: Option<&ComponentWaterfall>) -> String {
    let Some(waterfall) = waterfall else {
        return r#"<div class="waterfall-wrap"><p class="waterfall-empty">The selected day does not have complete fare-media components.</p></div>"#.to_string();
    };
    if waterfall.reference_total.is_none() || waterfall.components.is_empty() {
        return format!(
            r#"<div class="waterfall-wrap"><p class="waterfall-empty">Only {} nearby same-weekday reports have complete components; at least 4 are required.</p></div>"#,
            waterfall.baseline_n
        );
    }
    const WIDTH: f64 = 1100.0;
    const HEIGHT: f64 = 360.0;
    const LEFT: f64 = 70.0;
    const RIGHT: f64 = 34.0;
    const TOP: f64 = 38.0;
    const BOTTOM: f64 = 62.0;
    let plot_width = WIDTH - LEFT - RIGHT;
    let plot_height = HEIGHT - TOP - BOTTOM;
    let reference_total = waterfall
        .reference_total
        .expect("available waterfall has reference");
    let mut values = vec![reference_total, waterfall.actual_total];
    for component in &waterfall.components {
        values.push(component.cumulative_start);
        values.push(component.cumulative_end);
    }
    let min_value = values.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max_value = values.iter().copied().reduce(f64::max).unwrap_or(1.0);
    let spread = (max_value - min_value).max(1.0);
    let y_min = (min_value - spread * 0.12).max(0.0);
    let y_max = max_value + spread * 0.12;
    let y = |value: f64| TOP + plot_height - (value - y_min) / (y_max - y_min) * plot_height;
    let axis_label = |value: f64| {
        if value.abs() >= 1_000_000.0 {
            let decimals = if y_max - y_min < 250_000.0 { 2 } else { 1 };
            format!("{:.*}M", decimals, value / 1_000_000.0)
        } else {
            compact_number(value)
        }
    };
    let slots = waterfall.components.len() + 2;
    let step = plot_width / slots as f64;
    let x = |index: usize| LEFT + step * (index as f64 + 0.5);
    let bar_width = (step * 0.56).clamp(44.0, 78.0);

    let mut svg = format!(
        r#"<div class="waterfall-wrap"><svg class="waterfall-chart" viewBox="0 0 {WIDTH} {HEIGHT}" role="img" aria-label="Fare-media component waterfall for {}"><g class="grid">"#,
        escape_attr(&waterfall.date.format("%A, %-d %B %Y").to_string())
    );
    for tick in 0..=4 {
        let value = y_min + (y_max - y_min) * tick as f64 / 4.0;
        let yy = y(value);
        svg.push_str(&format!(
            r#"<line x1="{LEFT}" x2="{}" y1="{yy:.1}" y2="{yy:.1}"></line><text x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            WIDTH - RIGHT,
            LEFT - 10.0,
            yy + 4.0,
            axis_label(value)
        ));
    }

    // Reference and selected totals are horizontal anchors rather than bars:
    // the vertical scale is zoomed so category movements remain visible.
    let reference_y = y(reference_total);
    let actual_y = y(waterfall.actual_total);
    let reference_x = x(0);
    let actual_x = x(slots - 1);
    svg.push_str(&format!(
        r#"<line class="waterfall-total-anchor" x1="{:.1}" x2="{:.1}" y1="{reference_y:.1}" y2="{reference_y:.1}"></line><text class="waterfall-total-label" x="{reference_x:.1}" y="{:.1}" text-anchor="middle">{}</text>"#,
        reference_x - bar_width / 2.0,
        reference_x + bar_width / 2.0,
        reference_y - 10.0,
        compact_number(reference_total)
    ));

    for (index, component) in waterfall.components.iter().enumerate() {
        let xx = x(index + 1);
        let start_y = y(component.cumulative_start);
        let end_y = y(component.cumulative_end);
        let top_y = start_y.min(end_y);
        let height = (start_y - end_y).abs().max(2.0);
        let class = if component.delta >= 0.0 {
            "positive"
        } else {
            "negative"
        };
        let label = format!(
            "{}: {} actual vs {} reference; delta {}",
            component.label,
            format_number_full(component.actual),
            format_number_full(component.reference),
            signed_full(component.delta)
        );
        let safe = escape_attr(&label);
        svg.push_str(&format!(
            r#"<rect class="waterfall-hit" x="{:.1}" y="{TOP}" width="{bar_width}" height="{plot_height}" fill="transparent" tabindex="0" role="img" aria-label="{safe}" data-tooltip="{safe}"></rect><rect class="waterfall-bar {class}" x="{:.1}" y="{top_y:.1}" width="{bar_width}" height="{height:.1}" aria-hidden="true"></rect><text class="waterfall-delta {class}" x="{xx:.1}" y="{:.1}" text-anchor="middle">{}</text><text class="x-label waterfall-x-label" x="{xx:.1}" y="{}" text-anchor="middle">{}</text>"#,
            xx - bar_width / 2.0,
            xx - bar_width / 2.0,
            if component.delta >= 0.0 { top_y - 8.0 } else { top_y + height + 15.0 },
            signed_full(component.delta),
            HEIGHT - 24.0,
            component.label
        ));
        if index + 1 < waterfall.components.len() {
            let next_x = x(index + 2);
            svg.push_str(&format!(
                r#"<line class="waterfall-connector" x1="{:.1}" x2="{:.1}" y1="{end_y:.1}" y2="{end_y:.1}"></line>"#,
                xx + bar_width / 2.0,
                next_x - bar_width / 2.0
            ));
        }
    }
    if let Some(last) = waterfall.components.last() {
        let last_x = x(waterfall.components.len());
        svg.push_str(&format!(
            r#"<line class="waterfall-connector" x1="{:.1}" x2="{:.1}" y1="{:.1}" y2="{:.1}"></line>"#,
            last_x + bar_width / 2.0,
            actual_x - bar_width / 2.0,
            y(last.cumulative_end),
            y(last.cumulative_end)
        ));
    }
    svg.push_str(&format!(
        r#"<line class="waterfall-total-anchor actual" x1="{:.1}" x2="{:.1}" y1="{actual_y:.1}" y2="{actual_y:.1}"></line><text class="waterfall-total-label" x="{actual_x:.1}" y="{:.1}" text-anchor="middle">{}</text><text class="x-label waterfall-x-label total" x="{reference_x:.1}" y="{}" text-anchor="middle">Reference</text><text class="x-label waterfall-x-label total" x="{actual_x:.1}" y="{}" text-anchor="middle">Selected day</text>"#,
        actual_x - bar_width / 2.0,
        actual_x + bar_width / 2.0,
        actual_y - 10.0,
        compact_number(waterfall.actual_total),
        HEIGHT - 24.0,
        HEIGHT - 24.0
    ));
    svg.push_str("</g></svg>");
    svg.push_str(&format!(
        r#"<div class="waterfall-legend" aria-label="Waterfall chart legend"><span><i class="legend-dot high"></i>Increase vs reference</span><span><i class="legend-dot low"></i>Decrease vs reference</span><span>n = {} nearby reports</span></div>"#,
        waterfall.baseline_n
    ));
    let insight = waterfall_insight(waterfall);
    if !insight.is_empty() {
        svg.push_str(&format!(r#"<p class="insight">{insight}</p>"#));
    }
    svg.push_str(&waterfall_table(waterfall));
    svg.push_str("</div>");
    svg
}

fn waterfall_table(waterfall: &ComponentWaterfall) -> String {
    let mut table = String::from(
        r#"<details class="accessible-data"><summary>View component waterfall data table</summary><div class="table-scroll"><table><caption>Fare-media actuals, references, and arithmetic deltas</caption><thead><tr><th scope="col">Category</th><th scope="col">Reference</th><th scope="col">Actual</th><th scope="col">Delta</th></tr></thead><tbody>"#,
    );
    for component in &waterfall.components {
        table.push_str(&format!(
            "<tr><th scope=\"row\">{}</th><td>{}</td><td>{}</td><td>{}</td></tr>",
            component.label,
            format_number_full(component.reference),
            format_number_full(component.actual),
            signed_full(component.delta)
        ));
    }
    if let (Some(reference), Some(delta)) = (waterfall.reference_total, waterfall.delta_total) {
        table.push_str(&format!(
            "<tr><th scope=\"row\">Total</th><td>{}</td><td>{}</td><td>{}</td></tr>",
            format_number_full(reference),
            format_number_full(waterfall.actual_total),
            signed_full(delta)
        ));
    }
    table.push_str("</tbody></table></div></details>");
    table
}

fn baseline_table(points: &[WeekdayBaselinePoint]) -> String {
    let mut table = String::from(
        r#"<details class="accessible-data"><summary>View weekday baseline data table</summary><div class="table-scroll"><table><caption>Daily totals and rolling same-weekday baselines</caption><thead><tr><th scope="col">Date</th><th scope="col">Weekday</th><th scope="col">Total</th><th scope="col">Median</th><th scope="col">IQR low</th><th scope="col">IQR high</th><th scope="col">Delta</th><th scope="col">Status</th><th scope="col">Baseline n</th></tr></thead><tbody>"#,
    );
    for point in points {
        let delta = baseline_delta_pct(point)
            .map(|value| format!("{value:+.1}%"))
            .unwrap_or_else(|| "—".to_string());
        table.push_str(&format!(
            "<tr><th scope=\"row\">{}</th><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            point.date,
            weekday_name(point.weekday),
            format_number_full(point.total),
            point.median.map(format_number_full).unwrap_or_else(|| "—".to_string()),
            point.lower_quartile.map(format_number_full).unwrap_or_else(|| "—".to_string()),
            point.upper_quartile.map(format_number_full).unwrap_or_else(|| "—".to_string()),
            delta,
            point.status.label(),
            point.baseline_n
        ));
    }
    table.push_str("</tbody></table></div></details>");
    table
}

pub fn insight_line(summary: &RangeSummary) -> String {
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

pub fn fare_media_breakdown(record: &RidershipRecord) -> String {
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

pub fn data_card_markup(dataset: &Dataset, range: DateRange) -> String {
    let record = dataset
        .records
        .iter()
        .find(|record| record.date == range.end);

    let Some(record) = record else {
        return r#"<div class="data-card-wrap" role="status" aria-live="polite">
  <div class="data-card-missing">
    <p class="data-card-missing-title">No data for this date.</p>
    <p class="data-card-missing-hint">Pick another end date to see a single day's breakdown.</p>
  </div>
</div>"#
            .to_string();
    };

    // Determine prev/next available dates for day cycling buttons.
    let current_idx = dataset.records.iter().position(|r| r.date == range.end);
    let prev_date_str = match current_idx {
        Some(idx) if idx > 0 => dataset.records[idx - 1].date.to_string(),
        _ => String::new(),
    };
    let next_date_str = match current_idx {
        Some(idx) if idx + 1 < dataset.records.len() => dataset.records[idx + 1].date.to_string(),
        _ => String::new(),
    };
    let prev_disabled = prev_date_str.is_empty();
    let next_disabled = next_date_str.is_empty();

    let date_display = record.date.format("%A, %-d %B %Y").to_string();
    let date_short = record.date.format("%-d %b %Y").to_string();

    let total = record.total_ridership;
    let total_label = total
        .map(format_number_full)
        .unwrap_or_else(|| "—".to_string());

    // Fare media values and percentages
    let fare_items = [
        ("smart-card", "Smart Cards", record.smart_card),
        ("token", "Tokens", record.token),
        ("qr", "QR Tickets", record.qr),
        ("ncmc", "NCMC", record.ncmc),
        ("group", "Group Ticket", record.group_ticket),
    ];

    let total_for_pct = total.unwrap_or(0.0);
    let mut bar_segments = String::new();
    let mut legend_items = String::new();
    let mut payment_boxes = String::new();

    for (key, label, value) in &fare_items {
        let value = *value;
        let pct = if total_for_pct > 0.0 {
            (value.unwrap_or(0.0) / total_for_pct * 100.0 * 10.0).round() / 10.0
        } else {
            0.0
        };
        let pct_label = format!("{:.1}", pct);
        let value_label = value
            .map(format_number_full)
            .unwrap_or_else(|| "—".to_string());

        // Bar segment
        if pct > 0.0 {
            bar_segments.push_str(&format!(
                r#"<div class="dc-bar-segment dc-bar-{key}" style="width:{pct}%" role="img" aria-label="{label}: {pct_label}%"></div>"#
            ));
        }

        // Legend item
        legend_items.push_str(&format!(
            r#"<div class="dc-legend-item"><span class="dc-legend-color dc-bar-{key}"></span>{label} {pct_label}%</div>"#
        ));

        // Payment box
        let breakdown = match *key {
            "smart-card" => {
                let subs = [
                    ("Stored Value", record.stored_value),
                    ("One Day Pass", record.one_day_pass),
                    ("Three Day Pass", record.three_day_pass),
                    ("Five Day Pass", record.five_day_pass),
                ];
                let sub_html: String = subs
                    .iter()
                    .filter_map(|(sub_label, sub_value)| {
                        sub_value.map(|v| {
                            format!(
                                r#"<div class="dc-sub-row"><span class="dc-sub-label">{sub_label}</span><span class="dc-sub-value">{}</span></div>"#,
                                format_number_full(v)
                            )
                        })
                    })
                    .collect();
                if sub_html.is_empty() {
                    String::new()
                } else {
                    format!(r#"<div class="dc-breakdown">{sub_html}</div>"#)
                }
            }
            "qr" => {
                let subs = [
                    ("Namma Metro", record.qr_namma),
                    ("WhatsApp", record.qr_whatsapp),
                    ("Paytm", record.qr_paytm),
                    ("ONDC", record.qr_ondc),
                ];
                let sub_html: String = subs
                    .iter()
                    .filter_map(|(sub_label, sub_value)| {
                        sub_value.map(|v| {
                            format!(
                                r#"<div class="dc-sub-row"><span class="dc-sub-label">{sub_label}</span><span class="dc-sub-value">{}</span></div>"#,
                                format_number_full(v)
                            )
                        })
                    })
                    .collect();
                if sub_html.is_empty() {
                    String::new()
                } else {
                    format!(r#"<div class="dc-breakdown">{sub_html}</div>"#)
                }
            }
            _ => String::new(),
        };

        payment_boxes.push_str(&format!(
            r#"<div class="dc-payment-box dc-bar-{key}">
  <div class="dc-payment-value">{value_label}</div>
  <div class="dc-payment-label">{label}</div>
  {breakdown}
</div>"#
        ));
    }

    let missing_note = if total.is_none() {
        r#"<p class="dc-missing-note">Some fare media values are missing for this date. The total may be incomplete.</p>"#
    } else {
        ""
    };

    format!(
        r#"<div class="data-card-wrap" role="region" aria-label="Ridership data card for {date_short}">
  <div class="data-card-inner">
    <div class="dc-top-story">
      <div class="dc-date-row">
        <button class="dc-day-nav dc-day-prev" type="button" aria-label="Previous day" data-target-date="{prev_date_str}" aria-disabled="{prev_disabled}" tabindex="{}"><span aria-hidden="true">←</span></button>
        <span class="dc-date-value">{date_display}</span>
        <button class="dc-day-nav dc-day-next" type="button" aria-label="Next day" data-target-date="{next_date_str}" aria-disabled="{next_disabled}" tabindex="{}"><span aria-hidden="true">→</span></button>
      </div>
      <div class="dc-total-container">
        <div class="dc-total-value">{total_label}</div>
        <div class="dc-total-label">Total Rides</div>
      </div>
      <div class="dc-bar-chart" role="img" aria-label="Fare media proportion bar">
        {bar_segments}
      </div>
      <div class="dc-legend">{legend_items}</div>
      {missing_note}
    </div>
    <div class="dc-bottom-story">
      <div class="dc-payment-grid">
        {payment_boxes}
      </div>
    </div>
  </div>
</div>"#,
        if prev_disabled { "-1" } else { "0" },
        if next_disabled { "-1" } else { "0" },
    )
}
