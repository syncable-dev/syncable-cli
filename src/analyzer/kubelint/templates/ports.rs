//! Port-related check templates.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for detecting privileged ports (< 1024).
pub struct PrivilegedPortsTemplate;

impl Template for PrivilegedPortsTemplate {
    fn key(&self) -> &str {
        "privileged-ports"
    }

    fn human_name(&self) -> &str {
        "Privileged Ports"
    }

    fn description(&self) -> &str {
        "Detects containers using privileged ports (< 1024)"
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
        Ok(Box::new(PrivilegedPortsCheck))
    }
}

struct PrivilegedPortsCheck;

impl CheckFunc for PrivilegedPortsCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                for port in &container.ports {
                    if port.container_port < 1024 {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "Container '{}' uses privileged port {}",
                                container.name, port.container_port
                            ),
                            remediation: Some(
                                "Use ports >= 1024 to avoid requiring NET_BIND_SERVICE \
                                 capability. Map privileged ports via Service if needed."
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

/// Template for detecting SSH port usage.
pub struct SSHPortTemplate;

impl Template for SSHPortTemplate {
    fn key(&self) -> &str {
        "ssh-port"
    }

    fn human_name(&self) -> &str {
        "SSH Port"
    }

    fn description(&self) -> &str {
        "Detects containers exposing SSH port (22)"
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
        Ok(Box::new(SSHPortCheck))
    }
}

struct SSHPortCheck;

impl CheckFunc for SSHPortCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::all_containers(pod_spec) {
                for port in &container.ports {
                    if port.container_port == 22 {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "Container '{}' exposes SSH port 22",
                                container.name
                            ),
                            remediation: Some(
                                "SSH access in containers is generally discouraged. \
                                 Use kubectl exec for debugging or remove SSH."
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

/// Template for validating liveness probe port matches container port.
pub struct LivenessPortTemplate;

impl Template for LivenessPortTemplate {
    fn key(&self) -> &str {
        "liveness-port"
    }

    fn human_name(&self) -> &str {
        "Liveness Probe Port"
    }

    fn description(&self) -> &str {
        "Validates that liveness probe port matches an exposed container port"
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
        Ok(Box::new(LivenessPortCheck))
    }
}

struct LivenessPortCheck;

impl CheckFunc for LivenessPortCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::containers(pod_spec) {
                if let Some(probe) = &container.liveness_probe {
                    let probe_port = probe.http_get.as_ref().map(|h| h.port)
                        .or_else(|| probe.tcp_socket.as_ref().map(|t| t.port));

                    if let Some(port_num) = probe_port {
                        let has_matching_port = container.ports.iter()
                            .any(|p| p.container_port == port_num);

                        if !has_matching_port && !container.ports.is_empty() {
                            diagnostics.push(Diagnostic {
                                message: format!(
                                    "Container '{}' liveness probe uses port {} which is not exposed",
                                    container.name, port_num
                                ),
                                remediation: Some(
                                    "Ensure the liveness probe port matches an exposed container port."
                                        .to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }

        diagnostics
    }
}

/// Template for validating readiness probe port matches container port.
pub struct ReadinessPortTemplate;

impl Template for ReadinessPortTemplate {
    fn key(&self) -> &str {
        "readiness-port"
    }

    fn human_name(&self) -> &str {
        "Readiness Probe Port"
    }

    fn description(&self) -> &str {
        "Validates that readiness probe port matches an exposed container port"
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
        Ok(Box::new(ReadinessPortCheck))
    }
}

struct ReadinessPortCheck;

impl CheckFunc for ReadinessPortCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::containers(pod_spec) {
                if let Some(probe) = &container.readiness_probe {
                    let probe_port = probe.http_get.as_ref().map(|h| h.port)
                        .or_else(|| probe.tcp_socket.as_ref().map(|t| t.port));

                    if let Some(port_num) = probe_port {
                        let has_matching_port = container.ports.iter()
                            .any(|p| p.container_port == port_num);

                        if !has_matching_port && !container.ports.is_empty() {
                            diagnostics.push(Diagnostic {
                                message: format!(
                                    "Container '{}' readiness probe uses port {} which is not exposed",
                                    container.name, port_num
                                ),
                                remediation: Some(
                                    "Ensure the readiness probe port matches an exposed container port."
                                        .to_string(),
                                ),
                            });
                        }
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
    fn test_privileged_port_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: priv-port
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
        ports:
        - containerPort: 80
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = PrivilegedPortsCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("80"));
    }

    #[test]
    fn test_non_privileged_port_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: non-priv-port
spec:
  template:
    spec:
      containers:
      - name: app
        image: myapp:1.0
        ports:
        - containerPort: 8080
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = PrivilegedPortsCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_ssh_port_detected() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ssh-container
spec:
  template:
    spec:
      containers:
      - name: ssh
        image: ssh:latest
        ports:
        - containerPort: 22
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = SSHPortCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("SSH"));
    }
}
