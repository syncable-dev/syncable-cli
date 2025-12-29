//! Latest tag detection template.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting :latest image tags.
pub struct LatestTagTemplate;

impl Template for LatestTagTemplate {
    fn key(&self) -> &str {
        "latest-tag"
    }

    fn human_name(&self) -> &str {
        "Latest Tag"
    }

    fn description(&self) -> &str {
        "Detects containers using the :latest tag or no tag at all"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::default()
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(LatestTagCheck))
    }
}

struct LatestTagCheck;

impl CheckFunc for LatestTagCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                if let Some(image) = &container.image {
                    let uses_latest = image.ends_with(":latest")
                        || (!image.contains(':') && !image.contains('@'));

                    if uses_latest {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "Container '{}' uses image '{}' with latest tag or no tag",
                                container.name, image
                            ),
                            remediation: Some(
                                "Use a specific image tag instead of :latest for reproducibility."
                                    .to_string(),
                            ),
                        });
                    }
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::kubelint::parser::yaml::parse_yaml;

    #[test]
    fn test_latest_tag_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: latest-deploy
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:latest
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = LatestTagCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("latest"));
    }

    #[test]
    fn test_no_tag_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: no-tag-deploy
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = LatestTagCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_specific_tag_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: versioned-deploy
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = LatestTagCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_digest_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: digest-deploy
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx@sha256:abc123
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = LatestTagCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }
}
