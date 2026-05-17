//! Container Image Security Scan Step Generator — CI-09
//!
//! Emitted only when a Docker build step is present. Takes the output of
//! `generate_docker_step` directly — the dependency is encoded in the type:
//! `Option<DockerBuildStep>` in, `Option<ImageScanStep>` out.

use crate::generator::ci_generation::schema::{DockerBuildStep, ImageScanStep};

/// Returns `Some(ImageScanStep)` when a Docker build step exists, `None` otherwise.
///
/// The scan targets the same image reference produced by the Docker build step,
/// failing the job on any CRITICAL or HIGH severity finding.
pub fn generate_image_scan_step(docker: &Option<DockerBuildStep>) -> Option<ImageScanStep> {
    docker.as_ref().map(|d| ImageScanStep {
        image_ref: d.image_tag.clone(),
        fail_on_severity: "CRITICAL,HIGH".to_string(),
        format: "sarif".to_string(),
        output: "trivy-results.sarif".to_string(),
        upload_sarif: true,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::ci_generation::schema::DockerBuildStep;

    fn make_docker_step() -> DockerBuildStep {
        DockerBuildStep {
            image_tag: "{{REGISTRY_URL}}/{{IMAGE_NAME}}:${{ github.sha }}".to_string(),
            push: false,
            qemu: false,
            buildx: true,
        }
    }

    #[test]
    fn test_none_docker_yields_none_scan() {
        assert!(generate_image_scan_step(&None).is_none());
    }

    #[test]
    fn test_some_docker_yields_some_scan() {
        let docker = Some(make_docker_step());
        assert!(generate_image_scan_step(&docker).is_some());
    }

    #[test]
    fn test_image_ref_matches_docker_tag() {
        let docker = Some(make_docker_step());
        let scan = generate_image_scan_step(&docker).unwrap();
        assert_eq!(scan.image_ref, make_docker_step().image_tag);
    }

    #[test]
    fn test_fail_on_severity_is_critical_and_high() {
        let docker = Some(make_docker_step());
        let scan = generate_image_scan_step(&docker).unwrap();
        assert_eq!(scan.fail_on_severity, "CRITICAL,HIGH");
    }

    #[test]
    fn test_format_is_sarif() {
        let docker = Some(make_docker_step());
        let scan = generate_image_scan_step(&docker).unwrap();
        assert_eq!(scan.format, "sarif");
    }

    #[test]
    fn test_output_is_trivy_sarif_file() {
        let docker = Some(make_docker_step());
        let scan = generate_image_scan_step(&docker).unwrap();
        assert_eq!(scan.output, "trivy-results.sarif");
    }

    #[test]
    fn test_upload_sarif_is_true() {
        let docker = Some(make_docker_step());
        let scan = generate_image_scan_step(&docker).unwrap();
        assert!(scan.upload_sarif);
    }

    #[test]
    fn test_custom_image_tag_propagated() {
        let docker = Some(DockerBuildStep {
            image_tag: "ghcr.io/myorg/myapp:abc123".to_string(),
            push: true,
            qemu: false,
            buildx: true,
        });
        let scan = generate_image_scan_step(&docker).unwrap();
        assert_eq!(scan.image_ref, "ghcr.io/myorg/myapp:abc123");
    }
}
