use std::sync::Arc;

use chrono::{Datelike, NaiveDate, Weekday};
use reqwest::Client;
use tokio::sync::RwLock;

use crate::charts::{
    Chart, band_label, calendar_markup, format_count, line_chart_markup, reporting_pulse_markup,
};
use crate::client::CLIENT_SCRIPT;
use crate::data::{
    BaselineStatus, Dataset, DateRange, RangeSummary, ReportingPulseMonth, choose_range,
    clean_number, component_waterfall, fare_mix_series, field_completeness_grid,
    month_weekday_matrix, monthly_estimate_series, ncmc_share_series, parse_dataset, qr_mix_series,
    quantile, reporting_pulse_series, smart_mix_series, summarise, variability_ranking,
    weekday_baseline_series, weekday_distribution_series, weekend_signature_series,
};
use crate::payload::chart_payload_for;
use crate::render::{chart_jump_markup, parse_query, range_action_links};
use crate::server::{AppState, constant_time_eq};
use crate::style::STYLE;

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
fn weekday_baseline_uses_nearby_same_weekday_reports_but_not_the_current_day() {
    let mut csv = String::from(
        "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR\n",
    );
    // Seven Mondays in early 2024. Smart-card values are total minus 40
    // because the four other parent categories contribute 10 each.
    let first_monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    for (week, smart_card) in [60.0, 70.0, 80.0, 160.0, 90.0, 100.0, 110.0]
        .iter()
        .enumerate()
    {
        let date = first_monday + chrono::Duration::days(week as i64 * 7);
        csv.push_str(&format!(
            "{},{smart_card},10,10,10,10\n",
            date.format("%d-%m-%Y")
        ));
    }
    let dataset = parse_dataset(&csv, "fixture".into()).unwrap();
    let range = DateRange {
        start: first_monday,
        end: first_monday + chrono::Duration::days(42),
    };
    let points = weekday_baseline_series(&dataset, range);
    assert_eq!(points.len(), 7);
    let target = &points[3];
    assert_eq!(target.weekday, Weekday::Mon);
    assert_eq!(target.total, 200.0);
    assert_eq!(target.baseline_n, 6);
    // Neighbouring reports are 100, 110, 120, 130, 140, and 150. Including
    // the current 200 would shift the median to 130, so 125 proves exclusion.
    assert_eq!(target.median, Some(125.0));
    assert_eq!(target.lower_quartile, Some(112.5));
    assert_eq!(target.upper_quartile, Some(137.5));
    assert_eq!(target.status, BaselineStatus::High);
}

#[test]
fn audit_and_estimation_series_preserve_missing_semantics() {
    let csv = "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR\n\
01-01-2024,40,10,20,10,20\n\
03-01-2024,80,20,40,20,40\n";
    let dataset = parse_dataset(csv, "fixture".into()).unwrap();
    let range = DateRange {
        start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end: NaiveDate::from_ymd_opt(2024, 1, 7).unwrap(),
    };
    let variability = variability_ranking(&dataset, range);
    assert_eq!(variability.len(), 5);
    assert!(variability.iter().all(|row| row.n == 2));
    let pulse = reporting_pulse_series(&dataset, range);
    assert_eq!(pulse[0].calendar_days, 7);
    assert_eq!(pulse[0].observed_days, 2);
    assert_eq!(pulse[0].missing_days, 5);
    let estimates = monthly_estimate_series(&dataset, range);
    assert_eq!(estimates[0].observed_total, Some(300.0));
    assert_eq!(estimates[0].missing_days, 5);
    assert!(estimates[0].estimated_missing_total.is_some());
    let fields = field_completeness_grid(&dataset, range);
    assert_eq!(fields[0].published_days, 2);
    assert_eq!(fields[0].total_derivable_days, 2);
    assert_eq!(
        fields[0]
            .cells
            .iter()
            .find(|cell| cell.key == "smart-card")
            .unwrap()
            .present_days,
        2
    );
    assert_eq!(
        fields[0]
            .cells
            .iter()
            .find(|cell| cell.key == "stored-value")
            .unwrap()
            .present_days,
        0
    );
}

