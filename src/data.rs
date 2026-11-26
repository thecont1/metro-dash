use std::collections::HashMap;
#[cfg(feature = "server")]
use std::{fs, path::PathBuf};

use chrono::{Datelike, Duration, Months, NaiveDate, Weekday};
use csv::StringRecord;
#[cfg(feature = "server")]
use reqwest::Client;

pub const SOURCE_URL: &str = "https://raw.githubusercontent.com/thecont1/namma-metro-ridership-tracker/main/NammaMetro_Ridership_Dataset.csv";
pub const SOURCE_PAGE_URL: &str = "https://github.com/thecont1/namma-metro-ridership-tracker";
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
    pub qr_ondc: Option<f64>,
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
    /// Fingerprint of the source CSV text. Unlike `max_date`/`row_count` it
    /// also changes when upstream corrects existing rows in place.
    pub revision: u64,
    /// URL the client fetches for live recomputation. Normally the canonical
    /// GitHub CSV; the static exporter overrides it when `--csv` bundles a
    /// local copy next to `index.html`.
    pub source_url: String,
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
    pub qr_ondc: Option<String>,
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

pub const BASELINE_WINDOW: usize = 12;
pub const BASELINE_MIN_OBSERVATIONS: usize = 4;

pub const WATERFALL_CATEGORIES: [(&str, &str); 5] = [
    ("smart-card", "Smart Cards"),
    ("ncmc", "NCMC"),
    ("token", "Tokens"),
    ("qr", "QR"),
    ("group-ticket", "Group Ticket"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentDelta {
    pub key: &'static str,
    pub label: &'static str,
    pub reference: f64,
    pub actual: f64,
    pub delta: f64,
    pub cumulative_start: f64,
    pub cumulative_end: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentWaterfall {
    pub date: NaiveDate,
    pub weekday: Weekday,
    pub baseline_n: usize,
    pub reference_total: Option<f64>,
    pub actual_total: f64,
    pub delta_total: Option<f64>,
    pub components: Vec<ComponentDelta>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FareMixMonth {
    pub month: NaiveDate,
    pub published_days: usize,
    pub complete_days: usize,
    pub total: Option<f64>,
    pub components: [f64; 5],
}

#[derive(Debug, Clone, PartialEq)]
pub struct NcmcShareWeek {
    pub week: NaiveDate,
    pub published_days: usize,
    pub complete_days: usize,
    pub ncmc: f64,
    pub commuter_total: f64,
}

impl NcmcShareWeek {
    pub fn share(&self) -> Option<f64> {
        (self.complete_days > 0 && self.commuter_total > 0.0)
            .then(|| self.ncmc / self.commuter_total)
    }
}

pub const QR_PROVIDER_CATEGORIES: [(&str, &str); 4] = [
    ("qr-namma", "Namma Metro"),
    ("qr-whatsapp", "WhatsApp"),
    ("qr-paytm", "Paytm"),
    ("qr-ondc", "ONDC"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct QrMixMonth {
    pub month: NaiveDate,
    pub published_days: usize,
    pub complete_days: usize,
    pub providers: [f64; 4],
    pub provider_total: f64,
    pub parent_qr_total: f64,
}

impl QrMixMonth {
    pub fn has_mix(&self) -> bool {
        self.complete_days > 0 && self.provider_total > 0.0
    }
}

pub const SMART_SUBTYPE_CATEGORIES: [(&str, &str); 4] = [
    ("stored-value", "Stored Value"),
    ("one-day-pass", "One Day Pass"),
    ("three-day-pass", "Three Day Pass"),
    ("five-day-pass", "Five Day Pass"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct SmartMixMonth {
    pub month: NaiveDate,
    pub published_days: usize,
    pub complete_days: usize,
    pub subtypes: [f64; 4],
    pub subtype_total: f64,
    pub parent_smart_total: f64,
}

impl SmartMixMonth {
    pub fn has_mix(&self) -> bool {
        self.complete_days > 0 && self.subtype_total > 0.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeekendSignature {
    pub key: &'static str,
    pub label: &'static str,
    pub published_days: usize,
    pub complete_days: usize,
    pub minimum: Option<f64>,
    pub lower_quartile: Option<f64>,
    pub median: Option<f64>,
    pub upper_quartile: Option<f64>,
    pub maximum: Option<f64>,
    pub components: [f64; 5],
    pub component_total: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeekdayDistribution {
    pub weekday: Weekday,
    pub n: usize,
    pub minimum: Option<f64>,
    pub lower_quartile: Option<f64>,
    pub median: Option<f64>,
    pub upper_quartile: Option<f64>,
    pub maximum: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineStatus {
    High,
    Low,
    Within,
    Insufficient,
}

impl BaselineStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Low => "low",
            Self::Within => "within",
            Self::Insufficient => "insufficient",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::High => "Above band",
            Self::Low => "Below band",
            Self::Within => "Within band",
            Self::Insufficient => "Baseline unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeekdayBaselinePoint {
    pub date: NaiveDate,
    pub weekday: Weekday,
    pub total: f64,
    pub lower_quartile: Option<f64>,
    pub median: Option<f64>,
    pub upper_quartile: Option<f64>,
    pub baseline_n: usize,
    pub status: BaselineStatus,
}

pub fn choose_range(dataset: &Dataset, start: Option<&str>, end: Option<&str>) -> DateRange {
    let requested_start = start.and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
    let requested_end = end.and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
    let default_pair = default_first_visit_range(dataset);
    let mut start = requested_start.unwrap_or(default_pair.start);
    let mut end = requested_end.unwrap_or(default_pair.end);
    if requested_start.is_some() {
        start = start.max(dataset.min_date).min(dataset.max_date);
    } else {
        start = nearest_available(dataset, start);
    }
    if requested_end.is_some() {
        end = end.max(dataset.min_date).min(dataset.max_date);
    } else {
        end = nearest_available(dataset, end);
    }
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    DateRange { start, end }
}

pub fn default_first_visit_range(dataset: &Dataset) -> DateRange {
    let end_target = dataset.max_date;
    let start_target = end_target - chrono::Duration::days(FIRST_VISIT_WINDOW_DAYS);
    let start = nearest_available(dataset, start_target.max(dataset.min_date));
    let end = nearest_available(dataset, end_target);
    DateRange { start, end }
}

pub fn last_n_months_range(dataset: &Dataset, months: u32) -> DateRange {
    let latest = dataset.max_date;
    let first_day_of_ongoing =
        NaiveDate::from_ymd_opt(latest.year(), latest.month(), 1).expect("valid first of month");
    let end = (first_day_of_ongoing - Duration::days(1)).max(dataset.min_date);
    let start = first_day_of_ongoing
        .checked_sub_months(Months::new(months))
        .map(|date| date.max(dataset.min_date))
        .unwrap_or(dataset.min_date)
        .min(end);
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
        .enumerate()
        .min_by_key(|(_, record)| (record.date - date).num_days().unsigned_abs())
        .map(|(index, _)| index)
        .unwrap_or(0)
}

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
fn cache_path() -> PathBuf {
    std::env::var("METRO_CACHE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_CACHE_PATH))
}

#[cfg(feature = "server")]
fn write_dataset_cache(csv_text: &str, refreshed_at: &str) -> std::result::Result<(), String> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, csv_text).map_err(|error| error.to_string())?;
    fs::write(path.with_extension("refreshed-at"), refreshed_at).map_err(|error| error.to_string())
}

#[cfg(feature = "server")]
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
        revision: content_revision(csv_text),
        source_url: SOURCE_URL.to_string(),
    })
}

/// FNV-1a 64-bit fingerprint of the raw CSV text. Hand-rolled (rather than
/// `DefaultHasher`) so host and wasm builds produce identical values.
fn content_revision(csv_text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in csv_text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
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
            qr_ondc: find(&["qrondc"]),
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
    let qr_ondc = clean_number(
        mapping
            .qr_ondc
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let smart_card = sum_complete([stored_value, one_day_pass, three_day_pass, five_day_pass])
        .or_else(|| {
            clean_number(
                mapping
                    .smart_card
                    .as_deref()
                    .and_then(|name| field(headers, row, name)),
            )
        });
    let qr = sum_complete([qr_namma, qr_whatsapp, qr_paytm, qr_ondc]).or_else(|| {
        clean_number(
            mapping
                .qr
                .as_deref()
                .and_then(|name| field(headers, row, name)),
        )
    });
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
    let group_ticket = clean_number(
        mapping
            .group_ticket
            .as_deref()
            .and_then(|name| field(headers, row, name)),
    );
    let total_ridership = sum_complete([
        stored_value,
        one_day_pass,
        three_day_pass,
        five_day_pass,
        token,
        ncmc,
        qr_namma,
        qr_whatsapp,
        qr_paytm,
        qr_ondc,
        group_ticket,
    ])
    .or_else(|| sum_complete([smart_card, ncmc, token, qr, group_ticket]));
    let commuter_ridership = sum_complete([smart_card, ncmc]);
    let casual_ridership = sum_complete([token, qr, group_ticket]);
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
        qr_ondc,
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

/// Compute a rolling, weekday-specific baseline for every reported day in
/// `range`. The baseline for a day uses up to six published reports before
/// and six after that day with the same weekday, so a Monday is compared with
/// nearby Mondays and the current value never contaminates its own reference band.
pub fn weekday_baseline_series(dataset: &Dataset, range: DateRange) -> Vec<WeekdayBaselinePoint> {
    dataset
        .records
        .iter()
        .filter(|record| record.date >= range.start && record.date <= range.end)
        .filter_map(|record| {
            let total = record.total_ridership?;
            let half_window = BASELINE_WINDOW / 2;
            let prior_values = dataset
                .records
                .iter()
                .rev()
                .filter(|candidate| {
                    candidate.date < record.date
                        && candidate.date.weekday() == record.date.weekday()
                })
                .filter_map(|candidate| candidate.total_ridership)
                .take(half_window);
            let following_values = dataset
                .records
                .iter()
                .filter(|candidate| {
                    candidate.date > record.date
                        && candidate.date.weekday() == record.date.weekday()
                })
                .filter_map(|candidate| candidate.total_ridership)
                .take(half_window);
            let baseline_values: Vec<f64> = prior_values.chain(following_values).collect();
            let baseline_n = baseline_values.len();
            let quartiles = (baseline_n >= BASELINE_MIN_OBSERVATIONS).then(|| {
                (
                    quantile(&baseline_values, 0.25),
                    quantile(&baseline_values, 0.5),
                    quantile(&baseline_values, 0.75),
                )
            });
            let (lower_quartile, median, upper_quartile) = match quartiles {
                Some((Some(lower), Some(median), Some(upper))) => {
                    (Some(lower), Some(median), Some(upper))
                }
                _ => (None, None, None),
            };
            let status = match (lower_quartile, upper_quartile) {
                (Some(_), Some(upper)) if total > upper => BaselineStatus::High,
                (Some(lower), Some(_)) if total < lower => BaselineStatus::Low,
                (Some(_), Some(_)) => BaselineStatus::Within,
                _ => BaselineStatus::Insufficient,
            };
            Some(WeekdayBaselinePoint {
                date: record.date,
                weekday: record.date.weekday(),
                total,
                lower_quartile,
                median,
                upper_quartile,
                baseline_n,
                status,
            })
        })
        .collect()
}

fn raw_parent_value(record: &RidershipRecord, header: Option<&str>) -> Option<f64> {
    clean_number(
        record
            .raw_fields
            .get(header?)
            .map(std::string::String::as_str),
    )
}

fn parent_components(dataset: &Dataset, record: &RidershipRecord) -> Option<[f64; 5]> {
    let mapping = &dataset.field_mapping;
    Some([
        raw_parent_value(record, mapping.smart_card.as_deref())?,
        raw_parent_value(record, mapping.ncmc.as_deref())?,
        raw_parent_value(record, mapping.token.as_deref())?,
        raw_parent_value(record, mapping.qr.as_deref())?,
        raw_parent_value(record, mapping.group_ticket.as_deref())?,
    ])
}

/// Build a component-level waterfall for `date` against nearby reports from
/// the same weekday. The reference value for every category is the median of
/// the same candidate reports, so the displayed arithmetic reconciles exactly:
/// reference total + category deltas = selected-day total. Unlike the derived
/// totals used elsewhere, these are the published parent-category totals; QR
/// provider splits do not always reconcile with their parent in the source.
pub fn component_waterfall(dataset: &Dataset, date: NaiveDate) -> Option<ComponentWaterfall> {
    let record = dataset.records.iter().find(|record| record.date == date)?;
    let actual = parent_components(dataset, record)?;
    let half_window = BASELINE_WINDOW / 2;
    let complete_nearby = |candidate: &&RidershipRecord| {
        candidate.date != record.date
            && candidate.date.weekday() == record.date.weekday()
            && parent_components(dataset, candidate).is_some()
    };
    let prior: Vec<&RidershipRecord> = dataset
        .records
        .iter()
        .rev()
        .filter(|candidate| candidate.date < record.date)
        .filter(complete_nearby)
        .take(half_window)
        .collect();
    let following: Vec<&RidershipRecord> = dataset
        .records
        .iter()
        .filter(|candidate| candidate.date > record.date)
        .filter(complete_nearby)
        .take(half_window)
        .collect();
    let nearby: Vec<&RidershipRecord> = prior.into_iter().chain(following).collect();
    let baseline_n = nearby.len();
    let actual_total: f64 = actual.iter().sum();
    if baseline_n < BASELINE_MIN_OBSERVATIONS {
        return Some(ComponentWaterfall {
            date,
            weekday: date.weekday(),
            baseline_n,
            reference_total: None,
            actual_total,
            delta_total: None,
            components: Vec::new(),
        });
    }

    let mut references = [0.0; 5];
    for (index, reference) in references.iter_mut().enumerate() {
        let values: Vec<f64> = nearby
            .iter()
            .filter_map(|candidate| {
                parent_components(dataset, candidate).map(|components| components[index])
            })
            .collect();
        *reference = quantile(&values, 0.5)?;
    }
    let reference_total: f64 = references.iter().sum();
    let mut cumulative = reference_total;
    let components = WATERFALL_CATEGORIES
        .iter()
        .enumerate()
        .map(|(index, (key, label))| {
            let reference = references[index];
            let actual_value = actual[index];
            let delta = actual_value - reference;
            let cumulative_start = cumulative;
            cumulative += delta;
            ComponentDelta {
                key,
                label,
                reference,
                actual: actual_value,
                delta,
                cumulative_start,
                cumulative_end: cumulative,
            }
        })
        .collect::<Vec<_>>();
    Some(ComponentWaterfall {
        date,
        weekday: date.weekday(),
        baseline_n,
        reference_total: Some(reference_total),
        actual_total,
        delta_total: Some(actual_total - reference_total),
        components,
    })
}

/// Aggregate published parent-category totals by calendar month. Shares are
/// only meaningful for days with the full parent-component set, so incomplete
/// rows count as published but contribute neither to `complete_days` nor to
/// the monthly totals. Months with no usable rows remain present as gaps.
pub fn fare_mix_series(dataset: &Dataset, range: DateRange) -> Vec<FareMixMonth> {
    let mut cursor = NaiveDate::from_ymd_opt(range.start.year(), range.start.month(), 1)
        .expect("valid month start");
    let mut months = Vec::new();
    while cursor <= range.end {
        let next_month = cursor
            .checked_add_months(Months::new(1))
            .expect("valid next month");
        let month_end = (next_month - Duration::days(1)).min(range.end);
        let rows: Vec<&RidershipRecord> = dataset
            .records
            .iter()
            .filter(|record| record.date >= cursor.max(range.start) && record.date <= month_end)
            .collect();
        let mut components = [0.0; 5];
        let mut complete_days = 0;
        for record in &rows {
            if let Some(values) = parent_components(dataset, record) {
                complete_days += 1;
                for (index, value) in values.iter().enumerate() {
                    components[index] += value;
                }
            }
        }
        let total = (complete_days > 0).then(|| components.iter().sum::<f64>());
        months.push(FareMixMonth {
            month: cursor,
            published_days: rows.len(),
            complete_days,
            total,
            components,
        });
        cursor = next_month;
    }
    months
}

/// Aggregate Smart Card + NCMC into ISO weeks. Both components must be present
/// on a day for that day to contribute, so the count and share panels are
/// calculated from the same evidence base.
pub fn ncmc_share_series(dataset: &Dataset, range: DateRange) -> Vec<NcmcShareWeek> {
    let start_week =
        range.start - Duration::days(range.start.weekday().num_days_from_monday() as i64);
    let mut cursor = start_week;
    let mut weeks = Vec::new();
    while cursor <= range.end {
        let week_end = (cursor + Duration::days(6)).min(range.end);
        let rows: Vec<&RidershipRecord> = dataset
            .records
            .iter()
            .filter(|record| record.date >= cursor.max(range.start) && record.date <= week_end)
            .collect();
        let mut complete_days = 0;
        let mut ncmc = 0.0;
        let mut commuter_total = 0.0;
        for record in &rows {
            if let (Some(smart_card), Some(ncmc_value)) = (record.smart_card, record.ncmc) {
                complete_days += 1;
                ncmc += ncmc_value;
                commuter_total += smart_card + ncmc_value;
            }
        }
        weeks.push(NcmcShareWeek {
            week: cursor,
            published_days: rows.len(),
            complete_days,
            ncmc,
            commuter_total,
        });
        cursor += Duration::days(7);
    }
    weeks
}

fn qr_provider_components(record: &RidershipRecord) -> Option<[f64; 4]> {
    Some([
        record.qr_namma?,
        record.qr_whatsapp?,
        record.qr_paytm?,
        record.qr_ondc?,
    ])
}

/// Aggregate QR provider counts by month. Because provider splits do not
/// always reconcile with the parent QR total, shares use the provider sum
/// while the parent total is retained separately for the scale strip.
pub fn qr_mix_series(dataset: &Dataset, range: DateRange) -> Vec<QrMixMonth> {
    let mut cursor = NaiveDate::from_ymd_opt(range.start.year(), range.start.month(), 1)
        .expect("valid month start");
    let mut months = Vec::new();
    while cursor <= range.end {
        let next_month = cursor
            .checked_add_months(Months::new(1))
            .expect("valid next month");
        let month_end = (next_month - Duration::days(1)).min(range.end);
        let rows: Vec<&RidershipRecord> = dataset
            .records
            .iter()
            .filter(|record| record.date >= cursor.max(range.start) && record.date <= month_end)
            .collect();
        let mut providers = [0.0; 4];
        let mut complete_days = 0;
        let mut parent_qr_total = 0.0;
        for record in &rows {
            if let Some(values) = qr_provider_components(record) {
                complete_days += 1;
                for (index, value) in values.iter().enumerate() {
                    providers[index] += value;
                }
                if let Some(parent) = raw_parent_value(record, dataset.field_mapping.qr.as_deref())
                {
                    parent_qr_total += parent;
                }
            }
        }
        months.push(QrMixMonth {
            month: cursor,
            published_days: rows.len(),
            complete_days,
            providers,
            provider_total: providers.iter().sum(),
            parent_qr_total,
        });
        cursor = next_month;
    }
    months
}

fn smart_subtype_components(record: &RidershipRecord) -> Option<[f64; 4]> {
    Some([
        record.stored_value?,
        record.one_day_pass?,
        record.three_day_pass?,
        record.five_day_pass?,
    ])
}

/// Compare Monday–Friday, Saturday, and Sunday using only days with complete
/// published parent components, so the distribution and fare-media shares share
/// the same evidence base.
pub fn weekend_signature_series(dataset: &Dataset, range: DateRange) -> Vec<WeekendSignature> {
    [
        ("weekday", "Monday–Friday"),
        ("saturday", "Saturday"),
        ("sunday", "Sunday"),
    ]
    .iter()
    .map(|(key, label)| {
        let matches = |weekday: Weekday| match *key {
            "weekday" => weekday.num_days_from_monday() < 5,
            "saturday" => weekday == Weekday::Sat,
            _ => weekday == Weekday::Sun,
        };
        let rows: Vec<&RidershipRecord> = dataset
            .records
            .iter()
            .filter(|record| {
                record.date >= range.start
                    && record.date <= range.end
                    && matches(record.date.weekday())
            })
            .collect();
        let mut totals = Vec::new();
        let mut components = [0.0; 5];
        let mut complete_days = 0;
        for record in &rows {
            if let Some(values) = parent_components(dataset, record) {
                complete_days += 1;
                totals.push(values.iter().sum::<f64>());
                for (index, value) in values.iter().enumerate() {
                    components[index] += value;
                }
            }
        }
        WeekendSignature {
            key,
            label,
            published_days: rows.len(),
            complete_days,
            minimum: totals.iter().copied().reduce(f64::min),
            lower_quartile: quantile(&totals, 0.25),
            median: quantile(&totals, 0.5),
            upper_quartile: quantile(&totals, 0.75),
            maximum: totals.iter().copied().reduce(f64::max),
            components,
            component_total: components.iter().sum(),
        }
    })
    .collect()
}

/// Distribution of observed total ridership for each weekday. Missing reports
/// are omitted rather than zero-filled; zero-valued reports remain observations.
pub fn weekday_distribution_series(
    dataset: &Dataset,
    range: DateRange,
) -> Vec<WeekdayDistribution> {
    let mut buckets: [Vec<f64>; 7] = Default::default();
    for record in &dataset.records {
        if record.date < range.start || record.date > range.end {
            continue;
        }
        if let Some(total) = record.total_ridership {
            buckets[record.date.weekday().num_days_from_monday() as usize].push(total);
        }
    }
    (0..7)
        .map(|index| {
            let values = &buckets[index];
            WeekdayDistribution {
                weekday: Weekday::try_from(index as u8).expect("valid weekday index"),
                n: values.len(),
                minimum: values.iter().copied().reduce(f64::min),
                lower_quartile: quantile(values, 0.25),
                median: quantile(values, 0.5),
                upper_quartile: quantile(values, 0.75),
                maximum: values.iter().copied().reduce(f64::max),
            }
        })
        .collect()
}

/// Aggregate Smart Card subtypes by month. Shares use the subtype sum; the
/// published parent Smart Card total is retained separately as scale context.
pub fn smart_mix_series(dataset: &Dataset, range: DateRange) -> Vec<SmartMixMonth> {
    let mut cursor = NaiveDate::from_ymd_opt(range.start.year(), range.start.month(), 1)
        .expect("valid month start");
    let mut months = Vec::new();
    while cursor <= range.end {
        let next_month = cursor
            .checked_add_months(Months::new(1))
            .expect("valid next month");
        let month_end = (next_month - Duration::days(1)).min(range.end);
        let rows: Vec<&RidershipRecord> = dataset
            .records
            .iter()
            .filter(|record| record.date >= cursor.max(range.start) && record.date <= month_end)
            .collect();
        let mut subtypes = [0.0; 4];
        let mut complete_days = 0;
        let mut parent_smart_total = 0.0;
        for record in &rows {
            if let Some(values) = smart_subtype_components(record) {
                complete_days += 1;
                for (index, value) in values.iter().enumerate() {
                    subtypes[index] += value;
                }
                if let Some(parent) =
                    raw_parent_value(record, dataset.field_mapping.smart_card.as_deref())
                {
                    parent_smart_total += parent;
                }
            }
        }
        months.push(SmartMixMonth {
            month: cursor,
            published_days: rows.len(),
            complete_days,
            subtypes,
            subtype_total: subtypes.iter().sum(),
            parent_smart_total,
        });
        cursor = next_month;
    }
    months
}
