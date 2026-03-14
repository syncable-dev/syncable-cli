//! CI Pipeline Schema — CI-14
//!
//! Defines the canonical, platform-agnostic `CiPipeline` intermediate
//! representation. All template builders render from this struct, not
//! directly from `CiContext`. This decouples context collection from
//! output formatting and allows future agent patching of individual steps.

// TODO CI-14: implement CiPipeline struct and all step types