#[test]
fn monthly_estimates_fill_only_unpublished_dates_and_preserve_zero_observations() {
    let csv = "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR
01-01-2024,40,10,20,10,20
02-01-2024,,,,,
01-02-2024,120,30,60,30,60
02-02-2024,,,,,
";
    let dataset = parse_dataset(csv, "fixture".into()).unwrap();
    let estimates = monthly_estimate_series(
        &dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
        },
    );
    assert_eq!(estimates.len(), 2);
    let january = &estimates[0];
    assert_eq!(january.observed_total, Some(100.0));
    assert_eq!(january.observed_days, 1);
    assert_eq!(january.published_without_total_days, 1);
    assert_eq!(january.missing_days, 29);
    assert_eq!(january.estimated_missing_total, Some(2900.0));
    let february = &estimates[1];
    assert_eq!(february.observed_total, Some(300.0));
    assert_eq!(february.observed_days, 1);
    assert_eq!(february.published_without_total_days, 1);
    assert_eq!(february.missing_days, 27);
    assert_eq!(february.estimated_missing_total, Some(8100.0));

    let zero_csv = "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR
01-03-2024,0,0,0,0,0
";
    let zero_dataset = parse_dataset(zero_csv, "fixture".into()).unwrap();
    let zero_estimates = monthly_estimate_series(
        &zero_dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 3, 31).unwrap(),
        },
    );
    assert_eq!(zero_estimates[0].observed_total, Some(0.0));
    assert_eq!(zero_estimates[0].published_without_total_days, 0);
}

#[test]
fn month_weekday_matrix_reports_ratios_and_sample_sizes() {
    let csv = "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR\n\
01-01-2024,40,10,20,10,20\n\
05-02-2024,80,20,40,20,40\n";
    let dataset = parse_dataset(csv, "fixture".into()).unwrap();
    let matrix = month_weekday_matrix(
        &dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
        },
    );
    assert_eq!(matrix.months.len(), 2);
    assert_eq!(matrix.cells.len(), 14);
    let jan_mon = matrix
        .cells
        .iter()
        .find(|cell| cell.month.month() == 1 && cell.weekday == chrono::Weekday::Mon)
        .unwrap();
    assert_eq!(jan_mon.n, 1);
    assert_eq!(jan_mon.median, Some(100.0));
    assert_eq!(jan_mon.ratio, Some(100.0 / 150.0));
    let feb_mon = matrix
        .cells
        .iter()
        .find(|cell| cell.month.month() == 2 && cell.weekday == chrono::Weekday::Mon)
        .unwrap();
    assert_eq!(feb_mon.n, 1);
    assert_eq!(feb_mon.median, Some(200.0));
    assert_eq!(feb_mon.ratio, Some(200.0 / 150.0));
}

#[test]
fn weekend_signatures_use_complete_parent_days_for_distribution_and_mix() {
    let csv = "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR\n\
05-01-2024,50,10,20,10,10\n\
06-01-2024,30,10,10,0,10\n\
07-01-2024,20,5,10,0,5\n\
08-01-2024,50,,20,10,10\n";
    let dataset = parse_dataset(csv, "fixture".into()).unwrap();
    let groups = weekend_signature_series(
        &dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 1, 5).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 1, 8).unwrap(),
        },
    );
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].published_days, 2);
    assert_eq!(groups[0].complete_days, 1);
    assert_eq!(groups[0].median, Some(100.0));
    assert_eq!(groups[0].components, [50.0, 10.0, 20.0, 10.0, 10.0]);
    assert_eq!(groups[1].complete_days, 1);
    assert_eq!(groups[1].median, Some(60.0));
    assert_eq!(groups[2].median, Some(40.0));
}

#[test]
fn weekday_distribution_uses_observed_totals_without_zero_fill() {
    let csv = "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR\n\
01-01-2024,40,10,20,10,20\n\
02-01-2024,80,20,40,20,40\n\
08-01-2024,80,20,40,20,40\n\
09-01-2024,40,10,,10,20\n";
    let dataset = parse_dataset(csv, "fixture".into()).unwrap();
    let days = weekday_distribution_series(
        &dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 1, 14).unwrap(),
        },
    );
    assert_eq!(days.len(), 7);
    assert_eq!(days[0].n, 2);
    assert_eq!(days[0].minimum, Some(100.0));
    assert_eq!(days[0].median, Some(150.0));
    assert_eq!(days[0].maximum, Some(200.0));
    assert_eq!(days[1].n, 1);
    assert_eq!(days[1].median, Some(200.0));
    assert_eq!(days[2].n, 0);
    assert_eq!(days[2].median, None);
}

