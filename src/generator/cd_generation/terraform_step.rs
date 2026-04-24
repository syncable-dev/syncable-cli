//! CD-16 — Terraform Integration Step (Optional)
//!
//! Generates Terraform init/plan/apply steps gated by `CdContext.has_terraform`.
//! Injects `TF_VAR_image_tag` so Terraform can reference the deployed image.
//!
//! ```yaml
//! - name: Terraform Init
//!   uses: hashicorp/setup-terraform@v3
//!
//! - name: Terraform Plan
//!   run: terraform plan -input=false
//!   env:
//!     TF_VAR_image_tag: ${{ env.IMAGE_TAG }}
//!
//! - name: Terraform Apply
//!   if: github.ref == 'refs/heads/main'
//!   run: terraform apply -auto-approve -input=false
//! ```

use super::schema::TerraformStep;

// ── Public API ────────────────────────────────────────────────────────────────

/// Generates a `TerraformStep` from the context.
///
/// `terraform_dir` is the working directory (e.g. `"terraform"`, `"infra"`).
/// `auto_approve` should be `false` for production environments.
pub fn generate_terraform_step(
    terraform_dir: &str,
    auto_approve: bool,
) -> TerraformStep {
    TerraformStep {
        working_directory: terraform_dir.to_string(),
        version: "{{TERRAFORM_VERSION}}".to_string(),
        backend_config: vec![],
        auto_approve,
    }
}

/// Renders the Terraform steps as a GitHub Actions YAML snippet.
pub fn render_terraform_yaml(step: &TerraformStep, default_branch: &str) -> String {
    let mut yaml = String::new();

    // Setup Terraform
    yaml.push_str("      - name: Set up Terraform\n");
    yaml.push_str("        uses: hashicorp/setup-terraform@v3\n");
    if step.version != "{{TERRAFORM_VERSION}}" {
        yaml.push_str(&format!(
            "        with:\n          terraform_version: {}\n",
            step.version
        ));
    }
    yaml.push('\n');

    // Terraform Init
    yaml.push_str("      - name: Terraform Init\n");
    yaml.push_str(&format!(
        "        working-directory: {}\n",
        step.working_directory
    ));
    let mut init_cmd = "terraform init -input=false".to_string();
    for bc in &step.backend_config {
        init_cmd.push_str(&format!(" {bc}"));
    }
    yaml.push_str(&format!("        run: {init_cmd}\n\n"));

    // Terraform Plan
    yaml.push_str("      - name: Terraform Plan\n");
    yaml.push_str(&format!(
        "        working-directory: {}\n",
        step.working_directory
    ));
    yaml.push_str("        run: terraform plan -input=false -out=tfplan\n");
    yaml.push_str("        env:\n");
    yaml.push_str("          TF_VAR_image_tag: ${{ env.IMAGE_TAG }}\n\n");

    // Terraform Apply
    yaml.push_str("      - name: Terraform Apply\n");
    yaml.push_str(&format!(
        "        if: github.ref == 'refs/heads/{default_branch}'\n"
    ));
    yaml.push_str(&format!(
        "        working-directory: {}\n",
        step.working_directory
    ));
    if step.auto_approve {
        yaml.push_str("        run: terraform apply -auto-approve -input=false tfplan\n");
    } else {
        yaml.push_str("        run: terraform apply -input=false tfplan\n");
    }
    yaml.push_str("        env:\n");
    yaml.push_str("          TF_VAR_image_tag: ${{ env.IMAGE_TAG }}\n");

    yaml
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_step_sets_working_directory() {
        let step = generate_terraform_step("terraform", true);
        assert_eq!(step.working_directory, "terraform");
    }

    #[test]
    fn generate_step_defaults_to_placeholder_version() {
        let step = generate_terraform_step("infra", false);
        assert_eq!(step.version, "{{TERRAFORM_VERSION}}");
    }

    #[test]
    fn generate_step_auto_approve_flag() {
        let step = generate_terraform_step("tf", true);
        assert!(step.auto_approve);
        let step2 = generate_terraform_step("tf", false);
        assert!(!step2.auto_approve);
    }

    #[test]
    fn yaml_contains_setup_terraform() {
        let step = generate_terraform_step("terraform", true);
        let yaml = render_terraform_yaml(&step, "main");
        assert!(yaml.contains("hashicorp/setup-terraform@v3"));
    }

    #[test]
    fn yaml_contains_terraform_init() {
        let step = generate_terraform_step("infra", false);
        let yaml = render_terraform_yaml(&step, "main");
        assert!(yaml.contains("Terraform Init"));
        assert!(yaml.contains("terraform init -input=false"));
    }

    #[test]
    fn yaml_contains_terraform_plan() {
        let step = generate_terraform_step("terraform", false);
        let yaml = render_terraform_yaml(&step, "main");
        assert!(yaml.contains("Terraform Plan"));
        assert!(yaml.contains("terraform plan"));
    }

    #[test]
    fn yaml_injects_tf_var_image_tag() {
        let step = generate_terraform_step("terraform", false);
        let yaml = render_terraform_yaml(&step, "main");
        assert!(yaml.contains("TF_VAR_image_tag"));
    }

    #[test]
    fn yaml_apply_gated_by_branch() {
        let step = generate_terraform_step("terraform", false);
        let yaml = render_terraform_yaml(&step, "main");
        assert!(yaml.contains("if: github.ref == 'refs/heads/main'"));
    }

    #[test]
    fn yaml_apply_auto_approve_when_set() {
        let step = generate_terraform_step("terraform", true);
        let yaml = render_terraform_yaml(&step, "main");
        assert!(yaml.contains("-auto-approve"));
    }

    #[test]
    fn yaml_apply_no_auto_approve_when_unset() {
        let step = generate_terraform_step("terraform", false);
        let yaml = render_terraform_yaml(&step, "main");
        // Should have "terraform apply -input=false tfplan" without -auto-approve
        let apply_line = yaml
            .lines()
            .find(|l| l.contains("terraform apply"))
            .unwrap();
        assert!(!apply_line.contains("-auto-approve"));
    }

    #[test]
    fn yaml_uses_working_directory() {
        let step = generate_terraform_step("infra/prod", false);
        let yaml = render_terraform_yaml(&step, "main");
        assert!(yaml.contains("working-directory: infra/prod"));
    }

    #[test]
    fn yaml_custom_branch() {
        let step = generate_terraform_step("terraform", false);
        let yaml = render_terraform_yaml(&step, "master");
        assert!(yaml.contains("refs/heads/master"));
    }

    #[test]
    fn yaml_backend_config() {
        let mut step = generate_terraform_step("terraform", false);
        step.backend_config = vec!["-backend-config=env/prod.hcl".to_string()];
        let yaml = render_terraform_yaml(&step, "main");
        assert!(yaml.contains("-backend-config=env/prod.hcl"));
    }

    #[test]
    fn yaml_no_version_with_when_placeholder() {
        let step = generate_terraform_step("terraform", false);
        let yaml = render_terraform_yaml(&step, "main");
        // When version is placeholder, terraform_version with: block should not be emitted
        assert!(!yaml.contains("terraform_version:"));
    }

    #[test]
    fn yaml_version_with_when_set() {
        let mut step = generate_terraform_step("terraform", false);
        step.version = "1.7.0".to_string();
        let yaml = render_terraform_yaml(&step, "main");
        assert!(yaml.contains("terraform_version: 1.7.0"));
    }
}
