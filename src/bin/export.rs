use std::{path::PathBuf, sync::Arc, time::Duration};

use metro_dash::{
    data::{Dataset, fetch_dataset, parse_dataset},
    server::{AppState, app_router},
};
use reqwest::Client;
use tokio::sync::RwLock;
use topcoat::router::{Body, Request, to_bytes};

const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

fn usage() -> ! {
    eprintln!("usage: export [--out DIR] [--csv PATH]");
    eprintln!("  --out DIR   output directory for index.html (default: dist)");
    eprintln!("  --csv PATH  parse this CSV instead of fetching the live source");
    std::process::exit(2);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out_dir = PathBuf::from("dist");
    let mut csv_path: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => match args.next() {
                Some(dir) => out_dir = PathBuf::from(dir),
                None => usage(),
            },
            "--csv" => match args.next() {
                Some(path) => csv_path = Some(PathBuf::from(path)),
                None => usage(),
            },
            "-h" | "--help" => usage(),
            other => {
                eprintln!("unknown argument: {other}");
                usage();
            }
        }
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("NammaMetro-in-Charts-export/0.1")
        .build()?;

    let mut dataset: Dataset = match &csv_path {
        Some(path) => {
            let text = std::fs::read_to_string(path)?;
            let refreshed_at = chrono::Utc::now().to_rfc3339();
            parse_dataset(&text, refreshed_at)?
        }
        None => fetch_dataset(&client)
            .await
            .map_err(|error| format!("failed to fetch ridership dataset: {error}"))?,
    };

    // A --csv export bundles the selected file next to index.html and points
    // the client's live fetch at that same-origin copy, so interactive range
    // changes recompute from the same data the snapshot was built from.
    let bundled_csv = match &csv_path {
        Some(path) => {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "ridership.csv".to_string());
            dataset.source_url = name.clone();
            Some((path.clone(), name))
        }
        None => None,
    };
    println!(
        "Loaded {} ridership records (latest {})",
        dataset.records.len(),
        dataset.max_date
    );

    let state = AppState {
        dataset: Arc::new(RwLock::new(Some(dataset))),
        client,
    };
    let router = app_router(state);

    let request = Request::builder()
        .uri("/")
        .body(Body::empty())
        .expect("request should build");
    let response = router.handle(request).await;
    let status = response.status();
    let body = to_bytes(response.into_body(), MAX_BODY_BYTES)
        .await
        .map_err(|error| format!("failed to read response body: {error}"))?;
    if !status.is_success() {
        return Err(format!("render failed with status {status}").into());
    }

    std::fs::create_dir_all(&out_dir)?;
    if let Some((source, name)) = &bundled_csv {
        let dest = out_dir.join(name);
        // Skip the copy when source and destination are the same file —
        // fs::copy onto itself would truncate the CSV to zero bytes.
        if std::fs::canonicalize(source).ok() != std::fs::canonicalize(&dest).ok() {
            std::fs::copy(source, &dest)?;
            println!("Bundled {} as {}", source.display(), dest.display());
        }
    }
    let index_path = out_dir.join("index.html");
    std::fs::write(&index_path, &body)?;
    println!("Wrote {} ({} bytes)", index_path.display(), body.len());
    Ok(())
}
