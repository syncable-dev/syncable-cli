//! Platform module for Syncable platform integration
//!
//! This module provides session state management for the Syncable platform,
//! tracking selected projects and organizations across CLI sessions.

pub mod session;

pub use session::PlatformSession;
