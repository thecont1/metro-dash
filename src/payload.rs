use serde::Serialize;

use crate::charts::{
    CHARTS, band_index, band_label, baseline_insight, fare_media_breakdown,
    field_completeness_markup, format_number_full, format_range, insight_line,
    month_weekday_matrix_insight, monthly_estimates_markup, reporting_pulse_markup,
    variability_markup, weekday_distribution_insight, weekday_name, weekend_signature_insight,
};
use crate::data::{
    BASELINE_MIN_OBSERVATIONS, BASELINE_WINDOW, ComponentWaterfall, Dataset, DateRange,
    FareMixMonth, MonthWeekdayMatrix, NcmcShareWeek, QrMixMonth, RangeSummary, RidershipRecord,
    SmartMixMonth, WeekdayBaselinePoint, WeekdayDistribution, WeekendSignature,
    component_waterfall, fare_mix_series, field_completeness_grid, last_n_months_range,
    month_weekday_matrix, monthly_estimate_series, ncmc_share_series, qr_mix_series, record_index,
    reporting_pulse_series, smart_mix_series, summarise, variability_ranking,
    weekday_baseline_series, weekday_distribution_series, weekend_signature_series,
};
use chrono::{Datelike, NaiveDate, Weekday};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartPayload {
    pub range: PayloadRange,
    pub dataset: PayloadDataset,
    pub summary: PayloadSummary,
    pub charts: PayloadCharts,
    pub presets: Vec<PayloadPreset>,
    pub definitions: Vec<ChartDefinitionPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartDefinitionPayload {
    pub slug: &'static str,
    pub title: &'static str,
    pub deck: &'static str,
    pub method: &'static str,
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
    /// Fingerprint of the source CSV; lets the client detect in-place data
    /// corrections that leave `max_date`/`row_count` untouched.
    pub revision: String,
    /// Where the client fetches the CSV for live recomputation — the
    /// canonical GitHub URL, or a same-origin copy bundled by `export --csv`.
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadPreset {
    pub slug: &'static str,
    pub label: &'static str,
    pub start: String,
    pub end: String,
}

/// A named quick-filter range ("Last 3 months", …, "Use all available data").
#[derive(Debug, Clone, Copy)]
pub struct RangePreset {
    pub slug: &'static str,
    pub label: &'static str,
    pub range: DateRange,
}

/// Quick-filter targets computed from the active dataset. Shared by the SSR
/// range-action links and every JSON payload so the client can refresh the
/// links' `data-start`/`data-end` whenever it adopts a newer dataset.
pub fn range_presets(dataset: &Dataset) -> Vec<RangePreset> {
    [
        (
            "last-3-months",
            "Last 3 months",
            last_n_months_range(dataset, 3),
        ),
        (
            "last-6-months",
            "Last 6 months",
            last_n_months_range(dataset, 6),
        ),
        (
            "last-9-months",
            "Last 9 months",
            last_n_months_range(dataset, 9),
        ),
        (
            "last-one-year",
            "Last one year",
            last_n_months_range(dataset, 12),
        ),
        (
            "use-all-data",
            "Use all available data",
            DateRange {
                start: dataset.min_date,
                end: dataset.max_date,
            },
        ),
    ]
    .iter()
    .map(|(slug, label, range)| RangePreset {
        slug,
        label,
        range: *range,
    })
    .collect()
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
    pub percentile_values: Vec<(u8, String)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadCharts {
    pub data_card: DataCardPayload,
    pub calendar: CalendarPayload,
    pub commute_casual: CommuteCasualPayload,
    pub weekday_baseline: WeekdayBaselinePayload,
    pub component_waterfall: ComponentWaterfallPayload,
    pub fare_media_mix: FareMediaMixPayload,
    pub ncmc_share: NcmcSharePayload,
    pub qr_mix: FareMediaMixPayload,
    pub smart_mix: FareMediaMixPayload,
    pub weekday_distribution: WeekdayDistributionPayload,
    pub weekend_signatures: WeekendSignaturesPayload,
    pub month_weekday_matrix: MonthWeekdayMatrixPayload,
    pub variability_ranking: HtmlChartPayload,
    pub reporting_pulse: HtmlChartPayload,
    pub monthly_estimates: HtmlChartPayload,
    pub field_completeness: HtmlChartPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NcmcSharePayload {
    pub weeks: Vec<NcmcShareWeekPayload>,
    pub insight: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NcmcShareWeekPayload {
    pub week: String,
    pub label: String,
    pub published_days: usize,
    pub complete_days: usize,
    pub ncmc: f64,
    pub ncmc_label: String,
    pub commuter_total: f64,
    pub commuter_total_label: String,
    pub share: Option<f64>,
    pub share_label: Option<String>,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FareMediaMixPayload {
    pub months: Vec<FareMixMonthPayload>,
    pub insight: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FareMixMonthPayload {
    pub month: String,
    pub label: String,
    pub published_days: usize,
    pub complete_days: usize,
    pub total: Option<f64>,
    pub total_label: Option<String>,
    pub components: Vec<FareMixComponentPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FareMixComponentPayload {
    pub key: &'static str,
    pub label: &'static str,
    pub value: f64,
    pub value_label: String,
    pub share: Option<f64>,
    pub share_label: Option<String>,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentWaterfallPayload {
    pub available: bool,
    pub message: Option<String>,
    pub date: String,
    pub date_display: String,
    pub weekday: &'static str,
    pub baseline_n: usize,
    pub reference_total: Option<f64>,
    pub reference_total_label: Option<String>,
    pub actual_total: f64,
    pub actual_total_label: String,
    pub delta_total: Option<f64>,
    pub delta_total_label: Option<String>,
    pub components: Vec<ComponentDeltaPayload>,
    pub insight: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDeltaPayload {
    pub key: &'static str,
    pub label: &'static str,
    pub reference: f64,
    pub reference_label: String,
    pub actual: f64,
    pub actual_label: String,
    pub delta: f64,
    pub delta_label: String,
    pub cumulative_start: f64,
    pub cumulative_end: f64,
    pub positive: bool,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekdayBaselinePayload {
    pub window: usize,
    pub min_observations: usize,
    pub points: Vec<WeekdayBaselinePointPayload>,
    pub high_count: usize,
    pub low_count: usize,
    pub insufficient_count: usize,
    pub insight: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekdayBaselinePointPayload {
    pub date: String,
    pub weekday: &'static str,
    pub weekday_short: &'static str,
    pub total: f64,
    pub total_label: String,
    pub lower_quartile: Option<f64>,
    pub lower_quartile_label: Option<String>,
    pub median: Option<f64>,
    pub median_label: Option<String>,
    pub upper_quartile: Option<f64>,
    pub upper_quartile_label: Option<String>,
    pub baseline_n: usize,
    pub delta_pct: Option<f64>,
    pub delta_pct_label: Option<String>,
    pub status: &'static str,
    pub status_label: &'static str,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HtmlChartPayload {
    pub html: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthWeekdayMatrixPayload {
    pub months: Vec<MonthLabelPayload>,
    pub cells: Vec<MonthWeekdayCellPayload>,
    pub insight: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthLabelPayload {
    pub month: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthWeekdayCellPayload {
    pub month: String,
    pub weekday: &'static str,
    pub weekday_short: &'static str,
    pub weekday_index: usize,
    pub n: usize,
    pub median: Option<f64>,
    pub median_label: Option<String>,
    pub ratio: Option<f64>,
    pub ratio_label: Option<String>,
    pub css_class: String,
    pub heat: f64,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekendSignaturesPayload {
    pub groups: Vec<WeekendSignaturePayload>,
    pub insight: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekendSignaturePayload {
    pub key: &'static str,
    pub label: &'static str,
    pub published_days: usize,
    pub complete_days: usize,
    pub minimum: Option<f64>,
    pub minimum_label: Option<String>,
    pub lower_quartile: Option<f64>,
    pub lower_quartile_label: Option<String>,
    pub median: Option<f64>,
    pub median_label: Option<String>,
    pub upper_quartile: Option<f64>,
    pub upper_quartile_label: Option<String>,
    pub maximum: Option<f64>,
    pub maximum_label: Option<String>,
    pub components: Vec<FareMixComponentPayload>,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekdayDistributionPayload {
    pub days: Vec<WeekdayDistributionDayPayload>,
    pub insight: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekdayDistributionDayPayload {
    pub weekday: &'static str,
    pub weekday_short: &'static str,
    pub index: usize,
    pub n: usize,
    pub minimum: Option<f64>,
    pub minimum_label: Option<String>,
    pub lower_quartile: Option<f64>,
    pub lower_quartile_label: Option<String>,
    pub median: Option<f64>,
    pub median_label: Option<String>,
    pub upper_quartile: Option<f64>,
    pub upper_quartile_label: Option<String>,
    pub maximum: Option<f64>,
    pub maximum_label: Option<String>,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarPayload {
    pub cells: Vec<CalendarCellPayload>,
    pub legend_html: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarCellPayload {
    pub date: String,
    pub label: String,
    pub weekday: usize,
    pub week: usize,
    pub col: usize,
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
    let baseline_points = weekday_baseline_series(dataset, range);
    let weekday_baseline = weekday_baseline_payload(&baseline_points);
    let waterfall = component_waterfall(dataset, range.end);
    let component_waterfall = component_waterfall_payload(waterfall.as_ref(), range.end);
    let fare_media_mix = fare_mix_payload(&fare_mix_series(dataset, range));
    let ncmc_share = ncmc_share_payload(&ncmc_share_series(dataset, range));
    let qr_mix = qr_mix_payload(&qr_mix_series(dataset, range));
    let smart_mix = smart_mix_payload(&smart_mix_series(dataset, range));
    let weekday_distribution =
        weekday_distribution_payload(&weekday_distribution_series(dataset, range));
    let weekend_signatures = weekend_signatures_payload(&weekend_signature_series(dataset, range));
    let month_weekday_matrix = month_weekday_matrix_payload(&month_weekday_matrix(dataset, range));
    let variability_ranking = HtmlChartPayload {
        html: variability_markup(&variability_ranking(dataset, range)),
    };
    let reporting_pulse = HtmlChartPayload {
        html: reporting_pulse_markup(&reporting_pulse_series(dataset, range)),
    };
    let monthly_estimates = HtmlChartPayload {
        html: monthly_estimates_markup(&monthly_estimate_series(dataset, range)),
    };
    let field_completeness = HtmlChartPayload {
        html: field_completeness_markup(&field_completeness_grid(dataset, range)),
    };
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
            revision: dataset.revision.to_string(),
            source_url: dataset.source_url.clone(),
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
            percentile_values: summary
                .percentiles
                .iter()
                .map(|(p, v)| (*p, format_number_full(*v)))
                .collect(),
        },
        charts: PayloadCharts {
            data_card: data_card_payload(dataset, range),
            calendar: CalendarPayload {
                cells: calendar_payload_cells(dataset, range, &summary),
                legend_html: crate::charts::legend_markup(&summary),
            },
            commute_casual: CommuteCasualPayload {
                weekdays: weekday_payloads(),
                series: vec![
                    series_payload("commute", "Commute · Smart Card + NCMC", &summary.commute),
                    series_payload("casual", "Casual · Token + QR + Group", &summary.casual),
                ],
                insight: insight_line(&summary),
            },
            weekday_baseline,
            component_waterfall,
            fare_media_mix,
            ncmc_share,
            qr_mix,
            smart_mix,
            weekday_distribution,
            weekend_signatures,
            month_weekday_matrix,
            variability_ranking,
            reporting_pulse,
            monthly_estimates,
            field_completeness,
        },
        presets: range_presets(dataset)
            .iter()
            .map(|preset| PayloadPreset {
                slug: preset.slug,
                label: preset.label,
                start: preset.range.start.to_string(),
                end: preset.range.end.to_string(),
            })
            .collect(),
        definitions: CHARTS
            .iter()
            .map(|definition| ChartDefinitionPayload {
                slug: definition.slug,
                title: definition.title,
                deck: definition.deck,
                method: definition.method,
            })
            .collect(),
    }
}

fn month_weekday_matrix_payload(matrix: &MonthWeekdayMatrix) -> MonthWeekdayMatrixPayload {
    let cells = matrix
        .cells
        .iter()
        .map(|cell| {
            let css_class = if cell.n == 0 || cell.ratio.is_none() {
                "empty"
            } else if cell.ratio.unwrap_or(1.0) >= 1.05 {
                "high"
            } else if cell.ratio.unwrap_or(1.0) <= 0.95 {
                "low"
            } else {
                "neutral"
            }
            .to_string();
            let tooltip = if cell.n == 0 {
                format!(
                    "{} {}: no published totals in the selected range.",
                    cell.month.format("%b %Y"),
                    weekday_name(cell.weekday)
                )
            } else {
                format!(
                    "{} {}: median {}, {:.1}% of the overall {} median, n {}.",
                    cell.month.format("%b %Y"),
                    weekday_name(cell.weekday),
                    cell.median
                        .map(format_number_full)
                        .unwrap_or_else(|| "—".into()),
                    cell.ratio.unwrap_or(0.0) * 100.0,
                    weekday_name(cell.weekday),
                    cell.n
                )
            };
            MonthWeekdayCellPayload {
                month: cell.month.to_string(),
                weekday: weekday_name(cell.weekday),
                weekday_short: weekday_name(cell.weekday).get(..3).unwrap_or(""),
                weekday_index: cell.weekday.num_days_from_monday() as usize,
                n: cell.n,
                median: cell.median,
                median_label: cell.median.map(format_number_full),
                ratio: cell.ratio,
                ratio_label: cell.ratio.map(|ratio| format!("{:.1}%", ratio * 100.0)),
                css_class,
                heat: cell
                    .ratio
                    .map(|ratio| ((ratio - 1.0).abs() / 0.3).min(1.0))
                    .unwrap_or(0.0),
                tooltip,
            }
        })
        .collect();
    MonthWeekdayMatrixPayload {
        months: matrix
            .months
            .iter()
            .map(|month| MonthLabelPayload {
                month: month.to_string(),
                label: month.format("%b ’%y").to_string(),
            })
            .collect(),
        cells,
        insight: month_weekday_matrix_insight(matrix),
    }
}

fn weekend_signatures_payload(groups: &[WeekendSignature]) -> WeekendSignaturesPayload {
    let group_payloads = groups
        .iter()
        .map(|group| {
            let components = crate::data::WATERFALL_CATEGORIES
                .iter()
                .enumerate()
                .map(|(index, (key, label))| {
                    let value = group.components[index];
                    let share = (group.component_total > 0.0)
                        .then(|| value / group.component_total);
                    FareMixComponentPayload {
                        key,
                        label,
                        value,
                        value_label: format_number_full(value),
                        share,
                        share_label: share.map(percent_label),
                        tooltip: format!(
                            "{} · {}: {} ({} of aggregate complete-day total)",
                            group.label,
                            label,
                            format_number_full(value),
                            share.map(percent_label).unwrap_or_else(|| "—".into())
                        ),
                    }
                })
                .collect();
            let tooltip = if group.complete_days == 0 {
                format!(
                    "{}: no complete parent-component days among {} published days.",
                    group.label, group.published_days
                )
            } else {
                format!(
                    "{}: median {}, interquartile range {}–{}, full range {}–{}; {} complete of {} published days.",
                    group.label,
                    group.median.map(format_number_full).unwrap_or_else(|| "—".into()),
                    group.lower_quartile.map(format_number_full).unwrap_or_else(|| "—".into()),
                    group.upper_quartile.map(format_number_full).unwrap_or_else(|| "—".into()),
                    group.minimum.map(format_number_full).unwrap_or_else(|| "—".into()),
                    group.maximum.map(format_number_full).unwrap_or_else(|| "—".into()),
                    group.complete_days,
                    group.published_days
                )
            };
            WeekendSignaturePayload {
                key: group.key,
                label: group.label,
                published_days: group.published_days,
                complete_days: group.complete_days,
                minimum: group.minimum,
                minimum_label: group.minimum.map(format_number_full),
                lower_quartile: group.lower_quartile,
                lower_quartile_label: group.lower_quartile.map(format_number_full),
                median: group.median,
                median_label: group.median.map(format_number_full),
                upper_quartile: group.upper_quartile,
                upper_quartile_label: group.upper_quartile.map(format_number_full),
                maximum: group.maximum,
                maximum_label: group.maximum.map(format_number_full),
                components,
                tooltip,
            }
        })
        .collect();
    WeekendSignaturesPayload {
        groups: group_payloads,
        insight: weekend_signature_insight(groups),
    }
}

fn weekday_distribution_payload(days: &[WeekdayDistribution]) -> WeekdayDistributionPayload {
    let day_payloads = days
        .iter()
        .enumerate()
        .map(|(index, day)| {
            let label = if day.n == 0 {
                format!(
                    "{}: no published totals in the selected range.",
                    weekday_name(day.weekday)
                )
            } else {
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
            };
            WeekdayDistributionDayPayload {
                weekday: weekday_name(day.weekday),
                weekday_short: weekday_name(day.weekday).get(..3).unwrap_or(""),
                index,
                n: day.n,
                minimum: day.minimum,
                minimum_label: day.minimum.map(format_number_full),
                lower_quartile: day.lower_quartile,
                lower_quartile_label: day.lower_quartile.map(format_number_full),
                median: day.median,
                median_label: day.median.map(format_number_full),
                upper_quartile: day.upper_quartile,
                upper_quartile_label: day.upper_quartile.map(format_number_full),
                maximum: day.maximum,
                maximum_label: day.maximum.map(format_number_full),
                tooltip: label,
            }
        })
        .collect();
    WeekdayDistributionPayload {
        days: day_payloads,
        insight: weekday_distribution_insight(days),
    }
}

fn percent_label(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn fare_mix_payload(months: &[FareMixMonth]) -> FareMediaMixPayload {
    let month_payloads = months
        .iter()
        .map(|month| {
            let components = crate::data::WATERFALL_CATEGORIES
                .iter()
                .enumerate()
                .map(|(index, (key, label))| {
                    let value = month.components[index];
                    let share = month
                        .total
                        .filter(|total| *total > 0.0)
                        .map(|total| value / total);
                    FareMixComponentPayload {
                        key,
                        label,
                        value,
                        value_label: format_number_full(value),
                        share,
                        share_label: share.map(percent_label),
                        tooltip: format!(
                            "{} · {}: {} ({}), {} complete of {} published days",
                            month.month.format("%b %Y"),
                            label,
                            format_number_full(value),
                            share.map(percent_label).unwrap_or_else(|| "—".into()),
                            month.complete_days,
                            month.published_days
                        ),
                    }
                })
                .collect();
            FareMixMonthPayload {
                month: month.month.to_string(),
                label: month.month.format("%b ’%y").to_string(),
                published_days: month.published_days,
                complete_days: month.complete_days,
                total: month.total,
                total_label: month.total.map(format_number_full),
                components,
            }
        })
        .collect::<Vec<_>>();
    let usable: Vec<&FareMixMonth> = months
        .iter()
        .filter(|month| month.total.is_some())
        .collect();
    let insight = if usable.len() >= 2 {
        let first = usable.first().expect("nonempty");
        let last = usable.last().expect("nonempty");
        let share_of = |month: &FareMixMonth, index: usize| {
            month
                .total
                .filter(|total| *total > 0.0)
                .map(|total| month.components[index] / total)
                .unwrap_or(0.0)
        };
        format!(
            "Smart Cards {} → {}; NCMC {} → {} across complete observed days.",
            percent_label(share_of(first, 0)),
            percent_label(share_of(last, 0)),
            percent_label(share_of(first, 1)),
            percent_label(share_of(last, 1))
        )
    } else {
        "Not enough complete months in the selected range to compare the mix.".to_string()
    };
    FareMediaMixPayload {
        months: month_payloads,
        insight,
    }
}

fn ncmc_share_payload(weeks: &[NcmcShareWeek]) -> NcmcSharePayload {
    let week_payloads = weeks
        .iter()
        .map(|week| {
            let share = week.share();
            NcmcShareWeekPayload {
                week: week.week.to_string(),
                label: week.week.format("%d %b").to_string(),
                published_days: week.published_days,
                complete_days: week.complete_days,
                ncmc: week.ncmc,
                ncmc_label: format_number_full(week.ncmc),
                commuter_total: week.commuter_total,
                commuter_total_label: format_number_full(week.commuter_total),
                share,
                share_label: share.map(percent_label),
                tooltip: format!(
                    "Week of {}: NCMC {} of Smart Card + NCMC {} ({}), {} complete of {} published days",
                    week.week.format("%d %b %Y"),
                    format_number_full(week.ncmc),
                    format_number_full(week.commuter_total),
                    share.map(percent_label).unwrap_or_else(|| "—".into()),
                    week.complete_days,
                    week.published_days
                ),
            }
        })
        .collect();
    let usable: Vec<&NcmcShareWeek> = weeks.iter().filter(|week| week.share().is_some()).collect();
    let insight = if usable.len() >= 2 {
        let first = usable.first().expect("nonempty");
        let last = usable.last().expect("nonempty");
        format!(
            "NCMC share of Smart Card + NCMC: {} → {} across complete observed days.",
            percent_label(first.share().unwrap_or(0.0)),
            percent_label(last.share().unwrap_or(0.0))
        )
    } else {
        "Not enough complete weeks in the selected range to compare NCMC share.".to_string()
    };
    NcmcSharePayload {
        weeks: week_payloads,
        insight,
    }
}

fn qr_mix_payload(months: &[QrMixMonth]) -> FareMediaMixPayload {
    let month_payloads = months
        .iter()
        .map(|month| {
            let components = crate::data::QR_PROVIDER_CATEGORIES
                .iter()
                .enumerate()
                .map(|(index, (key, label))| {
                    let value = month.providers[index];
                    let share = month.has_mix().then(|| value / month.provider_total);
                    FareMixComponentPayload {
                        key,
                        label,
                        value,
                        value_label: format_number_full(value),
                        share,
                        share_label: share.map(percent_label),
                        tooltip: format!(
                            "{} · {}: {} ({} of provider-sum QR), {} complete of {} published days",
                            month.month.format("%b %Y"),
                            label,
                            format_number_full(value),
                            share.map(percent_label).unwrap_or_else(|| "—".into()),
                            month.complete_days,
                            month.published_days
                        ),
                    }
                })
                .collect();
            FareMixMonthPayload {
                month: month.month.to_string(),
                label: month.month.format("%b ’%y").to_string(),
                published_days: month.published_days,
                complete_days: month.complete_days,
                total: month.has_mix().then_some(month.parent_qr_total),
                total_label: month
                    .has_mix()
                    .then(|| format_number_full(month.parent_qr_total)),
                components,
            }
        })
        .collect::<Vec<_>>();
    let usable: Vec<&QrMixMonth> = months.iter().filter(|month| month.has_mix()).collect();
    let insight = if usable.len() >= 2 {
        let first = usable.first().expect("nonempty");
        let last = usable.last().expect("nonempty");
        let share_of =
            |month: &QrMixMonth, index: usize| month.providers[index] / month.provider_total;
        format!(
            "Namma Metro {} → {}; ONDC {} → {} of provider-sum QR.",
            percent_label(share_of(first, 0)),
            percent_label(share_of(last, 0)),
            percent_label(share_of(first, 3)),
            percent_label(share_of(last, 3))
        )
    } else {
        "Not enough complete months in the selected range to compare QR channels.".to_string()
    };
    FareMediaMixPayload {
        months: month_payloads,
        insight,
    }
}

fn smart_mix_payload(months: &[SmartMixMonth]) -> FareMediaMixPayload {
    let month_payloads = months
        .iter()
        .map(|month| {
            let components = crate::data::SMART_SUBTYPE_CATEGORIES
                .iter()
                .enumerate()
                .map(|(index, (key, label))| {
                    let value = month.subtypes[index];
                    let share = month
                        .has_mix()
                        .then(|| value / month.subtype_total);
                    FareMixComponentPayload {
                        key,
                        label,
                        value,
                        value_label: format_number_full(value),
                        share,
                        share_label: share.map(percent_label),
                        tooltip: format!(
                            "{} · {}: {} ({} of Smart Card subtypes), {} complete of {} published days",
                            month.month.format("%b %Y"),
                            label,
                            format_number_full(value),
                            share.map(percent_label).unwrap_or_else(|| "—".into()),
                            month.complete_days,
                            month.published_days
                        ),
                    }
                })
                .collect();
            FareMixMonthPayload {
                month: month.month.to_string(),
                label: month.month.format("%b ’%y").to_string(),
                published_days: month.published_days,
                complete_days: month.complete_days,
                total: month.has_mix().then_some(month.parent_smart_total),
                total_label: month
                    .has_mix()
                    .then(|| format_number_full(month.parent_smart_total)),
                components,
            }
        })
        .collect::<Vec<_>>();
    let usable: Vec<&SmartMixMonth> = months.iter().filter(|month| month.has_mix()).collect();
    let insight = if usable.len() >= 2 {
        let first = usable.first().expect("nonempty");
        let last = usable.last().expect("nonempty");
        let share_of =
            |month: &SmartMixMonth, index: usize| month.subtypes[index] / month.subtype_total;
        format!(
            "Stored Value {} → {}; One Day Pass {} → {} of Smart Card subtypes.",
            percent_label(share_of(first, 0)),
            percent_label(share_of(last, 0)),
            percent_label(share_of(first, 1)),
            percent_label(share_of(last, 1))
        )
    } else {
        "Not enough complete months in the selected range to compare Smart Card subtypes."
            .to_string()
    };
    FareMediaMixPayload {
        months: month_payloads,
        insight,
    }
}

fn signed_number(value: f64) -> String {
    if value > 0.0 {
        format!("+{}", format_number_full(value))
    } else {
        format_number_full(value)
    }
}

fn component_waterfall_payload(
    waterfall: Option<&ComponentWaterfall>,
    date: NaiveDate,
) -> ComponentWaterfallPayload {
    let date_display = date.format("%A, %-d %B %Y").to_string();
    let Some(waterfall) = waterfall else {
        return ComponentWaterfallPayload {
            available: false,
            message: Some(
                "The selected day does not have complete Smart Card, NCMC, Token, QR, and Group Ticket components."
                    .to_string(),
            ),
            date: date.to_string(),
            date_display,
            weekday: weekday_name(date.weekday()),
            baseline_n: 0,
            reference_total: None,
            reference_total_label: None,
            actual_total: 0.0,
            actual_total_label: "—".to_string(),
            delta_total: None,
            delta_total_label: None,
            components: Vec::new(),
            insight: String::new(),
        };
    };
    let available = waterfall.reference_total.is_some() && !waterfall.components.is_empty();
    let message = (!available).then(|| {
        format!(
            "Only {} nearby same-weekday reports have complete fare-media components; at least {} are required.",
            waterfall.baseline_n, BASELINE_MIN_OBSERVATIONS
        )
    });
    let components = waterfall
        .components
        .iter()
        .map(|component| ComponentDeltaPayload {
            key: component.key,
            label: component.label,
            reference: component.reference,
            reference_label: format_number_full(component.reference),
            actual: component.actual,
            actual_label: format_number_full(component.actual),
            delta: component.delta,
            delta_label: signed_number(component.delta),
            cumulative_start: component.cumulative_start,
            cumulative_end: component.cumulative_end,
            positive: component.delta >= 0.0,
            tooltip: format!(
                "{}: {} actual vs {} reference; delta {}",
                component.label,
                format_number_full(component.actual),
                format_number_full(component.reference),
                signed_number(component.delta)
            ),
        })
        .collect::<Vec<_>>();
    let insight = if available {
        let total_delta = waterfall.delta_total.unwrap_or(0.0);
        let mut contributors = waterfall.components.clone();
        contributors.sort_by(|a, b| b.delta.abs().total_cmp(&a.delta.abs()));
        let top = contributors
            .iter()
            .take(2)
            .map(|component| format!("{} {}", component.label, signed_number(component.delta)))
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "Total difference {}. Largest component movements: {}.",
            signed_number(total_delta),
            top
        )
    } else {
        String::new()
    };
    ComponentWaterfallPayload {
        available,
        message,
        date: waterfall.date.to_string(),
        date_display,
        weekday: weekday_name(waterfall.weekday),
        baseline_n: waterfall.baseline_n,
        reference_total: waterfall.reference_total,
        reference_total_label: waterfall.reference_total.map(format_number_full),
        actual_total: waterfall.actual_total,
        actual_total_label: format_number_full(waterfall.actual_total),
        delta_total: waterfall.delta_total,
        delta_total_label: waterfall.delta_total.map(signed_number),
        components,
        insight,
    }
}

fn weekday_baseline_payload(points: &[WeekdayBaselinePoint]) -> WeekdayBaselinePayload {
    WeekdayBaselinePayload {
        window: BASELINE_WINDOW,
        min_observations: BASELINE_MIN_OBSERVATIONS,
        points: points.iter().map(baseline_point_payload).collect(),
        high_count: points
            .iter()
            .filter(|point| point.status == crate::data::BaselineStatus::High)
            .count(),
        low_count: points
            .iter()
            .filter(|point| point.status == crate::data::BaselineStatus::Low)
            .count(),
        insufficient_count: points
            .iter()
            .filter(|point| point.status == crate::data::BaselineStatus::Insufficient)
            .count(),
        insight: baseline_insight(points),
    }
}

fn baseline_point_payload(point: &WeekdayBaselinePoint) -> WeekdayBaselinePointPayload {
    let delta_pct = point
        .median
        .filter(|median| *median != 0.0)
        .map(|median| (point.total - median) / median * 100.0);
    let weekday_short = match point.weekday {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    };
    let tooltip = match (point.median, point.lower_quartile, point.upper_quartile) {
        (Some(median), Some(lower), Some(upper)) => format!(
            "{}: {} total — {}; median {}; IQR {}–{}; delta {}; n {}",
            point.date.format("%a %-d %b %Y"),
            format_number_full(point.total),
            point.status.label().to_lowercase(),
            format_number_full(median),
            format_number_full(lower),
            format_number_full(upper),
            delta_pct
                .map(|value| format!("{value:+.1}%"))
                .unwrap_or_else(|| "—".to_string()),
            point.baseline_n
        ),
        _ => format!(
            "{}: {} total; baseline unavailable (n {})",
            point.date.format("%a %-d %b %Y"),
            format_number_full(point.total),
            point.baseline_n
        ),
    };
    WeekdayBaselinePointPayload {
        date: point.date.to_string(),
        weekday: weekday_name(point.weekday),
        weekday_short,
        total: point.total,
        total_label: format_number_full(point.total),
        lower_quartile: point.lower_quartile,
        lower_quartile_label: point.lower_quartile.map(format_number_full),
        median: point.median,
        median_label: point.median.map(format_number_full),
        upper_quartile: point.upper_quartile,
        upper_quartile_label: point.upper_quartile.map(format_number_full),
        baseline_n: point.baseline_n,
        delta_pct,
        delta_pct_label: delta_pct.map(|value| format!("{value:+.1}%")),
        status: point.status.as_str(),
        status_label: point.status.label(),
        tooltip,
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
    let mut col = 0usize;
    let mut prev_row: Option<usize> = None;
    let mut last_month = date.month();
    while date <= range.end {
        let record = map.get(&date).copied();
        let value = record.and_then(|record| record.total_ridership);
        let band = value.and_then(|value| band_label(value, summary));
        let breakdown = record.map(fare_media_breakdown).unwrap_or_default();
        let label = calendar_cell_label(date, value, band, &breakdown);
        let week = ((date - first_monday).num_days() / 7) as usize;
        let row = date.weekday().num_days_from_monday() as usize;
        let previous = date - chrono::Duration::days(1);
        let is_new_month = date.month() != previous.month();
        let month_label =
            (date == range.start || is_new_month).then(|| date.format("%b '%y").to_string());
        if date != range.start {
            if date.month() != last_month {
                col += 1;
                last_month = date.month();
            } else if prev_row == Some(6) {
                col += 1;
            }
        }
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
            weekday: row,
            week,
            col,
            month_label,
            month_gap: month_gap_count,
            total: value,
            total_label: value.map(format_number_full),
            band_label: band.map(str::to_string),
            band_index: band.and_then(band_index),
            missing: value.is_none(),
            breakdown,
        });
        prev_row = Some(row);
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
                fare_media_item(
                    "smart-card",
                    "Smart Cards",
                    record.smart_card,
                    total_for_pct,
                    &[
                        ("Stored Value", record.stored_value),
                        ("One Day Pass", record.one_day_pass),
                        ("Three Day Pass", record.three_day_pass),
                        ("Five Day Pass", record.five_day_pass),
                    ],
                ),
                fare_media_item("token", "Tokens", record.token, total_for_pct, &[]),
                fare_media_item(
                    "qr",
                    "QR Tickets",
                    record.qr,
                    total_for_pct,
                    &[
                        ("Namma Metro", record.qr_namma),
                        ("WhatsApp", record.qr_whatsapp),
                        ("Paytm", record.qr_paytm),
                        ("ONDC", record.qr_ondc),
                    ],
                ),
                fare_media_item("ncmc", "NCMC", record.ncmc, total_for_pct, &[]),
                fare_media_item(
                    "group",
                    "Group Ticket",
                    record.group_ticket,
                    total_for_pct,
                    &[],
                ),
            ];
            DataCardPayload {
                date: record.date.to_string(),
                date_display: record.date.format("%A, %-d %B %Y").to_string(),
                total,
                total_label: total
                    .map(format_number_full)
                    .unwrap_or_else(|| "—".to_string()),
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
        value_label: value
            .map(format_number_full)
            .unwrap_or_else(|| "—".to_string()),
        percentage,
        breakdown,
    }
}
