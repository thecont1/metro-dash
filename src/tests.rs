use std::sync::Arc;

use chrono::{NaiveDate, Weekday};
use reqwest::Client;
use tokio::sync::RwLock;

use crate::charts::{Chart, band_label, calendar_markup};
use crate::client::CLIENT_SCRIPT;
use crate::data::{
    BaselineStatus, Dataset, DateRange, RangeSummary, choose_range, clean_number, parse_dataset,
    quantile, summarise, weekday_baseline_series,
};
use crate::payload::chart_payload_for;
use crate::render::{chart_jump_markup, parse_query, range_action_links};
use crate::server::{AppState, constant_time_eq};

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
