//! Platform API client module
//!
//! Provides authenticated access to the Syncable Platform API for managing
//! organizations, projects, and other platform resources.
//!
//! # Example
//!
//! ```rust,ignore
//! use syncable_cli::platform::api::PlatformApiClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = PlatformApiClient::new()?;
//!
//!     // List organizations
//!     let orgs = client.list_organizations().await?;
//!     for org in orgs {
//!         println!("Organization: {}", org.name);
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod error;
pub mod types;

// Re-export commonly used items
pub use client::PlatformApiClient;
pub use error::{PlatformApiError, Result};
pub use types::{
    ArtifactRegistry, CloudCredentialStatus, CloudProvider, ClusterEntity, ClusterStatus,
    DeployedService, DeploymentConfig, DeploymentTaskStatus, Environment, Organization,
    PaginatedDeployments, PaginationInfo, Project, ProjectMember, RegistryStatus,
    TriggerDeploymentRequest, TriggerDeploymentResponse, UserProfile,
};
