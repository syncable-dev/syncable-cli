//! Miscellaneous check templates.

use crate::analyzer::kubelint::context::Object;
use crate::analyzer::kubelint::extract;
use crate::analyzer::kubelint::templates::{CheckFunc, ParameterDesc, Template, TemplateError};
use crate::analyzer::kubelint::types::{Diagnostic, ObjectKindsDesc};

/// Template for checking sysctls usage.
pub struct SysctlsTemplate;

impl Template for SysctlsTemplate {
    fn key(&self) -> &str {
        "sysctls"
    }

    fn human_name(&self) -> &str {
        "Sysctls"
    }

    fn description(&self) -> &str {
        "Checks for unsafe sysctl settings"
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
        Ok(Box::new(SysctlsCheck))
    }
}

struct SysctlsCheck;

impl CheckFunc for SysctlsCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Unsafe sysctls that require special permissions
        let unsafe_sysctls = [
            "kernel.shm",
            "kernel.msg",
            "kernel.sem",
            "fs.mqueue.",
            "net.",
        ];

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            if let Some(sc) = &pod_spec.security_context {
                for sysctl in &sc.sysctls {
                    let is_unsafe = unsafe_sysctls
                        .iter()
                        .any(|prefix| sysctl.name.starts_with(prefix));
                    if is_unsafe {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "Pod uses potentially unsafe sysctl '{}'",
                                sysctl.name
                            ),
                            remediation: Some(
                                "Ensure this sysctl is allowed by the cluster's PodSecurityPolicy \
                                 or PodSecurityStandard and is necessary for your workload."
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

/// Template for checking DNS config options.
pub struct DnsConfigOptionsTemplate;

impl Template for DnsConfigOptionsTemplate {
    fn key(&self) -> &str {
        "dnsconfig-options"
    }

    fn human_name(&self) -> &str {
        "DNS Config Options"
    }

    fn description(&self) -> &str {
        "Checks DNS configuration options"
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
        Ok(Box::new(DnsConfigOptionsCheck))
    }
}

struct DnsConfigOptionsCheck;

impl CheckFunc for DnsConfigOptionsCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            if let Some(dns_config) = &pod_spec.dns_config {
                // Check for ndots setting that could cause performance issues
                for option in &dns_config.options {
                    if let Some(name) = &option.name {
                        if name == "ndots" {
                            if let Some(value) = &option.value {
                                if let Ok(ndots) = value.parse::<i32>() {
                                    if ndots > 5 {
                                        diagnostics.push(Diagnostic {
                                            message: format!(
                                                "DNS ndots is set to {}, which may cause DNS lookup performance issues",
                                                ndots
                                            ),
                                            remediation: Some(
                                                "Consider lowering ndots to 2 or less for better DNS performance."
                                                    .to_string(),
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        diagnostics
    }
}

/// Template for checking startup probe port.
pub struct StartupPortTemplate;

impl Template for StartupPortTemplate {
    fn key(&self) -> &str {
        "startup-port"
    }

    fn human_name(&self) -> &str {
        "Startup Probe Port"
    }

    fn description(&self) -> &str {
        "Validates that startup probe port matches an exposed container port"
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
        Ok(Box::new(StartupPortCheck))
    }
}

struct StartupPortCheck;

impl CheckFunc for StartupPortCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(pod_spec) = extract::pod_spec::extract_pod_spec(&object.k8s_object) {
            for container in extract::container::containers(pod_spec) {
                if let Some(probe) = &container.startup_probe {
                    let probe_port = probe
                        .http_get
                        .as_ref()
                        .map(|h| h.port)
                        .or_else(|| probe.tcp_socket.as_ref().map(|t| t.port));

                    if let Some(port_num) = probe_port {
                        let has_matching_port =
                            container.ports.iter().any(|p| p.container_port == port_num);

                        if !has_matching_port && !container.ports.is_empty() {
                            diagnostics.push(Diagnostic {
                                message: format!(
                                    "Container '{}' startup probe uses port {} which is not exposed",
                                    container.name, port_num
                                ),
                                remediation: Some(
                                    "Ensure the startup probe port matches an exposed container port."
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

/// Template for checking env var valueFrom usage.
pub struct EnvVarValueFromTemplate;

impl Template for EnvVarValueFromTemplate {
    fn key(&self) -> &str {
        "env-var-value-from"
    }

    fn human_name(&self) -> &str {
        "Env Var Value From"
    }

    fn description(&self) -> &str {
        "Checks environment variable valueFrom configurations"
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
        Ok(Box::new(EnvVarValueFromCheck))
    }
}

struct EnvVarValueFromCheck;

impl CheckFunc for EnvVarValueFromCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let diagnostics = Vec::new();
        // This is a placeholder - the actual implementation would check
        // for specific valueFrom misconfigurations
        let _ = object;
        diagnostics
    }
}

/// Template for checking target port references.
pub struct TargetPortTemplate;

impl Template for TargetPortTemplate {
    fn key(&self) -> &str {
        "target-port"
    }

    fn human_name(&self) -> &str {
        "Target Port"
    }

    fn description(&self) -> &str {
        "Checks Service targetPort references"
    }

    fn supported_object_kinds(&self) -> ObjectKindsDesc {
        ObjectKindsDesc::new(&["Service"])
    }

    fn parameters(&self) -> Vec<ParameterDesc> {
        Vec::new()
    }

    fn instantiate(
        &self,
        _params: &serde_yaml::Value,
    ) -> Result<Box<dyn CheckFunc>, TemplateError> {
        Ok(Box::new(TargetPortCheck))
    }
}

struct TargetPortCheck;

impl CheckFunc for TargetPortCheck {
    fn check(&self, object: &Object) -> Vec<Diagnostic> {
        let diagnostics = Vec::new();
        // This check would need cross-resource validation to verify
        // that targetPort references valid container ports
        let _ = object;
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::kubelint::parser::yaml::parse_yaml;

    #[test]
    fn test_sysctls_unsafe() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sysctl-deploy
spec:
  template:
    spec:
      securityContext:
        sysctls:
        - name: net.core.somaxconn
          value: "1024"
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = SysctlsCheck;
        let diagnostics = check.check(&objects[0]);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("net.core.somaxconn"));
    }

    #[test]
    fn test_no_sysctls_ok() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: no-sysctl-deploy
spec:
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:1.21.0
"#;
        let objects = parse_yaml(yaml).unwrap();
        let check = SysctlsCheck;
        let diagnostics = check.check(&objects[0]);
        assert!(diagnostics.is_empty());
    }
}
