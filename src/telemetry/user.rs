use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserId {
    pub id: String,
    pub first_seen: chrono::DateTime<chrono::Utc>,
}

impl UserId {
    pub fn load_or_create() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_user_id_path()?;
        
        // Try to load existing user ID
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let user_id: UserId = serde_json::from_str(&content)?;
            Ok(user_id)
        } else {
            // Create new user ID
            let user_id = UserId {
                id: Uuid::new_v4().to_string(),
                first_seen: chrono::Utc::now(),
            };
            
            // Save to file
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = serde_json::to_string_pretty(&user_id)?;
            fs::write(&config_path, content)?;
            
            Ok(user_id)
        }
    }
    
    fn get_user_id_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = config_dir().ok_or("Could not determine config directory")?;
        Ok(config_dir.join("syncable-cli").join("user_id"))
    }
}