#[test]
fn smart_mix_uses_complete_subtype_days_and_keeps_parent_total_separate() {
    let csv = "Record Date,Total Smart Cards,StoredValueCard,OneDayPass,ThreeDayPass,FiveDayPass\n\
01-01-2024,100,80,10,5,5\n\
02-01-2024,100,90,,5,5\n\
01-02-2024,200,180,12,4,4\n";
    let dataset = parse_dataset(csv, "fixture".into()).unwrap();
    let months = smart_mix_series(
        &dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
        },
    );
    assert_eq!(months.len(), 2);
    assert_eq!(months[0].published_days, 2);
    assert_eq!(months[0].complete_days, 1);
    assert_eq!(months[0].subtypes, [80.0, 10.0, 5.0, 5.0]);
    assert_eq!(months[0].subtype_total, 100.0);
    assert_eq!(months[0].parent_smart_total, 100.0);
    assert!(months[0].has_mix());
    assert_eq!(months[1].subtypes, [180.0, 12.0, 4.0, 4.0]);
}

#[test]
fn parent_mix_totals_track_partial_coverage() {
    let smart_csv =
        "Record Date,Total Smart Cards,StoredValueCard,OneDayPass,ThreeDayPass,FiveDayPass
01-01-2024,100,80,10,5,5
02-01-2024,,90,20,5,5
";
    let smart_dataset = parse_dataset(smart_csv, "fixture".into()).unwrap();
    let smart_months = smart_mix_series(
        &smart_dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
        },
    );
    assert_eq!(smart_months[0].complete_days, 2);
    assert_eq!(smart_months[0].parent_smart_days, 1);
    assert_eq!(smart_months[0].parent_smart_total, 100.0);

    let qr_csv = "Record Date,Total QR,QR Namma Metro,QR WhatsApp,QR Paytm,QR ONDC
01-01-2024,100,40,30,10,10
02-01-2024,,50,20,10,10
";
    let qr_dataset = parse_dataset(qr_csv, "fixture".into()).unwrap();
    let qr_months = qr_mix_series(
        &qr_dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
        },
    );
    assert_eq!(qr_months[0].complete_days, 2);
    assert_eq!(qr_months[0].parent_qr_days, 1);
    assert_eq!(qr_months[0].parent_qr_total, 100.0);
}

#[test]
fn qr_mix_uses_complete_provider_days_and_keeps_parent_total_separate() {
    let csv = "Record Date,Total QR,QR Namma Metro,QR WhatsApp,QR Paytm,QR ONDC\n\
01-01-2024,100,40,30,10,10\n\
02-01-2024,100,50,,10,10\n\
01-02-2024,200,80,40,20,60\n";
    let dataset = parse_dataset(csv, "fixture".into()).unwrap();
    let months = qr_mix_series(
        &dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
        },
    );
    assert_eq!(months.len(), 2);
    assert_eq!(months[0].published_days, 2);
    assert_eq!(months[0].complete_days, 1);
    assert_eq!(months[0].providers, [40.0, 30.0, 10.0, 10.0]);
    assert_eq!(months[0].provider_total, 90.0);
    assert_eq!(months[0].parent_qr_total, 100.0);
    assert!(months[0].has_mix());
    assert_eq!(months[1].providers, [80.0, 40.0, 20.0, 60.0]);
}

#[test]
fn ncmc_share_weeks_use_only_days_with_both_components() {
    let csv = "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR\n\
01-01-2024,100,20,10,2,30\n\
02-01-2024,100,,10,2,30\n\
08-01-2024,100,60,10,2,30\n";
    let dataset = parse_dataset(csv, "fixture".into()).unwrap();
    let weeks = ncmc_share_series(
        &dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 1, 14).unwrap(),
        },
    );
    assert_eq!(weeks.len(), 2);
    assert_eq!(weeks[0].published_days, 2);
    assert_eq!(weeks[0].complete_days, 1);
    assert_eq!(weeks[0].ncmc, 20.0);
    assert_eq!(weeks[0].commuter_total, 120.0);
    assert_eq!(weeks[0].share(), Some(20.0 / 120.0));
    assert_eq!(weeks[1].complete_days, 1);
    assert_eq!(weeks[1].share(), Some(60.0 / 160.0));
}

