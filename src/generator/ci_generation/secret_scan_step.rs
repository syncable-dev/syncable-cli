//! Secret / Credential Leak Scan Step Generator — CI-10
//!
//! Always emitted regardless of platform or language. Gitleaks runs on the
//! repository checkout — no Docker image or build artifact required.

use crate::generator::ci_generation::schema::SecretScanStep;

/// Returns a `SecretScanStep`. Unconditional — every pipeline gets this step.
pub fn generate_secret_scan_step() -> SecretScanStep {
    SecretScanStep
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_scan_step_is_always_produced() {
        // SecretScanStep is a unit struct — constructing it succeeds and
        // confirms the function returns without conditions.
        let _ = generate_secret_scan_step();
    }

    #[test]
    fn test_secret_scan_step_serializes() {
        let step = generate_secret_scan_step();
        let serialized = serde_json::to_string(&step);
        assert!(serialized.is_ok());
    }
}
