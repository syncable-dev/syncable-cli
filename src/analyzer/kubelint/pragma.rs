//! Annotation-based rule ignoring.
//!
//! Supports `ignore-check.kube-linter.io/<check-name>` annotations
//! to disable specific checks for individual objects.

use crate::analyzer::kubelint::context::Object;
use std::collections::HashSet;

/// Prefix for kube-linter ignore annotations.
const IGNORE_ANNOTATION_PREFIX: &str = "ignore-check.kube-linter.io/";

/// Extract the set of ignored check names from an object's annotations.
pub fn get_ignored_checks(obj: &Object) -> HashSet<String> {
    let mut ignored = HashSet::new();

    if let Some(annotations) = obj.annotations() {
        for key in annotations.keys() {
            if let Some(check_name) = key.strip_prefix(IGNORE_ANNOTATION_PREFIX) {
                ignored.insert(check_name.to_string());
            }
        }
    }

    ignored
}

/// Check if a specific check should be ignored for an object.
pub fn should_ignore_check(obj: &Object, check_name: &str) -> bool {
    if let Some(annotations) = obj.annotations() {
        let annotation_key = format!("{}{}", IGNORE_ANNOTATION_PREFIX, check_name);
        annotations.contains_key(&annotation_key)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::kubelint::context::object::*;
    use crate::analyzer::kubelint::context::{K8sObject, ObjectMetadata};
    use std::collections::BTreeMap;

    fn make_object_with_annotations(annotations: BTreeMap<String, String>) -> Object {
        Object::new(
            ObjectMetadata::from_file("test.yaml"),
            K8sObject::Deployment(Box::new(DeploymentData {
                name: "test".to_string(),
                annotations: Some(annotations),
                ..Default::default()
            })),
        )
    }

    #[test]
    fn test_get_ignored_checks() {
        let mut annotations = BTreeMap::new();
        annotations.insert(
            "ignore-check.kube-linter.io/privileged-container".to_string(),
            "".to_string(),
        );
        annotations.insert(
            "ignore-check.kube-linter.io/latest-tag".to_string(),
            "reason".to_string(),
        );
        annotations.insert("other-annotation".to_string(), "value".to_string());

        let obj = make_object_with_annotations(annotations);
        let ignored = get_ignored_checks(&obj);

        assert!(ignored.contains("privileged-container"));
        assert!(ignored.contains("latest-tag"));
        assert_eq!(ignored.len(), 2);
    }

    #[test]
    fn test_should_ignore_check() {
        let mut annotations = BTreeMap::new();
        annotations.insert(
            "ignore-check.kube-linter.io/privileged-container".to_string(),
            "".to_string(),
        );

        let obj = make_object_with_annotations(annotations);

        assert!(should_ignore_check(&obj, "privileged-container"));
        assert!(!should_ignore_check(&obj, "latest-tag"));
    }
}
