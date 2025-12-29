//! Service account detection templates.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting default service account usage.
pub struct ServiceAccountTemplate;

impl Template for ServiceAccountTemplate {
    fn key(&self) -> &str {
        "service-account"
    }

    fn human_name(&self) -> &str {
        "Default Service Account"
    }

    fn description(&self) -> &str {
        "Detects pods using the default service account"
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
        Ok(Box::new(DefaultServiceAccountCheck))
    }
}

struct DefaultServiceAccountCheck;

impl CheckFunc for DefaultServiceAccountCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            let service_account = pod_spec.service_account_name.as_deref();

            // Check if using default service account or no service account specified
            if service_account.is_none() || service_account == Some("default") {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "Object '{}' is using the default service account",
                        object.name()
                    ),
                    remediation: Some(
                        "Create and use a dedicated ServiceAccount with only necessary permissions."
                            .to_string(),
                    ),
                });
            }
        }

        diagnostics
    }
}

/// Template for detecting deprecated serviceAccount field.
pub struct DeprecatedServiceAccountFieldTemplate;

impl Template for DeprecatedServiceAccountFieldTemplate {
    fn key(&self) -> &str {
        "deprecated-service-account-field"
    }

    fn human_name(&self) -> &str {
        "Deprecated Service Account Field"
    }

    fn description(&self) -> &str {
        "Detects use of the deprecated serviceAccount field instead of serviceAccountName"
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
        // Note: This check is a placeholder - the current parser doesn't distinguish
        // between serviceAccount and serviceAccountName fields
        Ok(Box::new(DeprecatedServiceAccountFieldCheck))
    }
}

struct DeprecatedServiceAccountFieldCheck;

impl CheckFunc for DeprecatedServiceAccountFieldCheck {
    fn check(&self, _object: &Object) -> Vec<Diagnostic> {
        // Note: The current YAML parser unifies serviceAccount and serviceAccountName
        // into service_account_name, so we can't detect the deprecated field usage.
        // This would require raw YAML inspection to implement properly.
        Vec::new()
    }
}
