use std::collections::HashMap;
use std::fmt::Write as _;

use chrono::{Datelike, NaiveDate, Weekday};

use crate::data::{
    BaselineStatus, Dataset, DateRange, RangeSummary, RidershipRecord, WeekdayBaselinePoint,
    WeekdayMetric,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Chart {
    DataCard,
    Calendar,
    CommuteCasual,
    WeekdayBaseline,
}

#[derive(Debug, Clone, Copy)]
pub struct ChartDefinition {
    pub chart: Chart,
    pub slug: &'static str,
    pub title: &'static str,
    pub deck: &'static str,
    pub method: &'static str,
}

pub static CHARTS: [ChartDefinition; 4] = [
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
        deck: "Each square is one calendar day. Deeper the purple, higher the total.",
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
