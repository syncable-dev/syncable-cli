use crate::config::types::Config;
use crate::telemetry::user::UserId;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use reqwest::Client;
use tokio::sync::Mutex;
use std::sync::Arc;

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
    
    pub fn track_command_start(&self, command: &str) {
        let properties = self.create_common_properties();
        let client = self.http_client.clone();
        let cmd = command.to_string();
        let pending_tasks = self.pending_tasks.clone();
        
        log::debug!("Tracking command start: {}", cmd);
        
        // Send the event asynchronously
        let handle = tokio::spawn(async move {
            // Create the event payload according to PostHog API
            let payload = json!({
                "api_key": POSTHOG_PROJECT_API_KEY,
                "event": "command_start",
                "properties": {
                    "command": cmd,
                    "version": env!("CARGO_PKG_VERSION"),
                    "os": std::env::consts::OS,
                    "personal_id": rand::random::<u32>(),
                    "distinct_id": properties.get("distinct_id").unwrap_or(&json!("unknown")),
                },
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
                        log::debug!("Successfully sent telemetry event: command_start");
                    } else {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        log::warn!("Failed to send telemetry event 'command_start': HTTP {} - {}", status, body);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to send telemetry event 'command_start': {}", e);
                }
            }
        });
        
        // Keep track of the task
        let pending_tasks_clone = pending_tasks.clone();
        tokio::spawn(async move {
            pending_tasks_clone.lock().await.push(handle);
        });
    }
    
    pub fn track_command_complete(&self, command: &str, duration: Duration, success: bool) {
        let properties = self.create_common_properties();
        let client = self.http_client.clone();
        let cmd = command.to_string();
        let duration_ms = duration.as_millis() as u64;
        let pending_tasks = self.pending_tasks.clone();
        
        log::debug!("Tracking command complete: {}", cmd);
        
        // Send the event asynchronously
        let handle = tokio::spawn(async move {
            // Create the event payload according to PostHog API
            let payload = json!({
                "api_key": POSTHOG_PROJECT_API_KEY,
                "event": "command_complete",
                "properties": {
                    "command": cmd,
                    "duration_ms": duration_ms,
                    "success": success,
                    "version": env!("CARGO_PKG_VERSION"),
                    "os": std::env::consts::OS,
                    "personal_id": rand::random::<u32>(),
                    "distinct_id": properties.get("distinct_id").unwrap_or(&json!("unknown")),
                },
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
                        log::debug!("Successfully sent telemetry event: command_complete");
                    } else {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        log::warn!("Failed to send telemetry event 'command_complete': HTTP {} - {}", status, body);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to send telemetry event 'command_complete': {}", e);
                }
            }
        });
        
        // Keep track of the task
        let pending_tasks_clone = pending_tasks.clone();
        tokio::spawn(async move {
            pending_tasks_clone.lock().await.push(handle);
        });
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
                        let body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        log::warn!("Failed to send telemetry event '{}': HTTP {} - {}", event_name, status, body);
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
    
    // Specific methods for the three events mentioned
    pub fn track_security_scan(&self) {
        self.track_event("Security Scan", HashMap::new());
    }
    
    pub fn track_analyze_folder(&self) {
        self.track_event("Analyze Folder", HashMap::new());
    }
    
    pub fn track_vulnerability_scan(&self) {
        self.track_event("Vulnerability Scan", HashMap::new());
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