pub mod charts;
pub mod data;
pub mod payload;

// Everything below is only needed to serve the app over HTTP (the topcoat
// handlers, the embedded page shell, and the client/style assets they
// inline). The wasm compute crate builds this library with
// `default-features = false`, so none of it is compiled for wasm32.
#[cfg(feature = "server")]
pub mod client;
#[cfg(feature = "server")]
pub mod render;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "server")]
pub mod style;

#[cfg(all(test, feature = "server"))]
mod tests;
