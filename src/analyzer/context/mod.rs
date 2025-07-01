pub mod analysis;
pub(crate) mod file_analyzers;
pub(crate) mod helpers;
pub(crate) mod language_analyzers;
pub(crate) mod microservices;
pub(crate) mod project_type;
pub(crate) mod tech_specific;

pub use analysis::analyze_context; 