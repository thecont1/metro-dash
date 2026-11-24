use std::sync::Arc;

use metro_dash::server::{AppState, app_router, load_initial_dataset, spawn_refresh_loop};
use reqwest::Client;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("NammaMetro-in-Charts/0.1")
        .build()
        .expect("HTTP client should build");
    let dataset = load_initial_dataset(&client).await;
    let state = AppState {
        dataset: Arc::new(RwLock::new(dataset)),
        client: client.clone(),
    };
    spawn_refresh_loop(state.dataset.clone(), client);
    let router = app_router(state);
    topcoat::start(router).await.unwrap();
}
