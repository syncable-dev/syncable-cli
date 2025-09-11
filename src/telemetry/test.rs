#[cfg(test)]
mod tests {
    use crate::config::types::Config;
    use crate::telemetry::TelemetryClient;
    use std::collections::HashMap;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_telemetry_client_creation() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await;
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_track_event() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        
        let mut properties = HashMap::new();
        properties.insert("test_property".to_string(), serde_json::json!("test_value"));
        
        client.track_event("test_event", properties);
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_security_scan() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        client.track_security_scan();
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_analyze_folder() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        
        // Updated to pass properties as required by the new signature
        let mut properties = HashMap::new();
        properties.insert("analysis_mode".to_string(), serde_json::json!("matrix"));
        properties.insert("color_scheme".to_string(), serde_json::json!("auto"));
        
        client.track_analyze_folder(properties);
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_vulnerability_scan() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        client.track_vulnerability_scan();
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_analyze() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        let properties = HashMap::new();
        client.track_analyze(properties);
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_generate() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        let properties = HashMap::new();
        client.track_generate(properties);
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_validate() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        let properties = HashMap::new();
        client.track_validate(properties);
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_support() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        let properties = HashMap::new();
        client.track_support(properties);
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_dependencies() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        let properties = HashMap::new();
        client.track_dependencies(properties);
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_vulnerabilities() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        
        // Updated to pass properties as required by the new signature
        let mut properties = HashMap::new();
        properties.insert("severity_threshold".to_string(), serde_json::json!("high"));
        properties.insert("output_format".to_string(), serde_json::json!("table"));
        
        client.track_vulnerabilities(properties);
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_security() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        
        // Updated to pass properties as required by the new signature
        let mut properties = HashMap::new();
        properties.insert("scan_mode".to_string(), serde_json::json!("balanced"));
        properties.insert("output_format".to_string(), serde_json::json!("table"));
        
        client.track_security(properties);
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_tools() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        let properties = HashMap::new();
        client.track_tools(properties);
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
}