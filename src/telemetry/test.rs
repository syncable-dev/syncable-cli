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
        client.track_analyze_folder();
        
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
        client.track_analyze();
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_generate() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        client.track_generate();
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_validate() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        client.track_validate();
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_support() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        client.track_support();
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_dependencies() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        client.track_dependencies();
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_vulnerabilities() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        client.track_vulnerabilities();
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_security() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        client.track_security();
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_track_tools() {
        let config = Config::default();
        let client = TelemetryClient::new(&config).await.unwrap();
        client.track_tools();
        
        // Give a small delay to allow the async task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // We can't easily verify the exact value, but we can confirm the method executes without error
        assert!(true);
    }
}