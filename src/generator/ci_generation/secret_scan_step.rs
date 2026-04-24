//! Secret / Credential Leak Scan Step Generator — CI-10
//!
//! Always emitted regardless of platform or language. Gitleaks runs on the
//! repository checkout — no Docker image or build artifact required.

use crate::generator::ci_generation::schema::SecretScanStep;

/// Returns a `SecretScanStep`. Unconditional — every pipeline gets this step.
pub fn generate_secret_scan_step() -> SecretScanStep {
    SecretScanStep {
        github_token_expr: "${{ secrets.GITHUB_TOKEN }}".to_string(),
        gitleaks_license_secret: None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_scan_step_is_always_produced() {
        let _ = generate_secret_scan_step();
    }

    #[test]
    fn test_github_token_is_builtin_expression() {
        let step = generate_secret_scan_step();
        assert_eq!(step.github_token_expr, "${{ secrets.GITHUB_TOKEN }}");
    }

    #[test]
    fn test_gitleaks_license_defaults_to_none() {
        let step = generate_secret_scan_step();
        assert!(step.gitleaks_license_secret.is_none());
    }

    #[test]
    fn test_secret_scan_step_serializes() {
        let step = generate_secret_scan_step();
        let serialized = serde_json::to_string(&step);
        assert!(serialized.is_ok());
    }
}