#[test]
fn fare_mix_months_aggregate_complete_days_and_preserve_gaps() {
    let csv = "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR\n\
01-01-2024,100,20,30,5,45\n\
02-01-2024,50,10,,2,20\n\
01-03-2024,200,40,60,10,90\n";
    let dataset = parse_dataset(csv, "fixture".into()).unwrap();
    let months = fare_mix_series(
        &dataset,
        DateRange {
            start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 3, 31).unwrap(),
        },
    );
    assert_eq!(months.len(), 3);
    assert_eq!(months[0].published_days, 2);
    assert_eq!(months[0].complete_days, 1);
    assert_eq!(months[0].total, Some(200.0));
    assert_eq!(months[0].components, [100.0, 20.0, 30.0, 45.0, 5.0]);
    assert_eq!(months[1].published_days, 0);
    assert_eq!(months[1].complete_days, 0);
    assert_eq!(months[1].total, None);
    assert_eq!(months[2].published_days, 1);
    assert_eq!(months[2].complete_days, 1);
    assert_eq!(months[2].total, Some(400.0));
    assert_eq!(months[2].components, [200.0, 40.0, 60.0, 90.0, 10.0]);
}

#[test]
fn component_waterfall_deltas_decompose_the_selected_day_exactly() {
    let mut csv = String::from(
        "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR\n",
    );
    let first_monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    for (week, smart_card) in [60.0, 70.0, 80.0, 160.0, 90.0, 100.0, 110.0]
        .iter()
        .enumerate()
    {
        let date = first_monday + chrono::Duration::days(week as i64 * 7);
        csv.push_str(&format!(
            "{},{smart_card},10,10,10,10\n",
            date.format("%d-%m-%Y")
        ));
    }
    let dataset = parse_dataset(&csv, "fixture".into()).unwrap();
    let selected = first_monday + chrono::Duration::days(21);
    let waterfall = component_waterfall(&dataset, selected).unwrap();

    assert_eq!(waterfall.baseline_n, 6);
    assert_eq!(waterfall.reference_total, Some(125.0));
    assert_eq!(waterfall.actual_total, 200.0);
    assert_eq!(waterfall.delta_total, Some(75.0));
    let smart_card = waterfall
        .components
        .iter()
        .find(|component| component.key == "smart-card")
        .unwrap();
    assert_eq!(smart_card.reference, 85.0);
    assert_eq!(smart_card.actual, 160.0);
    assert_eq!(smart_card.delta, 75.0);
    assert_eq!(smart_card.cumulative_start, 125.0);
    assert_eq!(smart_card.cumulative_end, 200.0);
    assert!(
        waterfall
            .components
            .iter()
            .skip(1)
            .all(|component| component.delta == 0.0)
    );
}

#[test]
fn component_waterfall_uses_published_parent_totals_when_provider_sums_diverge() {
    let mut csv = String::from(
        "Record Date,Total Smart Cards,Total NCMC,Total Tokens,Group Ticket,Total QR,QR Namma Metro,QR WhatsApp,QR Paytm,QR ONDC\n",
    );
    let first_monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    for week in 0..5 {
        let date = first_monday + chrono::Duration::days(week * 7);
        // Parent QR is 100 while its complete provider split sums to 90.
        csv.push_str(&format!(
            "{},1000,100,200,10,100,40,30,10,10\n",
            date.format("%d-%m-%Y")
        ));
    }
    let dataset = parse_dataset(&csv, "fixture".into()).unwrap();
    let selected = first_monday + chrono::Duration::days(14);
    let record = dataset
        .records
        .iter()
        .find(|record| record.date == selected)
        .unwrap();
    assert_eq!(
        record.qr,
        Some(90.0),
        "normalisation prefers the complete provider split"
    );
    let waterfall = component_waterfall(&dataset, selected).unwrap();
    let qr = waterfall
        .components
        .iter()
        .find(|component| component.key == "qr")
        .unwrap();
    assert_eq!(qr.actual, 100.0);
    assert_eq!(waterfall.actual_total, 1410.0);
}

#[test]
fn component_waterfall_preserves_missing_component_semantics() {
    let dataset = fixture_dataset();
    let missing_group_ticket_day = NaiveDate::from_ymd_opt(2026, 1, 3).unwrap();
    assert!(
        component_waterfall(&dataset, missing_group_ticket_day).is_none(),
        "incomplete selected-day components should make the waterfall unavailable"
    );
}

