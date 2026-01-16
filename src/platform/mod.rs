//! Platform module for Syncable platform integration
//!
//! This module provides:
//! - Session state management for tracking selected projects and organizations
//! - API client for interacting with the Syncable Platform API

pub mod api;
pub mod session;

pub use session::PlatformSession;
