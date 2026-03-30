//! Docker Build & Tag Step Generator — CI-08
//!
//! Emitted only when `CiContext.has_dockerfile` is true.
//! Produces a `DockerBuildStep` with placeholder tokens for registry and image
//! name that are resolved by the token engine or wired in by the CD generator.

use crate::generator::ci_generation::{context::CiContext, schema::DockerBuildStep};

/// Returns `Some(DockerBuildStep)` when a Dockerfile is present, `None` otherwise.
///
/// The image tag is built from two unresolved placeholders plus the GitHub
/// Actions expression for the commit SHA, which is always available at runtime:
///   `{{REGISTRY_URL}}/{{IMAGE_NAME}}:${{ github.sha }}`
pub fn generate_docker_step(ctx: &CiContext) -> Option<DockerBuildStep> {
    if !ctx.has_dockerfile {
        return None;
    }

    Some(DockerBuildStep {
        image_tag: "{{REGISTRY_URL}}/{{IMAGE_NAME}}:${{ github.sha }}".to_string(),
        push: false,
        qemu: false,
        buildx: true,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::ci_generation::{context::CiContext, test_helpers::make_base_ctx};
    use tempfile::TempDir;

    fn ctx_with_dockerfile(has: bool) -> (CiContext, TempDir) {
        let dir = TempDir::new().unwrap();
        let ctx = CiContext { has_dockerfile: has, ..make_base_ctx(dir.path(), "") };
        (ctx, dir)
    }

    #[test]
    fn test_no_dockerfile_returns_none() {
        let (ctx, _dir) = ctx_with_dockerfile(false);
        assert!(generate_docker_step(&ctx).is_none());
    }

    #[test]
    fn test_dockerfile_present_returns_some() {
        let (ctx, _dir) = ctx_with_dockerfile(true);
        assert!(generate_docker_step(&ctx).is_some());
    }

    #[test]
    fn test_image_tag_contains_registry_placeholder() {
        let (ctx, _dir) = ctx_with_dockerfile(true);
        let step = generate_docker_step(&ctx).unwrap();
        assert!(step.image_tag.contains("{{REGISTRY_URL}}"));
    }

    #[test]
    fn test_image_tag_contains_image_name_placeholder() {
        let (ctx, _dir) = ctx_with_dockerfile(true);
        let step = generate_docker_step(&ctx).unwrap();
        assert!(step.image_tag.contains("{{IMAGE_NAME}}"));
    }

    #[test]
    fn test_image_tag_contains_github_sha_expression() {
        let (ctx, _dir) = ctx_with_dockerfile(true);
        let step = generate_docker_step(&ctx).unwrap();
        assert!(step.image_tag.contains("${{ github.sha }}"));
    }

    #[test]
    fn test_push_defaults_to_false() {
        let (ctx, _dir) = ctx_with_dockerfile(true);
        let step = generate_docker_step(&ctx).unwrap();
        assert!(!step.push);
    }

    #[test]
    fn test_buildx_defaults_to_true() {
        let (ctx, _dir) = ctx_with_dockerfile(true);
        let step = generate_docker_step(&ctx).unwrap();
        assert!(step.buildx);
    }

    #[test]
    fn test_qemu_defaults_to_false() {
        let (ctx, _dir) = ctx_with_dockerfile(true);
        let step = generate_docker_step(&ctx).unwrap();
        assert!(!step.qemu);
    }

    #[test]
    fn test_full_image_tag_format() {
        let (ctx, _dir) = ctx_with_dockerfile(true);
        let step = generate_docker_step(&ctx).unwrap();
        assert_eq!(
            step.image_tag,
            "{{REGISTRY_URL}}/{{IMAGE_NAME}}:${{ github.sha }}"
        );
    }
}
