use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("telemetry collection error: {0}")]
    Collection(String),
}
