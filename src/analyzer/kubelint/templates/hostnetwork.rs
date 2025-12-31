//! Host network/PID/IPC detection templates.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting pods using hostNetwork.
pub struct HostNetworkTemplate;

impl Template for HostNetworkTemplate {
    fn key(&self) -> &str {
        "host-network"
    }

    fn human_name(&self) -> &str {
        "Host Network"
    }

    fn description(&self) -> &str {
        "Detects pods using host network namespace"
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
        Ok(Box::new(HostNetworkCheck))
    }
}

struct HostNetworkCheck;

impl CheckFunc for HostNetworkCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object)
            && pod_spec.host_network == Some(true) {
                diagnostics.push(Diagnostic {
                    message: "Pod is configured to use the host's network namespace".to_string(),
                    remediation: Some(
                        "Remove hostNetwork: true unless absolutely necessary. \
                         Using host network grants access to all network interfaces on the host."
                            .to_string(),
                    ),
                });
            }

        diagnostics
    }
}

/// Template for detecting pods using hostPID.
pub struct HostPIDTemplate;

impl Template for HostPIDTemplate {
    fn key(&self) -> &str {
        "host-pid"
    }

    fn human_name(&self) -> &str {
        "Host PID"
    }

    fn description(&self) -> &str {
        "Detects pods using host PID namespace"
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
        Ok(Box::new(HostPIDCheck))
    }
}

struct HostPIDCheck;

impl CheckFunc for HostPIDCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object)
            && pod_spec.host_pid == Some(true) {
                diagnostics.push(Diagnostic {
                    message: "Pod is configured to use the host's PID namespace".to_string(),
                    remediation: Some(
                        "Remove hostPID: true unless absolutely necessary. \
                         Using host PID allows processes in the container to see and signal all \
                         processes on the host."
                            .to_string(),
                    ),
                });
            }

        diagnostics
    }
}

/// Template for detecting pods using hostIPC.
pub struct HostIPCTemplate;

impl Template for HostIPCTemplate {
    fn key(&self) -> &str {
        "host-ipc"
    }

    fn human_name(&self) -> &str {
        "Host IPC"
    }

    fn description(&self) -> &str {
        "Detects pods using host IPC namespace"
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
        Ok(Box::new(HostIPCCheck))
    }
}

struct HostIPCCheck;

impl CheckFunc for HostIPCCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object)
            && pod_spec.host_ipc == Some(true) {
                diagnostics.push(Diagnostic {
                    message: "Pod is configured to use the host's IPC namespace".to_string(),
                    remediation: Some(
                        "Remove hostIPC: true unless absolutely necessary. \
                         Using host IPC allows processes to communicate with all processes on the host."
                            .to_string(),
                    ),
                });
            }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::kubelint::parser::yaml::parse_yaml;

    #[test]
    fn test_host_network_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: host-net-deploy
spec:
  template:
    spec:
      hostNetwork: true
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = HostNetworkCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("host's network"));
    }

    #[test]
    fn test_no_host_network_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: safe-deploy
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = HostNetworkCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_host_pid_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: host-pid-deploy
spec:
  template:
    spec:
      hostPID: true
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = HostPIDCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("PID namespace"));
    }

    #[test]
    fn test_host_ipc_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: host-ipc-deploy
spec:
  template:
    spec:
      hostIPC: true
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = HostIPCCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("IPC namespace"));
    }
}
