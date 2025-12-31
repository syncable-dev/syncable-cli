//! Device Authorization Grant flow (RFC 8628) for CLI authentication
//!
//! Implements the OAuth 2.0 device flow to authenticate CLI users via the Syncable web interface.

use super::credentials;
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use std::time::{Duration, Instant};

/// Production API URL (encore is reached via syncable.dev/api/*)
const SYNCABLE_API_URL_PROD: &str = "https://syncable.dev";
/// Development API URL
const SYNCABLE_API_URL_DEV: &str = "http://localhost:4000";
/// CLI client ID registered with the backend
const CLI_CLIENT_ID: &str = "syncable-cli";

/// Response from device code request
#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: u64,
}

/// Token response (success or error)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TokenResponse {
    Success {
        access_token: String,
        #[allow(dead_code)]
        token_type: String,
        expires_in: Option<u64>,
        refresh_token: Option<String>,
    },
    Error {
        error: String,
        #[allow(dead_code)]
        error_description: Option<String>,
    },
}

/// Get the API URL based on environment
fn get_api_url() -> &'static str {
    // Check for development environment
    if std::env::var("SYNCABLE_ENV").as_deref() == Ok("development") {
        SYNCABLE_API_URL_DEV
    } else {
        SYNCABLE_API_URL_PROD
    }
}

/// Perform the device authorization login flow
pub async fn login(no_browser: bool) -> Result<()> {
    println!("🔐 Authenticating with Syncable...\n");

    let client = Client::new();
    let api_url = get_api_url();

    // Step 1: Request device code
    let response = client
        .post(format!("{}/api/auth/device/code", api_url))
        .json(&serde_json::json!({
            "client_id": CLI_CLIENT_ID,
            "scope": "openid profile email"
        }))
        .send()
        .await
        .map_err(|e| anyhow!("Failed to connect to Syncable API: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Failed to request device authorization: {} - {}",
            status,
            body
        ));
    }

    let device_code: DeviceCodeResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Invalid response from server: {}", e))?;

    // Step 2: Display user code and instructions
    println!("📱 Device Authorization");
    println!("   ─────────────────────────────────────");
    println!("   Visit:  {}", device_code.verification_uri);
    println!("   Code:   \x1b[1;36m{}\x1b[0m", device_code.user_code);
    println!("   ─────────────────────────────────────\n");

    // Step 3: Open browser (unless --no-browser flag)
    if !no_browser {
        let url = device_code
            .verification_uri_complete
            .as_ref()
            .unwrap_or(&device_code.verification_uri);

        if let Err(e) = open::that(url) {
            println!("⚠️  Could not open browser automatically: {}", e);
            println!("   Please open the URL above manually.");
        } else {
            println!("🌐 Browser opened. Waiting for authorization...");
        }
    } else {
        println!("   Please open the URL above and enter the code.");
    }

    println!();

    // Step 4: Poll for token
    poll_for_token(&client, api_url, &device_code).await
}

/// Poll the token endpoint until authorization is complete
async fn poll_for_token(
    client: &Client,
    api_url: &str,
    device_code: &DeviceCodeResponse,
) -> Result<()> {
    let mut interval = device_code.interval;
    let deadline = Instant::now() + Duration::from_secs(device_code.expires_in);

    loop {
        // Check if code has expired
        if Instant::now() > deadline {
            return Err(anyhow!(
                "Device code expired. Please run 'sync-ctl auth login' again."
            ));
        }

        // Wait for polling interval
        tokio::time::sleep(Duration::from_secs(interval)).await;

        // Poll for token
        let response = client
            .post(format!("{}/api/auth/device/token", api_url))
            .json(&serde_json::json!({
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
                "device_code": device_code.device_code,
                "client_id": CLI_CLIENT_ID,
            }))
            .send()
            .await;

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                println!("⚠️  Network error, retrying: {}", e);
                continue;
            }
        };

        let body = response.text().await.unwrap_or_default();
        let token_response: TokenResponse = match serde_json::from_str(&body) {
            Ok(r) => r,
            Err(_) => {
                // Unexpected response, continue polling
                continue;
            }
        };

        match token_response {
            TokenResponse::Success {
                access_token,
                expires_in,
                refresh_token,
                ..
            } => {
                // Success! Save credentials
                credentials::save_credentials(
                    &access_token,
                    refresh_token.as_deref(),
                    None, // TODO: Fetch user email from session endpoint
                    expires_in,
                )?;

                println!("\n\x1b[1;32m✅ Authentication successful!\x1b[0m");
                println!("   Credentials saved to ~/.syncable.toml");
                return Ok(());
            }
            TokenResponse::Error { error, .. } => {
                match error.as_str() {
                    "authorization_pending" => {
                        // User hasn't completed authorization yet, keep polling
                        continue;
                    }
                    "slow_down" => {
                        // Server asked us to slow down
                        interval += 5;
                        continue;
                    }
                    "access_denied" => {
                        return Err(anyhow!("Authorization was denied by the user."));
                    }
                    "expired_token" => {
                        return Err(anyhow!(
                            "Device code expired. Please run 'sync-ctl auth login' again."
                        ));
                    }
                    _ => {
                        return Err(anyhow!("Authorization failed: {}", error));
                    }
                }
            }
        }
    }
}
