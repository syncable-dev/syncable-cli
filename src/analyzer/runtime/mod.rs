//! # Runtime Detection Module
//! 
//! Handles detection of JavaScript/TypeScript runtimes and package managers

pub mod javascript;
pub mod detection;

pub use javascript::{
    JavaScriptRuntime, PackageManager, RuntimeDetectionResult, DetectionConfidence, RuntimeDetector
};

pub use detection::RuntimeDetectionEngine;