//! Credential storage and retrieval for Syncable authentication
//!
//! Stores authentication tokens in ~/.syncable.toml

use crate::config::{load_config, save_global_config, types::SyncableAuth};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

/// Save credentials to global config file
pub fn save_credentials(
    access_token: &str,
    refresh_token: Option<&str>,
    user_email: Option<&str>,
    expires_in_secs: Option<u64>,
) -> Result<()> {
    let mut config = load_config(None).unwrap_or_default();

    let expires_at = expires_in_secs.map(|secs| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + secs
    });

    config.syncable_auth = SyncableAuth {
        access_token: Some(access_token.to_string()),
        refresh_token: refresh_token.map(|s| s.to_string()),
        expires_at,
        user_email: user_email.map(|s| s.to_string()),
    };

    save_global_config(&config)?;
    Ok(())
}

/// Get the current access token if valid
pub fn get_access_token() -> Option<String> {
    let config = load_config(None).ok()?;

    // Check expiry
    if let Some(expires_at) = config.syncable_auth.expires_at {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        if now > expires_at {
            return None; // Token expired
        }
    }

    config.syncable_auth.access_token
}

/// Get the authenticated user's email
pub fn get_user_email() -> Option<String> {
    let config = load_config(None).ok()?;
    config.syncable_auth.user_email
}

/// Check if the user is currently authenticated with a valid token
pub fn is_authenticated() -> bool {
    get_access_token().is_some()
}

/// Get authentication status including expiry info
pub fn get_auth_status() -> AuthStatus {
    let config = match load_config(None) {
        Ok(c) => c,
        Err(_) => return AuthStatus::NotAuthenticated,
    };

    match &config.syncable_auth.access_token {
        None => AuthStatus::NotAuthenticated,
        Some(_) => {
            if let Some(expires_at) = config.syncable_auth.expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                if now > expires_at {
                    return AuthStatus::Expired;
                }

                AuthStatus::Authenticated {
                    email: config.syncable_auth.user_email.clone(),
                    expires_at: Some(expires_at),
                }
            } else {
                AuthStatus::Authenticated {
                    email: config.syncable_auth.user_email.clone(),
                    expires_at: None,
                }
            }
        }
    }
}

/// Clear stored credentials (logout)
pub fn clear_credentials() -> Result<()> {
    let mut config = load_config(None).unwrap_or_default();
    config.syncable_auth = SyncableAuth::default();
    save_global_config(&config)?;
    Ok(())
}

/// Authentication status enum
#[derive(Debug)]
pub enum AuthStatus {
    NotAuthenticated,
    Expired,
    Authenticated {
        email: Option<String>,
        expires_at: Option<u64>,
    },
}
