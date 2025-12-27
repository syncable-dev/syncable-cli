use crate::config::types::Config;
use crate::telemetry::user::UserId;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

// PostHog API endpoint and key - Using EU endpoint to match your successful curl test
const POSTHOG_API_ENDPOINT: &str = "https://eu.i.posthog.com/capture/";
const POSTHOG_PROJECT_API_KEY: &str = "phc_t5zrCHU3yiU52lcUfOP3SiCSxdhJcmB2I3m06dGTk2D";

pub struct TelemetryClient {
    user_id: UserId,
    http_client: Client,
    pending_tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl TelemetryClient {
    pub async fn new(_config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let user_id = UserId::load_or_create()?;
        let http_client = Client::new();
        let pending_tasks = Arc::new(Mutex::new(Vec::new()));

        Ok(Self {
            user_id,
            http_client,
            pending_tasks,
        })
    }

    // Helper function to create common properties
    fn create_common_properties(&self) -> HashMap<String, serde_json::Value> {
        let mut properties = HashMap::new();
        properties.insert("version".to_string(), json!(env!("CARGO_PKG_VERSION")));
        properties.insert("os".to_string(), json!(std::env::consts::OS));
        properties.insert("personal_id".to_string(), json!(rand::random::<u32>()));
        properties.insert("distinct_id".to_string(), json!(self.user_id.id.clone()));
        properties
    }

    pub fn track_event(&self, name: &str, properties: HashMap<String, serde_json::Value>) {
        let mut event_properties = self.create_common_properties();

        // Merge provided properties
        for (key, value) in properties {
            event_properties.insert(key, value);
        }

        let event_name = name.to_string();
        let client = self.http_client.clone();
        let pending_tasks = self.pending_tasks.clone();

        log::debug!("Tracking event: {}", event_name);

        // Send the event asynchronously
        let handle = tokio::spawn(async move {
            // Create the event payload according to PostHog API
            let payload = json!({
                "api_key": POSTHOG_PROJECT_API_KEY,
                "event": event_name,
                "properties": event_properties,
                "timestamp": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            });

            log::debug!("Sending telemetry payload: {:?}", payload);

            match client
                .post(POSTHOG_API_ENDPOINT)
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        log::debug!("Successfully sent telemetry event: {}", event_name);
                    } else {
                        let status = response.status();
                        let body = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        log::warn!(
                            "Failed to send telemetry event '{}': HTTP {} - {}",
                            event_name,
                            status,
                            body
                        );
                    }
                }
                Err(e) => {
                    log::warn!("Failed to send telemetry event '{}': {}", event_name, e);
                }
            }
        });

        // Keep track of the task
        let pending_tasks_clone = pending_tasks.clone();
        tokio::spawn(async move {
            pending_tasks_clone.lock().await.push(handle);
        });
    }

    // Specific methods for the actual commands
    pub fn track_analyze(&self, properties: HashMap<String, serde_json::Value>) {
        self.track_event("analyze", properties);
    }

    pub fn track_generate(&self, properties: HashMap<String, serde_json::Value>) {
        self.track_event("generate", properties);
    }

    pub fn track_validate(&self, properties: HashMap<String, serde_json::Value>) {
        self.track_event("validate", properties);
    }

    pub fn track_support(&self, properties: HashMap<String, serde_json::Value>) {
        self.track_event("support", properties);
    }

    pub fn track_dependencies(&self, properties: HashMap<String, serde_json::Value>) {
        self.track_event("dependencies", properties);
    }

    // Updated to accept properties
    pub fn track_vulnerabilities(&self, properties: HashMap<String, serde_json::Value>) {
        self.track_event("Vulnerability Scan", properties);
    }

    // Updated to accept properties
    pub fn track_security(&self, properties: HashMap<String, serde_json::Value>) {
        self.track_event("Security Scan", properties);
    }

    pub fn track_tools(&self, properties: HashMap<String, serde_json::Value>) {
        self.track_event("tools", properties);
    }

    // Existing specific methods for events
    pub fn track_security_scan(&self) {
        // Deprecated: Use track_security with properties instead
    }

    // Updated to accept properties
    pub fn track_analyze_folder(&self, properties: HashMap<String, serde_json::Value>) {
        self.track_event("Analyze Folder", properties);
    }

    pub fn track_vulnerability_scan(&self) {
        // Deprecated: Use track_vulnerabilities with properties instead
    }

    // Flush method to ensure all events are sent before the program exits
    pub async fn flush(&self) {
        // Collect all pending tasks
        let mut tasks = Vec::new();
        {
            let mut pending_tasks = self.pending_tasks.lock().await;
            tasks.extend(pending_tasks.drain(..));
        }

        // Wait for all tasks to complete
        if !tasks.is_empty() {
            log::debug!("Waiting for {} telemetry tasks to complete", tasks.len());
            futures_util::future::join_all(tasks).await;
        }

        // Give a bit more time for network requests to complete
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
