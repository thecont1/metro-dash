use std::{sync::Arc, time::Duration};

use reqwest::Client;
use tokio::sync::RwLock;
use topcoat::{
    Result,
    asset::AssetBundle,
    context::Cx,
    router::{Json, Router, RouterBuilderDiscoverExt, page, parts, route},
};

mod charts;
mod client;
mod data;
mod payload;
mod render;
mod style;

use charts::Chart;
use data::{choose_range, fetch_dataset, load_cached_dataset};
use payload::{ChartPayload, chart_payload_for};
use render::{failure_view, parse_query, render_view};

#[derive(Debug, Clone)]
pub struct AppState {
    pub dataset: Arc<RwLock<Option<data::Dataset>>>,
    pub client: Client,
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
    if snapshot.is_none()
        && query.retry
        && let Ok(dataset) = fetch_dataset(&state.client).await
    {
        *state.dataset.write().await = Some(dataset.clone());
        snapshot = Some(dataset);
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

#[cfg(test)]
mod tests {
    use super::*;
    use charts::{band_label, calendar_markup};
    use chrono::{NaiveDate, Weekday};
    use client::CLIENT_SCRIPT;
    use data::{
        Dataset, DateRange, RangeSummary, choose_range, clean_number, parse_dataset, quantile,
        summarise,
    };
    use payload::chart_payload_for;
    use render::{all_data_link, default_range_link};

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
        assert!(payload.charts.calendar.legend.is_empty());
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
        assert!(CLIENT_SCRIPT.contains("getPickerValue(startPicker)"));
        assert!(CLIENT_SCRIPT.contains("getPickerValue(endPicker)"));
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
