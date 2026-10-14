#[allow(clippy::module_inception)]
pub mod client;
pub mod discovery;

pub use client::EngineClient;
pub use discovery::discover_and_connect;
