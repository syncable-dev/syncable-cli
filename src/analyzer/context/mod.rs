pub mod analysis;
pub(crate) mod file_analyzers;
pub(crate) mod health_detector;
pub(crate) mod helpers;
pub(crate) mod infra_detector;
pub(crate) mod language_analyzers;
pub(crate) mod microservices;
pub(crate) mod project_type;
pub(crate) mod tech_specific;

pub use analysis::analyze_context;
pub use health_detector::detect_health_endpoints;
pub use infra_detector::detect_infrastructure;
