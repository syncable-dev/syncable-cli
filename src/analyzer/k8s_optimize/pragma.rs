//! Annotation-based rule ignoring for k8s-optimize.
//!
//! Supports `ignore-check.k8s-optimize.io/<check-code>` annotations
//! to disable specific optimization checks for individual objects.
//!
//! # Example
//!
//! ```yaml
//! apiVersion: apps/v1
//! kind: Deployment
//! metadata:
//!   name: my-app
//!   annotations:
//!     # Ignore the high CPU request check for this deployment
//!     ignore-check.k8s-optimize.io/K8S-OPT-005: "Batch processing requires high CPU"
//!     # Ignore the excessive CPU ratio check
//!     ignore-check.k8s-optimize.io/K8S-OPT-007: ""
//! spec:
//!   # ...
//! ```

use std::collections::HashSet;

/// Prefix for k8s-optimize ignore annotations.
pub const IGNORE_ANNOTATION_PREFIX: &str = "ignore-check.k8s-optimize.io/";

/// Extract the set of ignored rule codes from an object's annotations.
///
/// # Arguments
///
/// * `annotations` - Optional map of annotations from the object metadata
///
/// # Returns
///
/// A set of rule codes (e.g., "K8S-OPT-001", "K8S-OPT-005") that should be ignored.
pub fn get_ignored_rules(
    annotations: Option<&std::collections::BTreeMap<String, String>>,
) -> HashSet<String> {
    let mut ignored = HashSet::new();

    if let Some(annotations) = annotations {
        for key in annotations.keys() {
            if let Some(rule_code) = key.strip_prefix(IGNORE_ANNOTATION_PREFIX) {
                ignored.insert(rule_code.to_string());
            }
        }
    }

    ignored
}

/// Check if a specific rule should be ignored for an object.
///
/// # Arguments
///
/// * `annotations` - Optional map of annotations from the object metadata
/// * `rule_code` - The rule code to check (e.g., "K8S-OPT-001")
///
/// # Returns
///
/// `true` if the rule should be ignored, `false` otherwise.
pub fn should_ignore_rule(
    annotations: Option<&std::collections::BTreeMap<String, String>>,
    rule_code: &str,
) -> bool {
    if let Some(annotations) = annotations {
        let annotation_key = format!("{}{}", IGNORE_ANNOTATION_PREFIX, rule_code);
        annotations.contains_key(&annotation_key)
    } else {
        false
    }
}

/// Extract annotations from a YAML value's metadata.
pub fn extract_annotations(
    yaml: &serde_yaml::Value,
) -> Option<std::collections::BTreeMap<String, String>> {
    let metadata = yaml.get("metadata")?;
    let annotations = metadata.get("annotations")?;
    let annotations_map = annotations.as_mapping()?;

    let mut result = std::collections::BTreeMap::new();
    for (key, value) in annotations_map {
        if let (Some(k), Some(v)) = (key.as_str(), value.as_str()) {
            result.insert(k.to_string(), v.to_string());
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Get the reason for ignoring a rule (if provided in the annotation value).
pub fn get_ignore_reason(
    annotations: Option<&std::collections::BTreeMap<String, String>>,
    rule_code: &str,
) -> Option<String> {
    let annotations = annotations?;
    let annotation_key = format!("{}{}", IGNORE_ANNOTATION_PREFIX, rule_code);
    let value = annotations.get(&annotation_key)?;

    if value.is_empty() {
        None
    } else {
        Some(value.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_get_ignored_rules() {
        let mut annotations = BTreeMap::new();
        annotations.insert(
            "ignore-check.k8s-optimize.io/K8S-OPT-001".to_string(),
            "".to_string(),
        );
        annotations.insert(
            "ignore-check.k8s-optimize.io/K8S-OPT-005".to_string(),
            "Batch job needs high CPU".to_string(),
        );
        annotations.insert("other-annotation".to_string(), "value".to_string());

        let ignored = get_ignored_rules(Some(&annotations));

        assert!(ignored.contains("K8S-OPT-001"));
        assert!(ignored.contains("K8S-OPT-005"));
        assert!(!ignored.contains("K8S-OPT-002"));
        assert_eq!(ignored.len(), 2);
    }

    #[test]
    fn test_should_ignore_rule() {
        let mut annotations = BTreeMap::new();
        annotations.insert(
            "ignore-check.k8s-optimize.io/K8S-OPT-001".to_string(),
            "".to_string(),
        );

        assert!(should_ignore_rule(Some(&annotations), "K8S-OPT-001"));
        assert!(!should_ignore_rule(Some(&annotations), "K8S-OPT-002"));
        assert!(!should_ignore_rule(None, "K8S-OPT-001"));
    }

    #[test]
    fn test_get_ignore_reason() {
        let mut annotations = BTreeMap::new();
        annotations.insert(
            "ignore-check.k8s-optimize.io/K8S-OPT-001".to_string(),
            "".to_string(),
        );
        annotations.insert(
            "ignore-check.k8s-optimize.io/K8S-OPT-005".to_string(),
            "Batch job needs high CPU".to_string(),
        );

        assert_eq!(get_ignore_reason(Some(&annotations), "K8S-OPT-001"), None);
        assert_eq!(
            get_ignore_reason(Some(&annotations), "K8S-OPT-005"),
            Some("Batch job needs high CPU".to_string())
        );
        assert_eq!(get_ignore_reason(Some(&annotations), "K8S-OPT-002"), None);
    }

    #[test]
    fn test_extract_annotations() {
        let yaml = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test
  annotations:
    ignore-check.k8s-optimize.io/K8S-OPT-001: ""
    other: value
"#,
        )
        .unwrap();

        let annotations = extract_annotations(&yaml);
        assert!(annotations.is_some());

        let annotations = annotations.unwrap();
        assert!(annotations.contains_key("ignore-check.k8s-optimize.io/K8S-OPT-001"));
        assert!(annotations.contains_key("other"));
    }
}
