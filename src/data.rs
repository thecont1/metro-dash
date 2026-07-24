use std::{collections::HashMap, fs, path::PathBuf};

use chrono::{Datelike, NaiveDate, Weekday};
use csv::StringRecord;
use reqwest::Client;

pub const SOURCE_URL: &str = "https://raw.githubusercontent.com/thecont1/namma-metro-ridership-tracker/main/NammaMetro_Ridership_Dataset.csv";
pub const SOURCE_PAGE_URL: &str = "https://github.com/thecont1/namma-metro-ridership-tracker";
pub const DEFAULT_START: &str = "2026-01-01";
pub const DEFAULT_END: &str = "2026-06-30";
pub const DEFAULT_CACHE_PATH: &str = ".cache/namma-metro-ridership.csv";
pub const FIRST_VISIT_WINDOW_DAYS: i64 = 90;

#[derive(Debug, Clone, PartialEq)]
pub struct RidershipRecord {
    pub date: NaiveDate,
    pub raw_fields: HashMap<String, String>,
    pub total_ridership: Option<f64>,
    pub smart_card: Option<f64>,
    pub ncmc: Option<f64>,
    pub token: Option<f64>,
    pub qr: Option<f64>,
    pub group_ticket: Option<f64>,
    pub commuter_ridership: Option<f64>,
    pub casual_ridership: Option<f64>,
    pub stored_value: Option<f64>,
    pub one_day_pass: Option<f64>,
    pub three_day_pass: Option<f64>,
    pub five_day_pass: Option<f64>,
    pub qr_namma: Option<f64>,
    pub qr_whatsapp: Option<f64>,
    pub qr_paytm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dataset {
    pub records: Vec<RidershipRecord>,
    pub min_date: NaiveDate,
    pub max_date: NaiveDate,
    pub invalid_date_count: usize,
    pub duplicate_count: usize,
    pub field_mapping: FieldMapping,
    pub refreshed_at: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldMapping {
    pub date: String,
    pub total: Option<String>,
    pub smart_card: Option<String>,
    pub ncmc: Option<String>,
    pub token: Option<String>,
    pub qr: Option<String>,
    pub group_ticket: Option<String>,
    pub stored_value: Option<String>,
    pub one_day_pass: Option<String>,
    pub three_day_pass: Option<String>,
    pub five_day_pass: Option<String>,
    pub qr_namma: Option<String>,
    pub qr_whatsapp: Option<String>,
    pub qr_paytm: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeSummary {
    pub calendar_days: usize,
    pub observation_days: usize,
    pub missing_days: usize,
    pub percentiles: Vec<(u8, f64)>,
    pub valid_total_count: usize,
    pub total_min: Option<f64>,
    pub total_max: Option<f64>,
    pub commute: Vec<WeekdayMetric>,
    pub casual: Vec<WeekdayMetric>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeekdayMetric {
    pub weekday: Weekday,
    pub n: usize,
    pub mean: Option<f64>,
    pub standard_deviation: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

pub fn choose_range(dataset: &Dataset, start: Option<&str>, end: Option<&str>) -> DateRange {
    let requested_start = start.and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
    let requested_end = end.and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
    let default_pair = default_first_visit_range(dataset);
    let mut start = requested_start.unwrap_or(default_pair.start);
    let mut end = requested_end.unwrap_or(default_pair.end);
    start = nearest_available(dataset, start);
    end = nearest_available(dataset, end);
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    DateRange { start, end }
}

pub fn default_range(dataset: &Dataset) -> DateRange {
    choose_range(dataset, Some(DEFAULT_START), Some(DEFAULT_END))
}

pub fn default_first_visit_range(dataset: &Dataset) -> DateRange {
    let end_target = dataset.max_date;
    let start_target = end_target - chrono::Duration::days(FIRST_VISIT_WINDOW_DAYS);
    let start = nearest_available(dataset, start_target.max(dataset.min_date));
    let end = nearest_available(dataset, end_target);
    DateRange { start, end }
}

pub fn nearest_available(dataset: &Dataset, target: NaiveDate) -> NaiveDate {
    dataset
        .records
        .iter()
        .min_by_key(|record| (record.date - target).num_days().unsigned_abs())
        .map(|record| record.date)
        .unwrap_or(dataset.min_date)
}

pub fn record_index(dataset: &Dataset, date: NaiveDate) -> usize {
    dataset
        .records
        .iter()
        .position(|record| record.date == date)
        .unwrap_or(0)
}

pub async fn fetch_dataset(client: &Client) -> std::result::Result<Dataset, String> {
    let response = client
        .get(SOURCE_URL)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("source returned HTTP {}", response.status()));
    }
    let body = response.text().await.map_err(|error| error.to_string())?;
    if body.trim().is_empty() {
        return Err("source returned an empty response".to_string());
    }
    let refreshed_at = chrono::Utc::now().to_rfc3339();
    let dataset = parse_dataset(&body, refreshed_at.clone())?;
    if let Err(error) = write_dataset_cache(&body, &refreshed_at) {
        eprintln!("Could not update ridership cache: {error}");
    }
    Ok(dataset)
}

fn cache_path() -> PathBuf {
    std::env::var("METRO_CACHE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_CACHE_PATH))
}

fn write_dataset_cache(csv_text: &str, refreshed_at: &str) -> std::result::Result<(), String> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, csv_text).map_err(|error| error.to_string())?;
    fs::write(path.with_extension("refreshed-at"), refreshed_at).map_err(|error| error.to_string())
}

pub fn load_cached_dataset() -> std::result::Result<Dataset, String> {
    let path = cache_path();
    let csv_text = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let refreshed_at = fs::read_to_string(path.with_extension("refreshed-at"))
        .map_err(|error| error.to_string())?;
    parse_dataset(&csv_text, refreshed_at.trim().to_string())
}

pub fn parse_dataset(csv_text: &str, refreshed_at: String) -> std::result::Result<Dataset, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(csv_text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|error| format!("CSV header error: {error}"))?
        .clone();
    let mapping = FieldMapping::from_headers(&headers)?;
    let mut by_date: HashMap<NaiveDate, RidershipRecord> = HashMap::new();
    let mut invalid_date_count = 0;
    let mut duplicate_count = 0;
    for row in reader.records() {
        let row = row.map_err(|error| format!("CSV row error: {error}"))?;
        let raw_date = field(&headers, &row, &mapping.date).unwrap_or_default();
        let Some(date) = parse_date(raw_date) else {
            invalid_date_count += 1;
            continue;
        };
        let record = normalise_record(&headers, &row, &mapping, date);
        if by_date.insert(date, record).is_some() {
            duplicate_count += 1;
            eprintln!("duplicate date {date}: keeping the latest valid row");
        }
    }
    let mut records: Vec<_> = by_date.into_values().collect();
    records.sort_by_key(|record| record.date);
    let Some(first) = records.first() else {
        return Err("CSV contained no usable dated records".to_string());
    };
    let min_date = first.date;
    let max_date = records.last().expect("first exists").date;
    Ok(Dataset {
        records,
        min_date,
        max_date,
        invalid_date_count,
        duplicate_count,
        field_mapping: mapping,
        refreshed_at,
    })
}

impl FieldMapping {
    fn from_headers(headers: &StringRecord) -> std::result::Result<Self, String> {
        let names: Vec<(String, String)> = headers
            .iter()
            .map(|header| (normalise_header(header), header.to_string()))
            .collect();
        let find = |aliases: &[&str]| {
            names
                .iter()
                .find(|(normalised, _)| aliases.iter().any(|alias| normalised == *alias))
                .map(|(_, original)| original.clone())
        };
        let date = find(&["recorddate", "date", "journeydate"])
            .ok_or_else(|| "CSV has no recognisable date column".to_string())?;
        Ok(Self {
            date,
            total: find(&[
                "totalridership",
                "totaldailyridership",
                "totaldaily ridership",
                "totaljourneys",
                "total",
            ]),
            smart_card: find(&["totalsmartcards", "smartcard", "smartcards"]),
            ncmc: find(&["totalncmc", "ncmc"]),
            token: find(&["totaltokens", "token", "tokens"]),
            qr: find(&["totalqr", "qr", "qrtickets"]),
            group_ticket: find(&["groupticket", "grouptickets"]),
            stored_value: find(&["storedvaluecard", "storedvalue", "storedvalue card"]),
            one_day_pass: find(&["onedaypass", "oneday pass"]),
            three_day_pass: find(&["threedaypass", "threeday pass"]),
            five_day_pass: find(&["fivedaypass", "fiveday pass"]),
            qr_namma: find(&["qrnammametro", "qrnamma"]),
            qr_whatsapp: find(&["qrwhatsapp"]),
            qr_paytm: find(&["qrpaytm"]),
        })
    }
}

fn normalise_header(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

fn field<'a>(headers: &'a StringRecord, row: &'a StringRecord, header: &str) -> Option<&'a str> {
    headers
        .iter()
        .position(|candidate| candidate == header)
        .and_then(|index| row.get(index))
        .map(str::trim)
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    ["%d-%m-%Y", "%Y-%m-%d", "%d/%m/%Y", "%Y/%m/%d"]
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(value.trim(), format).ok())
}

pub fn clean_number(value: Option<&str>) -> Option<f64> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    value
        .replace(',', "")
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

fn normalise_record(
    headers: &StringRecord,
    row: &StringRecord,
    mapping: &FieldMapping,
    date: NaiveDate,
) -> RidershipRecord {
    let smart_card = clean_number(
        mapping
            .smart_card
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let ncmc = clean_number(
        mapping
            .ncmc
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let token = clean_number(
        mapping
            .token
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let qr = clean_number(
        mapping
            .qr
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let group_ticket = clean_number(
        mapping
            .group_ticket
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let total_ridership = clean_number(
        mapping
            .total
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    )
    .or_else(|| sum_complete([smart_card, ncmc, token, qr, group_ticket]));
    let commuter_ridership = sum_complete([smart_card, ncmc]);
    let casual_ridership = sum_complete([token, qr, group_ticket]);
    let stored_value = clean_number(
        mapping
            .stored_value
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let one_day_pass = clean_number(
        mapping
            .one_day_pass
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let three_day_pass = clean_number(
        mapping
            .three_day_pass
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let five_day_pass = clean_number(
        mapping
            .five_day_pass
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let qr_namma = clean_number(
        mapping
            .qr_namma
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let qr_whatsapp = clean_number(
        mapping
            .qr_whatsapp
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let qr_paytm = clean_number(
        mapping
            .qr_paytm
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let raw_fields = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            (
                header.to_string(),
                row.get(index).unwrap_or_default().trim().to_string(),
            )
        })
        .collect();
    RidershipRecord {
        date,
        raw_fields,
        total_ridership,
        smart_card,
        ncmc,
        token,
        qr,
        group_ticket,
        commuter_ridership,
        casual_ridership,
        stored_value,
        one_day_pass,
        three_day_pass,
        five_day_pass,
        qr_namma,
        qr_whatsapp,
        qr_paytm,
    }
}

fn sum_complete<const N: usize>(values: [Option<f64>; N]) -> Option<f64> {
    values
        .iter()
        .copied()
        .collect::<Option<Vec<_>>>()
        .map(|values| values.iter().sum())
}

pub fn summarise(dataset: &Dataset, range: DateRange) -> RangeSummary {
    let selected: Vec<_> = dataset
        .records
        .iter()
        .filter(|record| record.date >= range.start && record.date <= range.end)
        .collect();
    let calendar_days = (range.end - range.start).num_days() as usize + 1;
    let totals: Vec<f64> = selected
        .iter()
        .filter_map(|record| record.total_ridership)
        .collect();
    let mut percentiles = Vec::new();
    for percentile in [2, 5, 10, 25, 50, 75, 90, 95, 98] {
        if let Some(value) = quantile(&totals, percentile as f64 / 100.0) {
            percentiles.push((percentile, value));
        }
    }
    RangeSummary {
        calendar_days,
        observation_days: selected.len(),
        missing_days: calendar_days.saturating_sub(selected.len()),
        percentiles,
        valid_total_count: totals.len(),
        total_min: totals.iter().copied().reduce(f64::min),
        total_max: totals.iter().copied().reduce(f64::max),
        commute: weekday_metrics(&selected, |record| record.commuter_ridership),
        casual: weekday_metrics(&selected, |record| record.casual_ridership),
    }
}

fn weekday_metrics(
    records: &[&RidershipRecord],
    value: impl Fn(&RidershipRecord) -> Option<f64>,
) -> Vec<WeekdayMetric> {
    [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ]
    .iter()
    .map(|&weekday| {
        let values: Vec<f64> = records
            .iter()
            .filter(|record| record.date.weekday() == weekday)
            .filter_map(|record| value(record))
            .collect();
        let n = values.len();
        let mean = (!values.is_empty()).then(|| values.iter().sum::<f64>() / n as f64);
        let standard_deviation = (n > 1).then(|| {
            let average = mean.expect("n > 1 has a mean");
            (values
                .iter()
                .map(|value| (value - average).powi(2))
                .sum::<f64>()
                / (n - 1) as f64)
                .sqrt()
        });
        WeekdayMetric {
            weekday,
            n,
            mean,
            standard_deviation,
            min: values.iter().copied().reduce(f64::min),
            max: values.iter().copied().reduce(f64::max),
        }
    })
    .collect()
}

pub fn quantile(values: &[f64], probability: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    Some(sorted[lower] + (sorted[upper] - sorted[lower]) * (position - lower as f64))
}
