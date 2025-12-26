mod client;
mod config;
mod user;

#[cfg(test)]
mod test;

pub use client::TelemetryClient;
pub use config::TelemetryConfig;
pub use user::UserId;

use crate::config::types::Config;
use std::sync::OnceLock;

static TELEMETRY_CLIENT: OnceLock<TelemetryClient> = OnceLock::new();

pub async fn init_telemetry(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let telemetry_enabled = config.telemetry.enabled
        && std::env::var("SYNCABLE_CLI_TELEMETRY").unwrap_or_default() != "false";

    if telemetry_enabled {
        let client = TelemetryClient::new(config).await?;
        TELEMETRY_CLIENT
            .set(client)
            .map_err(|_| "Failed to set telemetry client")?;
    }

    Ok(())
}

pub fn get_telemetry_client() -> Option<&'static TelemetryClient> {
    TELEMETRY_CLIENT.get()
}
