//! # Runtime Detection Module
//!
//! Handles detection of JavaScript/TypeScript runtimes and package managers

pub mod detection;
pub mod javascript;

pub use javascript::{
    DetectionConfidence, JavaScriptRuntime, PackageManager, RuntimeDetectionResult, RuntimeDetector,
};

pub use detection::RuntimeDetectionEngine;
