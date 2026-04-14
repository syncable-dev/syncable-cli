//! CD-29 — Manual Dispatch Inputs
//!
//! Generates the `workflow_dispatch` block that lets operators trigger a deploy
//! manually from the GitHub Actions UI (or API).
//!
//! ```yaml
//! on:
//!   workflow_dispatch:
//!     inputs:
//!       image_tag:
//!         description: 'Image tag to deploy (leave empty for latest build)'
//!         required: false
//!         type: string
//!       environment:
//!         description: 'Target environment'
//!         required: true
//!         type: choice
//!         options:
//!           - development
//!           - staging
//!           - production
//! ```

/// A dispatch input definition.
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchInput {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub input_type: DispatchInputType,
}

/// Type discriminator for dispatch inputs.
#[derive(Debug, Clone, PartialEq)]
pub enum DispatchInputType {
    /// Free-form string input.
    StringInput { default: Option<String> },
    /// Constrained choice input.
    Choice { options: Vec<String> },
    /// Boolean toggle.
    Boolean { default: bool },
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Generates the standard set of dispatch inputs for a CD workflow.
///
/// Returns the `image_tag` (optional string) and `environment` (required choice)
/// inputs. Extra environments can be supplied; defaults to
/// `["development", "staging", "production"]`.
pub fn generate_dispatch_inputs(environments: &[String]) -> Vec<DispatchInput> {
    let env_options = if environments.is_empty() {
        vec![
            "development".to_string(),
            "staging".to_string(),
            "production".to_string(),
        ]
    } else {
        environments.to_vec()
    };

    vec![
        DispatchInput {
            name: "image_tag".to_string(),
            description: "Image tag to deploy (leave empty for latest build)".to_string(),
            required: false,
            input_type: DispatchInputType::StringInput { default: None },
        },
        DispatchInput {
            name: "environment".to_string(),
            description: "Target environment".to_string(),
            required: true,
            input_type: DispatchInputType::Choice {
                options: env_options,
            },
        },
        DispatchInput {
            name: "dry_run".to_string(),
            description: "Perform a dry-run without deploying".to_string(),
            required: false,
            input_type: DispatchInputType::Boolean { default: false },
        },
    ]
}

/// Renders the `workflow_dispatch:` block as YAML.
pub fn render_dispatch_yaml(inputs: &[DispatchInput]) -> String {
    let mut yaml = String::new();
    yaml.push_str("  workflow_dispatch:\n");

    if inputs.is_empty() {
        return yaml;
    }

    yaml.push_str("    inputs:\n");
    for input in inputs {
        yaml.push_str(&format!("      {}:\n", input.name));
        yaml.push_str(&format!(
            "        description: '{}'\n",
            input.description
        ));
        yaml.push_str(&format!(
            "        required: {}\n",
            input.required
        ));

        match &input.input_type {
            DispatchInputType::StringInput { default } => {
                yaml.push_str("        type: string\n");
                if let Some(d) = default {
                    yaml.push_str(&format!("        default: '{d}'\n"));
                }
            }
            DispatchInputType::Choice { options } => {
                yaml.push_str("        type: choice\n");
                yaml.push_str("        options:\n");
                for opt in options {
                    yaml.push_str(&format!("          - {opt}\n"));
                }
            }
            DispatchInputType::Boolean { default } => {
                yaml.push_str("        type: boolean\n");
                yaml.push_str(&format!("        default: {default}\n"));
            }
        }
    }

    yaml
}

/// Returns a GitHub Actions expression to read a dispatch input at runtime.
///
/// e.g. `${{ github.event.inputs.image_tag }}`
pub fn dispatch_input_expression(input_name: &str) -> String {
    format!("${{{{ github.event.inputs.{input_name} }}}}")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_default_inputs_count() {
        let inputs = generate_dispatch_inputs(&[]);
        assert_eq!(inputs.len(), 3);
    }

    #[test]
    fn generate_image_tag_input() {
        let inputs = generate_dispatch_inputs(&[]);
        let image_tag = &inputs[0];
        assert_eq!(image_tag.name, "image_tag");
        assert!(!image_tag.required);
        assert!(matches!(
            image_tag.input_type,
            DispatchInputType::StringInput { default: None }
        ));
    }

    #[test]
    fn generate_environment_input_defaults() {
        let inputs = generate_dispatch_inputs(&[]);
        let env = &inputs[1];
        assert_eq!(env.name, "environment");
        assert!(env.required);
        if let DispatchInputType::Choice { options } = &env.input_type {
            assert_eq!(options.len(), 3);
            assert_eq!(options[0], "development");
            assert_eq!(options[1], "staging");
            assert_eq!(options[2], "production");
        } else {
            panic!("Expected Choice type");
        }
    }

    #[test]
    fn generate_environment_custom_envs() {
        let envs = vec!["dev".to_string(), "prod".to_string()];
        let inputs = generate_dispatch_inputs(&envs);
        let env = &inputs[1];
        if let DispatchInputType::Choice { options } = &env.input_type {
            assert_eq!(options.len(), 2);
            assert_eq!(options[0], "dev");
            assert_eq!(options[1], "prod");
        } else {
            panic!("Expected Choice type");
        }
    }

    #[test]
    fn generate_dry_run_input() {
        let inputs = generate_dispatch_inputs(&[]);
        let dry_run = &inputs[2];
        assert_eq!(dry_run.name, "dry_run");
        assert!(!dry_run.required);
        assert!(matches!(
            dry_run.input_type,
            DispatchInputType::Boolean { default: false }
        ));
    }

    #[test]
    fn yaml_contains_workflow_dispatch() {
        let inputs = generate_dispatch_inputs(&[]);
        let yaml = render_dispatch_yaml(&inputs);
        assert!(yaml.contains("workflow_dispatch:"));
    }

    #[test]
    fn yaml_contains_inputs_block() {
        let inputs = generate_dispatch_inputs(&[]);
        let yaml = render_dispatch_yaml(&inputs);
        assert!(yaml.contains("inputs:"));
    }

    #[test]
    fn yaml_image_tag_type_string() {
        let inputs = generate_dispatch_inputs(&[]);
        let yaml = render_dispatch_yaml(&inputs);
        assert!(yaml.contains("type: string"));
    }

    #[test]
    fn yaml_environment_type_choice() {
        let inputs = generate_dispatch_inputs(&[]);
        let yaml = render_dispatch_yaml(&inputs);
        assert!(yaml.contains("type: choice"));
    }

    #[test]
    fn yaml_choice_options_listed() {
        let inputs = generate_dispatch_inputs(&[]);
        let yaml = render_dispatch_yaml(&inputs);
        assert!(yaml.contains("- development"));
        assert!(yaml.contains("- staging"));
        assert!(yaml.contains("- production"));
    }

    #[test]
    fn yaml_boolean_input() {
        let inputs = generate_dispatch_inputs(&[]);
        let yaml = render_dispatch_yaml(&inputs);
        assert!(yaml.contains("type: boolean"));
        assert!(yaml.contains("default: false"));
    }

    #[test]
    fn yaml_empty_inputs() {
        let yaml = render_dispatch_yaml(&[]);
        assert!(yaml.contains("workflow_dispatch:"));
        assert!(!yaml.contains("inputs:"));
    }

    #[test]
    fn dispatch_expression_format() {
        let expr = dispatch_input_expression("image_tag");
        assert_eq!(expr, "${{ github.event.inputs.image_tag }}");
    }

    #[test]
    fn dispatch_expression_environment() {
        let expr = dispatch_input_expression("environment");
        assert_eq!(expr, "${{ github.event.inputs.environment }}");
    }

    #[test]
    fn yaml_string_with_default() {
        let inputs = vec![DispatchInput {
            name: "version".to_string(),
            description: "App version".to_string(),
            required: false,
            input_type: DispatchInputType::StringInput {
                default: Some("latest".to_string()),
            },
        }];
        let yaml = render_dispatch_yaml(&inputs);
        assert!(yaml.contains("default: 'latest'"));
    }

    #[test]
    fn render_all_inputs_order() {
        let inputs = generate_dispatch_inputs(&[]);
        let yaml = render_dispatch_yaml(&inputs);
        let image_pos = yaml.find("image_tag:").unwrap();
        let env_pos = yaml.find("environment:").unwrap();
        let dry_pos = yaml.find("dry_run:").unwrap();
        assert!(image_pos < env_pos);
        assert!(env_pos < dry_pos);
    }
}