#[test]
fn query_parser_handles_malformed_values_without_panicking() {
    let query = parse_query("start=nope&end=2026-01-03&chart=commute-casual&ignored=yes");
    assert_eq!(query.start.as_deref(), Some("nope"));
    assert_eq!(query.chart.as_deref(), Some("commute-casual"));
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
fn range_actions_render_clean_href_with_data_attrs() {
    let dataset = fixture_dataset();
    let range = choose_range(&dataset, None, None);
    let actions = range_action_links(&dataset, Chart::Calendar, &range);
    assert!(actions.contains(r#"href="/""#));
    assert!(actions.contains(r#"data-start="2026-01-01""#));
    assert!(actions.contains(r#"data-end="2026-01-20""#));
    assert!(actions.contains(r#"data-chart="calendar""#));
}

#[test]
fn chart_jump_lists_every_registered_chart_and_marks_current() {
    let markup = chart_jump_markup(Chart::Calendar);
    assert!(markup.contains(r#"class="chart-jump""#));
    assert!(markup.contains(r#"aria-label="Jump to chart""#));
    assert!(markup.contains(r#"<option value="data-card">Today's Ridership</option>"#));
    assert!(markup.contains(r#"<option value="calendar" selected>Daily Total Ridership</option>"#));
    assert!(
        markup.contains(r#"<option value="commute-casual">Commute vs Casual by Weekday</option>"#)
    );
    assert!(
        markup.contains(r#"<option value="weekday-baseline">Unusual for its Weekday?</option>"#)
    );
    assert!(markup.contains(
        r#"<option value="component-waterfall">What Accounts for the Difference?</option>"#
    ));
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
    assert!(markup.contains("calendar-cell missing"));
    assert!(markup.contains("Thursday, 29 February 2024: Missing data"));
    assert!(markup.contains("Feb '24"));
    assert!(markup.contains("Mar '24"));
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
    assert!(
        payload
            .charts
            .calendar
            .legend_html
            .contains(r#"class="legend""#)
    );
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
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("weekdayBaseline")
    );
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("componentWaterfall")
    );
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("fareMediaMix")
    );
    assert_eq!(payload.charts.fare_media_mix.months.len(), 1);
    assert_eq!(payload.charts.fare_media_mix.months[0].components.len(), 5);
    assert!(payload.charts.fare_media_mix.months[0].total.is_some());
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("ncmcShare")
    );
    assert_eq!(payload.charts.ncmc_share.weeks.len(), 4);
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("smartMix")
    );
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("weekdayDistribution")
    );
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("weekendSignatures")
    );
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("monthWeekdayMatrix")
    );
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("variabilityRanking")
    );
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("reportingPulse")
    );
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("monthlyEstimates")
    );
    assert!(
        serde_json::to_string(&payload)
            .unwrap()
            .contains("fieldCompleteness")
    );
    assert!(!payload.charts.component_waterfall.available);
    assert_eq!(payload.charts.weekday_baseline.window, 12);
    assert_eq!(
        payload.charts.weekday_baseline.points.len(),
        dataset
            .records
            .iter()
            .filter(|record| record.total_ridership.is_some())
            .count()
    );
}

#[test]
fn text_date_controls_snap_and_recover_when_cleared() {
    assert!(CLIENT_SCRIPT.contains("getPickerValue(startPicker)"));
    assert!(CLIENT_SCRIPT.contains("getPickerValue(endPicker)"));
    assert!(CLIENT_SCRIPT.contains("/api/chart"));
    assert!(CLIENT_SCRIPT.contains("D3.select"));
    assert!(CLIENT_SCRIPT.contains("GSAP.fromTo"));
}

#[test]
fn client_script_supports_direct_chart_selection() {
    assert!(CLIENT_SCRIPT.contains("const chartJump"));
    assert!(CLIENT_SCRIPT.contains("chartJump?.addEventListener('change'"));
    assert!(CLIENT_SCRIPT.contains("chartJump.value = activeChart"));
}

#[test]
fn charts_are_configured_to_fill_available_width() {
    assert!(STYLE.contains(".data-card-wrap{width:100%}"));
    assert!(STYLE.contains(
        ".d3-line-svg,.d3-baseline-svg,.d3-waterfall-svg,.d3-mix-svg,.d3-ncmc-svg,.d3-qr-svg,.d3-smart-svg,.d3-dist-svg,.d3-signature-svg{display:block;width:100%"
    ));
    assert!(CLIENT_SCRIPT.contains("window.addEventListener('resize'"));
}

#[test]
fn calendar_cells_are_always_square() {
    // Square cells: one shared `cell` size drives both rect dimensions, the
    // missing-cross diagonals, and the vertical pitch — width and height must
    // never be independent variables.
    assert!(!CLIENT_SCRIPT.contains("cellWidth"));
    assert!(!CLIENT_SCRIPT.contains("cellHeight"));
    assert!(CLIENT_SCRIPT.contains("const cell = 22"));
    assert!(CLIENT_SCRIPT.contains(".attr('width', cell).attr('height', cell)"));
    assert!(CLIENT_SCRIPT.contains("d.weekday * (cell + gap)"));
    assert!(CLIENT_SCRIPT.contains("7 * cell + 6 * gap"));
}

#[test]
fn line_chart_insets_edge_points_and_uses_translucent_fill() {
    let dataset = fixture_dataset();
    let range = DateRange {
        start: dataset.min_date,
        end: dataset.max_date,
    };
    let summary = summarise(&dataset, range);
    let markup = line_chart_markup(&summary);
    assert!(markup.contains(r#"x="84.0""#));
    assert!(markup.contains(r#"x="872.0""#));
    assert!(CLIENT_SCRIPT.contains("xInset = 18"));
    assert!(CLIENT_SCRIPT.contains("right = 10"));
    assert!(CLIENT_SCRIPT.contains("viewport?.width"));
    assert!(CLIENT_SCRIPT.contains("height * Math.max"));
    assert!(CLIENT_SCRIPT.contains("renderLine(false)"));
    assert!(CLIENT_SCRIPT.contains("left + xInset"));
    assert!(CLIENT_SCRIPT.contains("left + plotWidth - xInset"));
    assert!(STYLE.contains(".series.commute,.series.casual{fill-opacity:.18}"));
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
    let response = crate::server::app_router(state).handle(request).await;
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
    let response = crate::server::app_router(state).handle(request).await;
    let status = response.status();
    let body = topcoat::router::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&body);
    assert!(status.is_success() || status == topcoat::router::StatusCode::OK);
    assert!(html.contains("The source data could not be loaded."));
}

#[tokio::test]
async fn home_page_links_code_and_data_sources() {
    let request = topcoat::router::Request::builder()
        .uri("/")
        .body(topcoat::router::Body::empty())
        .unwrap();
    let state = AppState {
        dataset: Arc::new(RwLock::new(Some(fixture_dataset()))),
        client: Client::new(),
    };
    let response = crate::server::app_router(state).handle(request).await;
    assert!(response.status().is_success());
    let body = topcoat::router::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains(r#"class="source-links""#));
    assert!(html.contains(r#"<title>GitHub</title>"#));
    assert!(html.contains(r#"href="https://github.com/thecont1/metro-dash""#));
    assert!(html.contains(r#"href="https://github.com/thecont1/namma-metro-ridership-tracker""#));
    assert!(html.contains(r#"<span class="source-prefix">Source</span>"#));
    assert!(!html.contains("Source data on GitHub ↗"));
}

#[test]
fn constant_time_eq_matches_plain_equality_across_edge_cases() {
    // Boundaries: empty, single byte, differing lengths, and a
    // difference in the first vs. last position — the two cases a
    // short-circuiting comparison would treat very differently.
    let cases: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (b"", b"a"),
        (b"a", b""),
        (b"a", b"a"),
        (b"a", b"b"),
        (b"secret", b"secret"),
        (b"secret", b"Secret"),  // differs at byte 0
        (b"secret", b"secreT"),  // differs at the last byte
        (b"secret", b"secre"),   // prefix, shorter
        (b"secret", b"secrets"), // prefix, longer
        (b"\x00", b"\x00"),
        (b"\xff\x00", b"\x00\xff"), // same XOR-sum, different bytes
    ];
    for (a, b) in cases {
        assert_eq!(
            constant_time_eq(a, b),
            a == b,
            "constant_time_eq disagreed with == for {a:?} vs {b:?}"
        );
    }
}

#[tokio::test]
async fn every_response_carries_the_security_headers() {
    // Cover both branches of the layer: a normal render AND an error
    // response. The first version of this layer used `?`, which let the
    // router build error responses *after* the layer had returned —
    // every 404/500 went out bare. Only the error cases catch that.
    let state = AppState {
        dataset: Arc::new(RwLock::new(None)),
        client: Client::new(),
    };
    let router = crate::server::app_router(state);

    // "/"          -> 200 failure view (handler returns Ok)
    // "/api/chart" -> 500 (handler returns Err: no dataset)
    // "/nope"      -> 404 (no route matches at all)
    for uri in ["/", "/api/chart", "/nope"] {
        let request = topcoat::router::Request::builder()
            .uri(uri)
            .body(topcoat::router::Body::empty())
            .unwrap();
        let response = router.handle(request).await;
        let status = response.status();
        let headers = response.headers();
        for name in [
            "strict-transport-security",
            "x-content-type-options",
            "x-frame-options",
            "referrer-policy",
            "permissions-policy",
            "content-security-policy",
        ] {
            assert!(
                headers.contains_key(name),
                "missing header {name} on {uri} (status {status})"
            );
        }
        assert_eq!(headers["x-content-type-options"], "nosniff", "on {uri}");
        assert_eq!(headers["x-frame-options"], "DENY", "on {uri}");
        let csp = headers["content-security-policy"].to_str().unwrap();
        assert!(csp.contains("frame-ancestors 'none'"), "on {uri}");
        // The dashboard's charts are drawn by d3 and animated by gsap,
        // both loaded as classic <script src> tags from jsdelivr. If
        // these two URLs ever fall out of the CSP the charts stop
        // rendering in the browser while every test here still passes —
        // so assert them explicitly.
        assert!(
            csp.contains("https://cdn.jsdelivr.net/npm/d3@7.9.0/dist/d3.min.js"),
            "CSP would block d3 on {uri}"
        );
        assert!(
            csp.contains("https://cdn.jsdelivr.net/npm/gsap@3.12.5/dist/gsap.min.js"),
            "CSP would block gsap on {uri}"
        );
        assert!(csp.contains("object-src 'none'"), "on {uri}");
    }
}

#[test]
fn format_count_rounds_the_complete_value_before_grouping() {
    assert_eq!(format_count(1000.0), "1,000");
    assert_eq!(format_count(1000.5), "1,000.5");
    assert_eq!(format_count(10.96), "11");
    assert_eq!(format_count(-10.96), "-11");
    assert_eq!(format_count(0.49), "0.5");
    assert_eq!(format_count(-0.49), "-0.5");
}

#[test]
fn reporting_pulse_markup_binds_missing_width_and_counts_in_order() {
    let month = ReportingPulseMonth {
        month: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        calendar_days: 6,
        observed_days: 3,
        missing_days: 3,
        coverage: 0.5,
        longest_missing_run: 2,
    };
    let html = reporting_pulse_markup(&[month]);
    assert!(html.contains("width:50.0%"));
    assert!(html.contains("3/6d · gap 2"));
}

#[test]
fn chart_payloads_include_accessible_tables_for_d3_charts() {
    let dataset = fixture_dataset();
    let payload = chart_payload_for(
        &dataset,
        DateRange {
            start: dataset.min_date,
            end: dataset.max_date,
        },
    );
    let tables = [
        &payload.charts.weekday_baseline.table_html,
        &payload.charts.component_waterfall.table_html,
        &payload.charts.fare_media_mix.table_html,
        &payload.charts.ncmc_share.table_html,
        &payload.charts.qr_mix.table_html,
        &payload.charts.smart_mix.table_html,
        &payload.charts.weekday_distribution.table_html,
        &payload.charts.weekend_signatures.table_html,
        &payload.charts.month_weekday_matrix.table_html,
    ];
    assert!(tables.iter().all(|html| html.contains("accessible-data")));
    let serialized = serde_json::to_string(&payload).unwrap();
    assert!(serialized.contains("tableHtml"));
    assert!(CLIENT_SCRIPT.contains("syncAccessibleTable(accessibleTableForChart(activeChart))"));
    assert!(
        CLIENT_SCRIPT
            .contains(".matrix-scroll,.d3-mix-scene,.d3-ncmc-scene,.d3-qr-scene,.d3-smart-scene")
    );
}
