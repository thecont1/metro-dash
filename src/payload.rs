use serde::Serialize;

use crate::charts::{
    band_index, band_label, fare_media_breakdown, format_number_full, format_range,
    insight_line, weekday_name,
};
use crate::data::{
    record_index, summarise, Dataset, DateRange, RangeSummary, RidershipRecord,
};
use chrono::{Datelike, NaiveDate, Weekday};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartPayload {
    pub range: PayloadRange,
    pub dataset: PayloadDataset,
    pub summary: PayloadSummary,
    pub charts: PayloadCharts,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadRange {
    pub start: String,
    pub end: String,
    pub label: String,
    pub start_index: usize,
    pub end_index: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadDataset {
    pub min_date: String,
    pub max_date: String,
    pub row_count: usize,
    pub refreshed_at: String,
    pub available_dates: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadSummary {
    pub calendar_days: usize,
    pub observation_days: usize,
    pub missing_days: usize,
    pub valid_total_count: usize,
    pub total_min: Option<f64>,
    pub total_max: Option<f64>,
    pub total_min_label: Option<String>,
    pub total_max_label: Option<String>,
    pub insufficient_percentiles: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadCharts {
    pub data_card: DataCardPayload,
    pub calendar: CalendarPayload,
    pub commute_casual: CommuteCasualPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarPayload {
    pub cells: Vec<CalendarCellPayload>,
    pub legend: Vec<LegendItemPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarCellPayload {
    pub date: String,
    pub label: String,
    pub weekday: usize,
    pub week: usize,
    pub month_label: Option<String>,
    pub month_gap: usize,
    pub total: Option<f64>,
    pub total_label: Option<String>,
    pub band_label: Option<String>,
    pub band_index: Option<usize>,
    pub missing: bool,
    pub breakdown: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegendItemPayload {
    pub label: &'static str,
    pub band_index: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommuteCasualPayload {
    pub weekdays: Vec<WeekdayPayload>,
    pub series: Vec<SeriesPayload>,
    pub insight: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekdayPayload {
    pub index: usize,
    pub short: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPayload {
    pub key: &'static str,
    pub label: &'static str,
    pub points: Vec<WeekdayPointPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekdayPointPayload {
    pub weekday: &'static str,
    pub weekday_short: &'static str,
    pub index: usize,
    pub n: usize,
    pub mean: Option<f64>,
    pub mean_label: Option<String>,
    pub standard_deviation: Option<f64>,
    pub standard_deviation_label: Option<String>,
    pub min: Option<f64>,
    pub min_label: Option<String>,
    pub max: Option<f64>,
    pub max_label: Option<String>,
    pub tooltip: String,
}

pub fn chart_payload_for(dataset: &Dataset, range: DateRange) -> ChartPayload {
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
            data_card: data_card_payload(dataset, range),
            calendar: CalendarPayload {
                cells: calendar_payload_cells(dataset, range, &summary),
                legend: Vec::new(),
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
    let map: std::collections::HashMap<NaiveDate, &RidershipRecord> = dataset
        .records
        .iter()
        .filter(|record| record.date >= range.start && record.date <= range.end)
        .map(|record| (record.date, record))
        .collect();
    let mut cells = Vec::new();
    let mut date = range.start;
    let mut month_gap_count = 0usize;
    let mut last_monday_month: Option<u32> = None;
    while date <= range.end {
        let record = map.get(&date).copied();
        let value = record.and_then(|record| record.total_ridership);
        let band = value.and_then(|value| band_label(value, summary));
        let breakdown = record.map(fare_media_breakdown).unwrap_or_default();
        let label = calendar_cell_label(date, value, band, &breakdown);
        let week = ((date - first_monday).num_days() / 7) as usize;
        let previous = date - chrono::Duration::days(1);
        let is_new_month = date.month() != previous.month();
        let month_label = (date == range.start || is_new_month)
            .then(|| date.format("%b %Y").to_string());
        if date.weekday() == chrono::Weekday::Mon {
            match last_monday_month {
                None => last_monday_month = Some(date.month()),
                Some(prev) if prev != date.month() => {
                    last_monday_month = Some(date.month());
                    month_gap_count += 1;
                }
                _ => {}
            }
        }
        cells.push(CalendarCellPayload {
            date: date.to_string(),
            label,
            weekday: date.weekday().num_days_from_monday() as usize,
            week,
            month_label,
            month_gap: month_gap_count,
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
    metrics: &[crate::data::WeekdayMetric],
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
    metric: &crate::data::WeekdayMetric,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataCardPayload {
    pub date: String,
    pub date_display: String,
    pub total: Option<f64>,
    pub total_label: String,
    pub fare_media: Vec<FareMediaItem>,
    pub has_missing: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FareMediaItem {
    pub key: &'static str,
    pub label: &'static str,
    pub value: Option<f64>,
    pub value_label: String,
    pub percentage: f64,
    pub breakdown: Vec<FareMediaSubItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FareMediaSubItem {
    pub label: String,
    pub value: f64,
    pub value_label: String,
}

fn data_card_payload(dataset: &Dataset, range: DateRange) -> DataCardPayload {
    let record = dataset.records.iter().find(|r| r.date == range.end);

    match record {
        Some(record) => {
            let total = record.total_ridership;
            let total_for_pct = total.unwrap_or(0.0);
            let fare_media = vec![
                fare_media_item("smart-card", "Smart Cards", record.smart_card, total_for_pct, &[
                    ("Stored Value", record.stored_value),
                    ("One Day Pass", record.one_day_pass),
                    ("Three Day Pass", record.three_day_pass),
                    ("Five Day Pass", record.five_day_pass),
                ]),
                fare_media_item("token", "Tokens", record.token, total_for_pct, &[]),
                fare_media_item("qr", "QR Tickets", record.qr, total_for_pct, &[
                    ("Namma Metro", record.qr_namma),
                    ("WhatsApp", record.qr_whatsapp),
                    ("Paytm", record.qr_paytm),
                ]),
                fare_media_item("ncmc", "NCMC", record.ncmc, total_for_pct, &[]),
                fare_media_item("group", "Group Ticket", record.group_ticket, total_for_pct, &[]),
            ];
            DataCardPayload {
                date: record.date.to_string(),
                date_display: record.date.format("%A, %-d %B %Y").to_string(),
                total,
                total_label: total.map(format_number_full).unwrap_or_else(|| "—".to_string()),
                fare_media,
                has_missing: total.is_none(),
            }
        }
        None => DataCardPayload {
            date: range.end.to_string(),
            date_display: range.end.format("%A, %-d %B %Y").to_string(),
            total: None,
            total_label: "—".to_string(),
            fare_media: Vec::new(),
            has_missing: true,
        },
    }
}

fn fare_media_item(
    key: &'static str,
    label: &'static str,
    value: Option<f64>,
    total: f64,
    subs: &[(&'static str, Option<f64>)],
) -> FareMediaItem {
    let percentage = if total > 0.0 {
        value.unwrap_or(0.0) / total * 100.0
    } else {
        0.0
    };
    let breakdown: Vec<FareMediaSubItem> = subs
        .iter()
        .filter_map(|(sub_label, sub_value)| {
            sub_value.map(|v| FareMediaSubItem {
                label: sub_label.to_string(),
                value: v,
                value_label: format_number_full(v),
            })
        })
        .collect();
    FareMediaItem {
        key,
        label,
        value,
        value_label: value.map(format_number_full).unwrap_or_else(|| "—".to_string()),
        percentage,
        breakdown,
    }
}
