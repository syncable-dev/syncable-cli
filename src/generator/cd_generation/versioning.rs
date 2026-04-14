//! CD-15 — Artifact Versioning & Image Tag Strategy
//!
//! Generates a consistent image tagging scheme across CI and CD:
//!
//! ```yaml
//! env:
//!   IMAGE_TAG: ${{ github.repository }}:${{ github.sha }}
//!   IMAGE_TAG_LATEST: ${{ github.repository }}:latest
//!   IMAGE_TAG_VERSION: ${{ github.repository }}:${{ github.ref_name }}
//! ```
//!
//! Tag matrix:
//! - Every push to main → SHA tag + `latest`
//! - Tag push (`v1.2.3`) → version tag + `latest`
//! - PR → SHA tag only (no `latest`)

// ── Types ─────────────────────────────────────────────────────────────────────

/// Image tag strategy for the CD pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagStrategy {
    /// Always use `<sha>` as the primary tag.
    Sha,
    /// Use semver when a tag is pushed, SHA otherwise.
    SemverWithShaFallback,
}

/// Represents the set of image tags to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageTags {
    /// Primary tag (always present), e.g. `ghcr.io/org/app:${{ github.sha }}`.
    pub sha_tag: String,
    /// Latest tag (only on default branch push), e.g. `ghcr.io/org/app:latest`.
    pub latest_tag: String,
    /// Version tag (only on tag push), e.g. `ghcr.io/org/app:${{ github.ref_name }}`.
    pub version_tag: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Computes the image tags for the given registry URL and image name.
pub fn compute_image_tags(registry_url: &str, image_name: &str) -> ImageTags {
    let base = if registry_url.is_empty() {
        image_name.to_string()
    } else {
        format!("{registry_url}/{image_name}")
    };

    ImageTags {
        sha_tag: format!("{base}:${{{{ github.sha }}}}"),
        latest_tag: format!("{base}:latest"),
        version_tag: format!("{base}:${{{{ github.ref_name }}}}"),
    }
}

/// Renders the `env:` block with image tag environment variables.
///
/// These are placed at the top level of the workflow YAML so all jobs
/// can reference `${{ env.IMAGE_TAG }}` etc.
pub fn render_versioning_env_block(tags: &ImageTags) -> String {
    format!(
        "\
env:
  IMAGE_TAG: {sha}
  IMAGE_TAG_LATEST: {latest}
  IMAGE_TAG_VERSION: {version}
",
        sha = tags.sha_tag,
        latest = tags.latest_tag,
        version = tags.version_tag,
    )
}

/// Renders a GitHub Actions step that computes the effective tag list
/// based on the event context (push to main, tag push, PR).
pub fn render_tag_resolution_step() -> String {
    "\
      - name: Compute image tags
        id: tags
        run: |
          TAGS=\"${{ env.IMAGE_TAG }}\"
          if [[ \"${{ github.ref }}\" == refs/heads/${{ github.event.repository.default_branch }} ]]; then
            TAGS=\"${TAGS},${{ env.IMAGE_TAG_LATEST }}\"
          fi
          if [[ \"${{ github.ref }}\" == refs/tags/v* ]]; then
            TAGS=\"${TAGS},${{ env.IMAGE_TAG_VERSION }},${{ env.IMAGE_TAG_LATEST }}\"
          fi
          echo \"tags=${TAGS}\" >> \"$GITHUB_OUTPUT\"
"
    .to_string()
}

/// Returns the expression to reference the computed tags in a build step.
pub fn tags_output_expression() -> &'static str {
    "${{ steps.tags.outputs.tags }}"
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha_tag_contains_github_sha() {
        let tags = compute_image_tags("ghcr.io", "my-app");
        assert!(tags.sha_tag.contains("github.sha"));
        assert!(tags.sha_tag.starts_with("ghcr.io/my-app:"));
    }

    #[test]
    fn latest_tag_is_literal_latest() {
        let tags = compute_image_tags("ghcr.io", "my-app");
        assert!(tags.latest_tag.ends_with(":latest"));
    }

    #[test]
    fn version_tag_contains_ref_name() {
        let tags = compute_image_tags("ghcr.io", "my-app");
        assert!(tags.version_tag.contains("github.ref_name"));
    }

    #[test]
    fn empty_registry_url_uses_image_name_only() {
        let tags = compute_image_tags("", "my-app");
        assert!(tags.sha_tag.starts_with("my-app:"));
    }

    #[test]
    fn env_block_contains_all_three_vars() {
        let tags = compute_image_tags("ghcr.io", "my-app");
        let block = render_versioning_env_block(&tags);
        assert!(block.contains("IMAGE_TAG:"));
        assert!(block.contains("IMAGE_TAG_LATEST:"));
        assert!(block.contains("IMAGE_TAG_VERSION:"));
    }

    #[test]
    fn tag_resolution_step_checks_default_branch() {
        let step = render_tag_resolution_step();
        assert!(step.contains("default_branch"));
    }

    #[test]
    fn tag_resolution_step_checks_semver_tag() {
        let step = render_tag_resolution_step();
        assert!(step.contains("refs/tags/v*"));
    }

    #[test]
    fn tag_resolution_step_outputs_to_github_output() {
        let step = render_tag_resolution_step();
        assert!(step.contains("GITHUB_OUTPUT"));
    }

    #[test]
    fn tags_output_expression_references_step() {
        let expr = tags_output_expression();
        assert!(expr.contains("steps.tags.outputs.tags"));
    }

    #[test]
    fn acr_registry_url_produces_correct_tags() {
        let tags = compute_image_tags("myapp.azurecr.io", "api");
        assert!(tags.sha_tag.starts_with("myapp.azurecr.io/api:"));
        assert!(tags.latest_tag.starts_with("myapp.azurecr.io/api:"));
    }

    #[test]
    fn gar_registry_url_produces_correct_tags() {
        let tags = compute_image_tags("us-docker.pkg.dev/my-project", "api");
        assert!(tags.sha_tag.starts_with("us-docker.pkg.dev/my-project/api:"));
    }
}
