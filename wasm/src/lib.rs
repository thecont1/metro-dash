//! WebAssembly compute core for the static metro-dash site.
//!
//! The parsed [`Dataset`] lives in wasm linear memory between calls: the
//! client fetches the public CSV once per page load, hands the text to
//! [`load_dataset`], and every subsequent range change is a synchronous
//! [`chart_payload`] call — no server round-trip, no duplicated logic. The
//! payload shape is identical to what `GET /api/chart` returns, so the
//! existing client renderer consumes it unchanged.
//!
//! The `*_inner` functions are plain Rust so they can be unit-tested on the
//! host; the `#[wasm_bindgen]` wrappers only translate errors to JS.

use std::cell::RefCell;

use metro_dash::{
    data::{Dataset, choose_range, parse_dataset},
    payload::{ChartPayload, chart_payload_for},
};
use serde_json::json;
use wasm_bindgen::prelude::*;

thread_local! {
    static DATASET: RefCell<Option<Dataset>> = const { RefCell::new(None) };
}

fn nonempty(value: &str) -> Option<&str> {
    if value.is_empty() { None } else { Some(value) }
}

fn dataset_info_json(dataset: &Dataset) -> String {
    json!({
        "minDate": dataset.min_date.to_string(),
        "maxDate": dataset.max_date.to_string(),
        "rowCount": dataset.records.len(),
        "availableDates": dataset
            .records
            .iter()
            .map(|record| record.date.to_string())
            .collect::<Vec<_>>(),
        "refreshedAt": dataset.refreshed_at.as_str(),
        "invalidDateCount": dataset.invalid_date_count,
        "duplicateCount": dataset.duplicate_count,
    })
    .to_string()
}

fn load_dataset_inner(csv_text: &str, refreshed_at: &str) -> Result<String, String> {
    let dataset = parse_dataset(csv_text, refreshed_at.to_string())?;
    let info = dataset_info_json(&dataset);
    DATASET.with(|cell| *cell.borrow_mut() = Some(dataset));
    Ok(info)
}

fn dataset_info_inner() -> Result<String, String> {
    DATASET.with(|cell| {
        let borrow = cell.borrow();
        let dataset = borrow.as_ref().ok_or("dataset not loaded")?;
        Ok(dataset_info_json(dataset))
    })
}

fn chart_payload_inner(start: &str, end: &str) -> Result<String, String> {
    DATASET.with(|cell| {
        let borrow = cell.borrow();
        let dataset = borrow.as_ref().ok_or("dataset not loaded")?;
        let range = choose_range(dataset, nonempty(start), nonempty(end));
        let payload: ChartPayload = chart_payload_for(dataset, range);
        serde_json::to_string(&payload).map_err(|error| error.to_string())
    })
}

/// Parse the ridership CSV and make it the active dataset.
///
/// `refreshed_at` is an RFC 3339 timestamp supplied by the caller (the
/// browser's fetch time) because wasm has no clock of its own. Returns a
/// JSON object describing the dataset (`minDate`, `maxDate`, `rowCount`,
/// `availableDates`, `refreshedAt`, `invalidDateCount`, `duplicateCount`).
#[wasm_bindgen(js_name = loadDataset)]
pub fn load_dataset(csv_text: &str, refreshed_at: &str) -> Result<String, JsValue> {
    load_dataset_inner(csv_text, refreshed_at).map_err(|error| js_sys::Error::new(&error).into())
}

/// Describe the currently loaded dataset, or throw if none is loaded.
#[wasm_bindgen(js_name = datasetInfo)]
pub fn dataset_info() -> Result<String, JsValue> {
    dataset_info_inner().map_err(|error| js_sys::Error::new(&error).into())
}

/// Compute the chart payload for a date range.
///
/// `start`/`end` are `YYYY-MM-DD` strings; empty strings select the default
/// range, matching the server's `choose_range` semantics. Returns the same
/// JSON `ChartPayload` that `GET /api/chart` produces.
#[wasm_bindgen(js_name = chartPayload)]
pub fn chart_payload(start: &str, end: &str) -> Result<String, JsValue> {
    chart_payload_inner(start, end).map_err(|error| js_sys::Error::new(&error).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/ridership_fixture.csv");

    #[test]
    fn load_then_payload_matches_api_shape() {
        let info = load_dataset_inner(FIXTURE, "2026-01-01T00:00:00Z").unwrap();
        let info: serde_json::Value = serde_json::from_str(&info).unwrap();
        assert_eq!(info["rowCount"], 20);
        assert_eq!(info["minDate"], "2026-01-01");
        assert_eq!(info["maxDate"], "2026-01-20");
        assert_eq!(info["availableDates"].as_array().unwrap().len(), 20);

        let payload = chart_payload_inner("2026-01-01", "2026-01-02").unwrap();
        let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(payload["range"]["start"], "2026-01-01");
        assert_eq!(payload["range"]["end"], "2026-01-02");
        assert_eq!(
            payload["charts"]["calendar"]["cells"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            payload["dataset"]["sourceUrl"],
            metro_dash::data::SOURCE_URL
        );
    }

    #[test]
    fn empty_range_selects_default_window() {
        load_dataset_inner(FIXTURE, "2026-01-01T00:00:00Z").unwrap();
        let payload: serde_json::Value =
            serde_json::from_str(&chart_payload_inner("", "").unwrap()).unwrap();
        // Default first-visit range is a 90-day window clamped to the data.
        assert_eq!(payload["range"]["start"], "2026-01-01");
        assert_eq!(payload["range"]["end"], "2026-01-20");
    }

    #[test]
    fn payload_before_load_is_an_error() {
        // Each #[test] runs on its own thread, so the thread_local dataset
        // starts empty here.
        assert!(chart_payload_inner("", "").is_err());
        assert!(dataset_info_inner().is_err());
    }
}
