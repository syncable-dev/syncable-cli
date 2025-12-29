//! YAML parsing for Kubernetes manifests.

use crate::analyzer::kubelint::context::object::*;
use crate::analyzer::kubelint::context::Object;
use std::collections::BTreeMap;
use std::path::Path;

/// Parse a YAML string containing one or more Kubernetes objects.
pub fn parse_yaml(content: &str) -> Result<Vec<Object>, YamlParseError> {
    parse_yaml_with_path(content, Path::new("<stdin>"))
}

/// Parse YAML content with a source file path.
pub fn parse_yaml_with_path(content: &str, path: &Path) -> Result<Vec<Object>, YamlParseError> {
    let mut objects = Vec::new();
    let mut line_number = 1u32;

    // Split on document separator and track line numbers
    for doc in content.split("\n---") {
        let doc = doc.trim();
        if doc.is_empty() || doc.starts_with('#') {
            // Count lines for empty or comment-only documents
            line_number += doc.lines().count() as u32 + 1;
            continue;
        }

        // Parse the YAML document
        match serde_yaml::from_str::<serde_yaml::Value>(doc) {
            Ok(value) => {
                if let Some(obj) = parse_k8s_object(&value, path, line_number) {
                    objects.push(obj);
                }
            }
            Err(e) => {
                return Err(YamlParseError::SyntaxError(format!(
                    "at line {}: {}",
                    line_number, e
                )));
            }
        }

        // Update line number for next document
        line_number += doc.lines().count() as u32 + 1;
    }

    Ok(objects)
}

/// Parse a YAML file.
pub fn parse_yaml_file(path: &Path) -> Result<Vec<Object>, YamlParseError> {
    let content =
        std::fs::read_to_string(path).map_err(|e| YamlParseError::IoError(e.to_string()))?;

    parse_yaml_with_path(&content, path)
}

/// Parse all YAML files in a directory (recursively).
pub fn parse_yaml_dir(path: &Path) -> Result<Vec<Object>, YamlParseError> {
    let mut objects = Vec::new();

    for entry in walkdir::WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let entry_path = entry.path();
        if entry_path.is_file() {
            let ext = entry_path.extension().and_then(|e| e.to_str());
            if matches!(ext, Some("yaml") | Some("yml")) {
                match parse_yaml_file(entry_path) {
                    Ok(mut objs) => objects.append(&mut objs),
                    Err(e) => {
                        // Log warning but continue parsing other files
                        eprintln!(
                            "Warning: failed to parse {}: {}",
                            entry_path.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    Ok(objects)
}

/// Parse a single K8s object from a YAML value.
fn parse_k8s_object(value: &serde_yaml::Value, path: &Path, line: u32) -> Option<Object> {
    let api_version = value.get("apiVersion")?.as_str()?;
    let kind = value.get("kind")?.as_str()?;

    let metadata = ObjectMetadata::from_file(path).with_line(line);
    let k8s_obj = match kind {
        "Deployment" => K8sObject::Deployment(Box::new(parse_deployment(value))),
        "StatefulSet" => K8sObject::StatefulSet(Box::new(parse_statefulset(value))),
        "DaemonSet" => K8sObject::DaemonSet(Box::new(parse_daemonset(value))),
        "ReplicaSet" => K8sObject::ReplicaSet(Box::new(parse_replicaset(value))),
        "Pod" => K8sObject::Pod(Box::new(parse_pod(value))),
        "Job" => K8sObject::Job(Box::new(parse_job(value))),
        "CronJob" => K8sObject::CronJob(Box::new(parse_cronjob(value))),
        "Service" => K8sObject::Service(Box::new(parse_service(value))),
        "Ingress" => K8sObject::Ingress(Box::new(parse_ingress(value))),
        "NetworkPolicy" => K8sObject::NetworkPolicy(Box::new(parse_network_policy(value))),
        "Role" => K8sObject::Role(Box::new(parse_role(value))),
        "ClusterRole" => K8sObject::ClusterRole(Box::new(parse_cluster_role(value))),
        "RoleBinding" => K8sObject::RoleBinding(Box::new(parse_role_binding(value))),
        "ClusterRoleBinding" => {
            K8sObject::ClusterRoleBinding(Box::new(parse_cluster_role_binding(value)))
        }
        "ServiceAccount" => K8sObject::ServiceAccount(Box::new(parse_service_account(value))),
        "HorizontalPodAutoscaler" => K8sObject::HorizontalPodAutoscaler(Box::new(parse_hpa(value))),
        "PodDisruptionBudget" => K8sObject::PodDisruptionBudget(Box::new(parse_pdb(value))),
        "PersistentVolumeClaim" => K8sObject::PersistentVolumeClaim(Box::new(parse_pvc(value))),
        _ => K8sObject::Unknown(Box::new(parse_unknown(value, api_version, kind))),
    };

    Some(Object::new(metadata, k8s_obj))
}

// ============================================================================
// Parse helper functions
// ============================================================================

fn get_string(value: &serde_yaml::Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(|s| s.to_string())
}

fn get_i32(value: &serde_yaml::Value, key: &str) -> Option<i32> {
    value.get(key)?.as_i64().map(|n| n as i32)
}

fn get_i64(value: &serde_yaml::Value, key: &str) -> Option<i64> {
    value.get(key)?.as_i64()
}

fn get_bool(value: &serde_yaml::Value, key: &str) -> Option<bool> {
    value.get(key)?.as_bool()
}

fn get_string_map(value: &serde_yaml::Value, key: &str) -> Option<BTreeMap<String, String>> {
    let mapping = value.get(key)?.as_mapping()?;
    let mut map = BTreeMap::new();
    for (k, v) in mapping {
        if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
            map.insert(key.to_string(), val.to_string());
        }
    }
    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

fn parse_metadata(value: &serde_yaml::Value) -> (String, Option<String>, Option<BTreeMap<String, String>>, Option<BTreeMap<String, String>>) {
    let metadata = value.get("metadata");
    let name = metadata
        .and_then(|m| get_string(m, "name"))
        .unwrap_or_default();
    let namespace = metadata.and_then(|m| get_string(m, "namespace"));
    let labels = metadata.and_then(|m| get_string_map(m, "labels"));
    let annotations = metadata.and_then(|m| get_string_map(m, "annotations"));
    (name, namespace, labels, annotations)
}

fn parse_label_selector(value: &serde_yaml::Value) -> Option<LabelSelector> {
    let selector = value.get("selector")?;
    Some(LabelSelector {
        match_labels: get_string_map(selector, "matchLabels"),
    })
}

fn parse_pod_spec(value: &serde_yaml::Value) -> Option<PodSpec> {
    let spec = value.get("spec")?.get("template")?.get("spec")?;
    Some(parse_pod_spec_inner(spec))
}

fn parse_pod_spec_direct(value: &serde_yaml::Value) -> Option<PodSpec> {
    let spec = value.get("spec")?;
    Some(parse_pod_spec_inner(spec))
}

fn parse_pod_spec_inner(spec: &serde_yaml::Value) -> PodSpec {
    PodSpec {
        containers: parse_containers(spec.get("containers")),
        init_containers: parse_containers(spec.get("initContainers")),
        volumes: parse_volumes(spec.get("volumes")),
        service_account_name: get_string(spec, "serviceAccountName")
            .or_else(|| get_string(spec, "serviceAccount")),
        host_network: get_bool(spec, "hostNetwork"),
        host_pid: get_bool(spec, "hostPID"),
        host_ipc: get_bool(spec, "hostIPC"),
        security_context: parse_pod_security_context(spec.get("securityContext")),
        affinity: parse_affinity(spec.get("affinity")),
        dns_config: parse_dns_config(spec.get("dnsConfig")),
        restart_policy: get_string(spec, "restartPolicy"),
        priority_class_name: get_string(spec, "priorityClassName"),
    }
}

fn parse_containers(containers: Option<&serde_yaml::Value>) -> Vec<ContainerSpec> {
    let Some(containers) = containers else {
        return Vec::new();
    };
    let Some(arr) = containers.as_sequence() else {
        return Vec::new();
    };

    arr.iter().map(parse_container).collect()
}

fn parse_container(c: &serde_yaml::Value) -> ContainerSpec {
    ContainerSpec {
        name: get_string(c, "name").unwrap_or_default(),
        image: get_string(c, "image"),
        security_context: parse_security_context(c.get("securityContext")),
        resources: parse_resources(c.get("resources")),
        liveness_probe: parse_probe(c.get("livenessProbe")),
        readiness_probe: parse_probe(c.get("readinessProbe")),
        startup_probe: parse_probe(c.get("startupProbe")),
        env: parse_env_vars(c.get("env")),
        volume_mounts: parse_volume_mounts(c.get("volumeMounts")),
        ports: parse_container_ports(c.get("ports")),
    }
}

fn parse_security_context(sc: Option<&serde_yaml::Value>) -> Option<SecurityContext> {
    let sc = sc?;
    Some(SecurityContext {
        privileged: get_bool(sc, "privileged"),
        allow_privilege_escalation: get_bool(sc, "allowPrivilegeEscalation"),
        run_as_non_root: get_bool(sc, "runAsNonRoot"),
        run_as_user: get_i64(sc, "runAsUser"),
        read_only_root_filesystem: get_bool(sc, "readOnlyRootFilesystem"),
        capabilities: parse_capabilities(sc.get("capabilities")),
        proc_mount: get_string(sc, "procMount"),
    })
}

fn parse_capabilities(caps: Option<&serde_yaml::Value>) -> Option<Capabilities> {
    let caps = caps?;
    Some(Capabilities {
        add: parse_string_array(caps.get("add")),
        drop: parse_string_array(caps.get("drop")),
    })
}

fn parse_string_array(value: Option<&serde_yaml::Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_sequence())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_resources(res: Option<&serde_yaml::Value>) -> Option<ResourceRequirements> {
    let res = res?;
    Some(ResourceRequirements {
        limits: get_string_map(res, "limits"),
        requests: get_string_map(res, "requests"),
    })
}

fn parse_probe(probe: Option<&serde_yaml::Value>) -> Option<Probe> {
    let probe = probe?;
    Some(Probe {
        http_get: probe.get("httpGet").map(|h| HttpGetAction {
            port: h.get("port").and_then(|p| p.as_i64()).unwrap_or(0) as i32,
            path: get_string(h, "path"),
        }),
        tcp_socket: probe.get("tcpSocket").map(|t| TcpSocketAction {
            port: t.get("port").and_then(|p| p.as_i64()).unwrap_or(0) as i32,
        }),
        exec: probe.get("exec").map(|e| ExecAction {
            command: parse_string_array(e.get("command")),
        }),
    })
}

fn parse_env_vars(env: Option<&serde_yaml::Value>) -> Vec<EnvVar> {
    let Some(env) = env else {
        return Vec::new();
    };
    let Some(arr) = env.as_sequence() else {
        return Vec::new();
    };

    arr.iter()
        .map(|e| EnvVar {
            name: get_string(e, "name").unwrap_or_default(),
            value: get_string(e, "value"),
            value_from: parse_env_var_source(e.get("valueFrom")),
        })
        .collect()
}

fn parse_env_var_source(vf: Option<&serde_yaml::Value>) -> Option<EnvVarSource> {
    let vf = vf?;
    if let Some(secret) = vf.get("secretKeyRef") {
        return Some(EnvVarSource::SecretKeyRef {
            name: get_string(secret, "name").unwrap_or_default(),
            key: get_string(secret, "key").unwrap_or_default(),
        });
    }
    if let Some(cm) = vf.get("configMapKeyRef") {
        return Some(EnvVarSource::ConfigMapKeyRef {
            name: get_string(cm, "name").unwrap_or_default(),
            key: get_string(cm, "key").unwrap_or_default(),
        });
    }
    if let Some(field) = vf.get("fieldRef") {
        return Some(EnvVarSource::FieldRef {
            field_path: get_string(field, "fieldPath").unwrap_or_default(),
        });
    }
    None
}

fn parse_volume_mounts(mounts: Option<&serde_yaml::Value>) -> Vec<VolumeMount> {
    let Some(mounts) = mounts else {
        return Vec::new();
    };
    let Some(arr) = mounts.as_sequence() else {
        return Vec::new();
    };

    arr.iter()
        .map(|m| VolumeMount {
            name: get_string(m, "name").unwrap_or_default(),
            mount_path: get_string(m, "mountPath").unwrap_or_default(),
            read_only: get_bool(m, "readOnly"),
        })
        .collect()
}

fn parse_container_ports(ports: Option<&serde_yaml::Value>) -> Vec<ContainerPort> {
    let Some(ports) = ports else {
        return Vec::new();
    };
    let Some(arr) = ports.as_sequence() else {
        return Vec::new();
    };

    arr.iter()
        .map(|p| ContainerPort {
            container_port: get_i32(p, "containerPort").unwrap_or(0),
            protocol: get_string(p, "protocol"),
            host_port: get_i32(p, "hostPort"),
        })
        .collect()
}

fn parse_volumes(volumes: Option<&serde_yaml::Value>) -> Vec<Volume> {
    let Some(volumes) = volumes else {
        return Vec::new();
    };
    let Some(arr) = volumes.as_sequence() else {
        return Vec::new();
    };

    arr.iter()
        .map(|v| Volume {
            name: get_string(v, "name").unwrap_or_default(),
            host_path: v.get("hostPath").map(|h| HostPathVolumeSource {
                path: get_string(h, "path").unwrap_or_default(),
                type_: get_string(h, "type"),
            }),
            secret: v.get("secret").map(|s| SecretVolumeSource {
                secret_name: get_string(s, "secretName"),
            }),
        })
        .collect()
}

fn parse_pod_security_context(psc: Option<&serde_yaml::Value>) -> Option<PodSecurityContext> {
    let psc = psc?;
    Some(PodSecurityContext {
        run_as_non_root: get_bool(psc, "runAsNonRoot"),
        run_as_user: get_i64(psc, "runAsUser"),
        sysctls: parse_sysctls(psc.get("sysctls")),
    })
}

fn parse_sysctls(sysctls: Option<&serde_yaml::Value>) -> Vec<Sysctl> {
    let Some(sysctls) = sysctls else {
        return Vec::new();
    };
    let Some(arr) = sysctls.as_sequence() else {
        return Vec::new();
    };

    arr.iter()
        .map(|s| Sysctl {
            name: get_string(s, "name").unwrap_or_default(),
            value: get_string(s, "value").unwrap_or_default(),
        })
        .collect()
}

fn parse_affinity(affinity: Option<&serde_yaml::Value>) -> Option<Affinity> {
    let affinity = affinity?;
    Some(Affinity {
        pod_anti_affinity: parse_pod_anti_affinity(affinity.get("podAntiAffinity")),
        node_affinity: parse_node_affinity(affinity.get("nodeAffinity")),
    })
}

fn parse_pod_anti_affinity(paa: Option<&serde_yaml::Value>) -> Option<PodAntiAffinity> {
    let paa = paa?;
    Some(PodAntiAffinity {
        required_during_scheduling_ignored_during_execution: parse_pod_affinity_terms(
            paa.get("requiredDuringSchedulingIgnoredDuringExecution"),
        ),
        preferred_during_scheduling_ignored_during_execution: parse_weighted_pod_affinity_terms(
            paa.get("preferredDuringSchedulingIgnoredDuringExecution"),
        ),
    })
}

fn parse_pod_affinity_terms(terms: Option<&serde_yaml::Value>) -> Vec<PodAffinityTerm> {
    let Some(terms) = terms else {
        return Vec::new();
    };
    let Some(arr) = terms.as_sequence() else {
        return Vec::new();
    };

    arr.iter()
        .map(|t| PodAffinityTerm {
            topology_key: get_string(t, "topologyKey").unwrap_or_default(),
        })
        .collect()
}

fn parse_weighted_pod_affinity_terms(
    terms: Option<&serde_yaml::Value>,
) -> Vec<WeightedPodAffinityTerm> {
    let Some(terms) = terms else {
        return Vec::new();
    };
    let Some(arr) = terms.as_sequence() else {
        return Vec::new();
    };

    arr.iter()
        .map(|t| WeightedPodAffinityTerm {
            weight: get_i32(t, "weight").unwrap_or(0),
            pod_affinity_term: t
                .get("podAffinityTerm")
                .map(|pat| PodAffinityTerm {
                    topology_key: get_string(pat, "topologyKey").unwrap_or_default(),
                })
                .unwrap_or_default(),
        })
        .collect()
}

fn parse_node_affinity(na: Option<&serde_yaml::Value>) -> Option<NodeAffinity> {
    let na = na?;
    Some(NodeAffinity {
        required_during_scheduling_ignored_during_execution: na
            .get("requiredDuringSchedulingIgnoredDuringExecution")
            .map(|r| NodeSelector {
                node_selector_terms: r
                    .get("nodeSelectorTerms")
                    .and_then(|t| t.as_sequence())
                    .map(|arr| {
                        arr.iter()
                            .map(|term| NodeSelectorTerm {
                                match_expressions: term
                                    .get("matchExpressions")
                                    .and_then(|e| e.as_sequence())
                                    .map(|arr| {
                                        arr.iter()
                                            .map(|expr| NodeSelectorRequirement {
                                                key: get_string(expr, "key").unwrap_or_default(),
                                                operator: get_string(expr, "operator")
                                                    .unwrap_or_default(),
                                                values: parse_string_array(expr.get("values")),
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            }),
    })
}

fn parse_dns_config(dns: Option<&serde_yaml::Value>) -> Option<DnsConfig> {
    let dns = dns?;
    Some(DnsConfig {
        options: dns
            .get("options")
            .and_then(|o| o.as_sequence())
            .map(|arr| {
                arr.iter()
                    .map(|opt| PodDnsConfigOption {
                        name: get_string(opt, "name"),
                        value: get_string(opt, "value"),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    })
}

// ============================================================================
// Object type parsers
// ============================================================================

fn parse_deployment(value: &serde_yaml::Value) -> DeploymentData {
    let (name, namespace, labels, annotations) = parse_metadata(value);
    let spec = value.get("spec");

    DeploymentData {
        name,
        namespace,
        labels,
        annotations,
        replicas: spec.and_then(|s| get_i32(s, "replicas")),
        selector: parse_label_selector(value.get("spec").unwrap_or(value)),
        pod_spec: parse_pod_spec(value),
        strategy: spec.and_then(|s| s.get("strategy")).map(|strat| DeploymentStrategy {
            type_: get_string(strat, "type"),
            rolling_update: strat.get("rollingUpdate").map(|ru| RollingUpdateDeployment {
                max_unavailable: get_string(ru, "maxUnavailable")
                    .or_else(|| get_i32(ru, "maxUnavailable").map(|n| n.to_string())),
                max_surge: get_string(ru, "maxSurge")
                    .or_else(|| get_i32(ru, "maxSurge").map(|n| n.to_string())),
            }),
        }),
    }
}

fn parse_statefulset(value: &serde_yaml::Value) -> StatefulSetData {
    let (name, namespace, labels, annotations) = parse_metadata(value);
    let spec = value.get("spec");

    StatefulSetData {
        name,
        namespace,
        labels,
        annotations,
        replicas: spec.and_then(|s| get_i32(s, "replicas")),
        selector: parse_label_selector(value.get("spec").unwrap_or(value)),
        pod_spec: parse_pod_spec(value),
    }
}

fn parse_daemonset(value: &serde_yaml::Value) -> DaemonSetData {
    let (name, namespace, labels, annotations) = parse_metadata(value);
    let spec = value.get("spec");

    DaemonSetData {
        name,
        namespace,
        labels,
        annotations,
        selector: parse_label_selector(value.get("spec").unwrap_or(value)),
        pod_spec: parse_pod_spec(value),
        update_strategy: spec.and_then(|s| s.get("updateStrategy")).map(|us| DaemonSetUpdateStrategy {
            type_: get_string(us, "type"),
        }),
    }
}

fn parse_replicaset(value: &serde_yaml::Value) -> ReplicaSetData {
    let (name, namespace, labels, annotations) = parse_metadata(value);
    let spec = value.get("spec");

    ReplicaSetData {
        name,
        namespace,
        labels,
        annotations,
        replicas: spec.and_then(|s| get_i32(s, "replicas")),
        selector: parse_label_selector(value.get("spec").unwrap_or(value)),
        pod_spec: parse_pod_spec(value),
    }
}

fn parse_pod(value: &serde_yaml::Value) -> PodData {
    let (name, namespace, labels, annotations) = parse_metadata(value);

    PodData {
        name,
        namespace,
        labels,
        annotations,
        spec: parse_pod_spec_direct(value),
    }
}

fn parse_job(value: &serde_yaml::Value) -> JobData {
    let (name, namespace, labels, annotations) = parse_metadata(value);
    let spec = value.get("spec");

    JobData {
        name,
        namespace,
        labels,
        annotations,
        pod_spec: parse_pod_spec(value),
        ttl_seconds_after_finished: spec.and_then(|s| get_i32(s, "ttlSecondsAfterFinished")),
    }
}

fn parse_cronjob(value: &serde_yaml::Value) -> CronJobData {
    let (name, namespace, labels, annotations) = parse_metadata(value);

    // CronJob has jobTemplate.spec.template.spec
    let job_template = value
        .get("spec")
        .and_then(|s| s.get("jobTemplate"));

    let job_spec = job_template.map(|jt| {
        let (_, _, job_labels, job_annotations) = jt
            .get("metadata")
            .map(|m| {
                (
                    get_string(m, "name").unwrap_or_default(),
                    get_string(m, "namespace"),
                    get_string_map(m, "labels"),
                    get_string_map(m, "annotations"),
                )
            })
            .unwrap_or_default();

        let job_spec = jt.get("spec");
        JobData {
            name: name.clone(),
            namespace: namespace.clone(),
            labels: job_labels,
            annotations: job_annotations,
            pod_spec: job_spec.and_then(|js| {
                js.get("template")
                    .and_then(|t| t.get("spec"))
                    .map(parse_pod_spec_inner)
            }),
            ttl_seconds_after_finished: job_spec.and_then(|s| get_i32(s, "ttlSecondsAfterFinished")),
        }
    });

    CronJobData {
        name,
        namespace,
        labels,
        annotations,
        job_spec,
    }
}

fn parse_service(value: &serde_yaml::Value) -> ServiceData {
    let (name, namespace, labels, annotations) = parse_metadata(value);
    let spec = value.get("spec");

    ServiceData {
        name,
        namespace,
        labels,
        annotations,
        selector: spec.and_then(|s| get_string_map(s, "selector")),
        ports: spec
            .and_then(|s| s.get("ports"))
            .and_then(|p| p.as_sequence())
            .map(|arr| {
                arr.iter()
                    .map(|p| ServicePort {
                        port: get_i32(p, "port").unwrap_or(0),
                        target_port: get_string(p, "targetPort")
                            .or_else(|| get_i32(p, "targetPort").map(|n| n.to_string())),
                        protocol: get_string(p, "protocol"),
                        name: get_string(p, "name"),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        type_: spec.and_then(|s| get_string(s, "type")),
    }
}

fn parse_ingress(value: &serde_yaml::Value) -> IngressData {
    let (name, namespace, labels, annotations) = parse_metadata(value);
    let spec = value.get("spec");

    IngressData {
        name,
        namespace,
        labels,
        annotations,
        rules: spec
            .and_then(|s| s.get("rules"))
            .and_then(|r| r.as_sequence())
            .map(|arr| {
                arr.iter()
                    .map(|rule| IngressRule {
                        host: get_string(rule, "host"),
                        http: rule.get("http").map(|http| HttpIngressRuleValue {
                            paths: http
                                .get("paths")
                                .and_then(|p| p.as_sequence())
                                .map(|arr| {
                                    arr.iter()
                                        .map(|path| HttpIngressPath {
                                            path: get_string(path, "path"),
                                            backend: path
                                                .get("backend")
                                                .map(|b| IngressBackend {
                                                    service: b.get("service").map(|svc| {
                                                        IngressServiceBackend {
                                                            name: get_string(svc, "name")
                                                                .unwrap_or_default(),
                                                            port: svc.get("port").map(|p| {
                                                                ServiceBackendPort {
                                                                    number: get_i32(p, "number"),
                                                                    name: get_string(p, "name"),
                                                                }
                                                            }),
                                                        }
                                                    }),
                                                })
                                                .unwrap_or_default(),
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                        }),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn parse_network_policy(value: &serde_yaml::Value) -> NetworkPolicyData {
    let (name, namespace, labels, annotations) = parse_metadata(value);
    let spec = value.get("spec");

    NetworkPolicyData {
        name,
        namespace,
        labels,
        annotations,
        pod_selector: spec.and_then(|s| s.get("podSelector")).map(|ps| LabelSelector {
            match_labels: get_string_map(ps, "matchLabels"),
        }),
    }
}

fn parse_role(value: &serde_yaml::Value) -> RoleData {
    let (name, namespace, labels, annotations) = parse_metadata(value);

    RoleData {
        name,
        namespace,
        labels,
        annotations,
        rules: parse_policy_rules(value.get("rules")),
    }
}

fn parse_cluster_role(value: &serde_yaml::Value) -> ClusterRoleData {
    let (name, _, labels, annotations) = parse_metadata(value);

    ClusterRoleData {
        name,
        labels,
        annotations,
        rules: parse_policy_rules(value.get("rules")),
    }
}

fn parse_policy_rules(rules: Option<&serde_yaml::Value>) -> Vec<PolicyRule> {
    let Some(rules) = rules else {
        return Vec::new();
    };
    let Some(arr) = rules.as_sequence() else {
        return Vec::new();
    };

    arr.iter()
        .map(|r| PolicyRule {
            api_groups: parse_string_array(r.get("apiGroups")),
            resources: parse_string_array(r.get("resources")),
            verbs: parse_string_array(r.get("verbs")),
        })
        .collect()
}

fn parse_role_binding(value: &serde_yaml::Value) -> RoleBindingData {
    let (name, namespace, labels, annotations) = parse_metadata(value);

    RoleBindingData {
        name,
        namespace,
        labels,
        annotations,
        role_ref: parse_role_ref(value.get("roleRef")),
        subjects: parse_subjects(value.get("subjects")),
    }
}

fn parse_cluster_role_binding(value: &serde_yaml::Value) -> ClusterRoleBindingData {
    let (name, _, labels, annotations) = parse_metadata(value);

    ClusterRoleBindingData {
        name,
        labels,
        annotations,
        role_ref: parse_role_ref(value.get("roleRef")),
        subjects: parse_subjects(value.get("subjects")),
    }
}

fn parse_role_ref(role_ref: Option<&serde_yaml::Value>) -> RoleRef {
    let Some(rr) = role_ref else {
        return RoleRef::default();
    };
    RoleRef {
        api_group: get_string(rr, "apiGroup").unwrap_or_default(),
        kind: get_string(rr, "kind").unwrap_or_default(),
        name: get_string(rr, "name").unwrap_or_default(),
    }
}

fn parse_subjects(subjects: Option<&serde_yaml::Value>) -> Vec<Subject> {
    let Some(subjects) = subjects else {
        return Vec::new();
    };
    let Some(arr) = subjects.as_sequence() else {
        return Vec::new();
    };

    arr.iter()
        .map(|s| Subject {
            kind: get_string(s, "kind").unwrap_or_default(),
            name: get_string(s, "name").unwrap_or_default(),
            namespace: get_string(s, "namespace"),
        })
        .collect()
}

fn parse_service_account(value: &serde_yaml::Value) -> ServiceAccountData {
    let (name, namespace, labels, annotations) = parse_metadata(value);

    ServiceAccountData {
        name,
        namespace,
        labels,
        annotations,
    }
}

fn parse_hpa(value: &serde_yaml::Value) -> HpaData {
    let (name, namespace, labels, annotations) = parse_metadata(value);
    let spec = value.get("spec");

    HpaData {
        name,
        namespace,
        labels,
        annotations,
        min_replicas: spec.and_then(|s| get_i32(s, "minReplicas")),
        max_replicas: spec.and_then(|s| get_i32(s, "maxReplicas")).unwrap_or(0),
        scale_target_ref: spec
            .and_then(|s| s.get("scaleTargetRef"))
            .map(|str| CrossVersionObjectReference {
                api_version: get_string(str, "apiVersion"),
                kind: get_string(str, "kind").unwrap_or_default(),
                name: get_string(str, "name").unwrap_or_default(),
            })
            .unwrap_or_default(),
    }
}

fn parse_pdb(value: &serde_yaml::Value) -> PdbData {
    let (name, namespace, labels, annotations) = parse_metadata(value);
    let spec = value.get("spec");

    PdbData {
        name,
        namespace,
        labels,
        annotations,
        min_available: spec.and_then(|s| {
            get_string(s, "minAvailable").or_else(|| get_i32(s, "minAvailable").map(|n| n.to_string()))
        }),
        max_unavailable: spec.and_then(|s| {
            get_string(s, "maxUnavailable").or_else(|| get_i32(s, "maxUnavailable").map(|n| n.to_string()))
        }),
        selector: spec.and_then(|s| s.get("selector")).map(|sel| LabelSelector {
            match_labels: get_string_map(sel, "matchLabels"),
        }),
        unhealthy_pod_eviction_policy: spec.and_then(|s| get_string(s, "unhealthyPodEvictionPolicy")),
    }
}

fn parse_pvc(value: &serde_yaml::Value) -> PvcData {
    let (name, namespace, labels, annotations) = parse_metadata(value);

    PvcData {
        name,
        namespace,
        labels,
        annotations,
    }
}

fn parse_unknown(value: &serde_yaml::Value, api_version: &str, kind: &str) -> UnknownObject {
    let (name, namespace, labels, annotations) = parse_metadata(value);

    UnknownObject {
        api_version: api_version.to_string(),
        kind: kind.to_string(),
        name,
        namespace,
        labels,
        annotations,
        raw: value.clone(),
    }
}

/// YAML parsing errors.
#[derive(Debug, Clone)]
pub enum YamlParseError {
    /// I/O error reading file.
    IoError(String),
    /// YAML syntax error.
    SyntaxError(String),
    /// Invalid Kubernetes object.
    InvalidObject(String),
}

impl std::fmt::Display for YamlParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
            Self::SyntaxError(msg) => write!(f, "YAML syntax error: {}", msg),
            Self::InvalidObject(msg) => write!(f, "Invalid K8s object: {}", msg),
        }
    }
}

impl std::error::Error for YamlParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_deployment() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nginx-deployment
  namespace: default
  labels:
    app: nginx
spec:
  replicas: 3
  selector:
    matchLabels:
      app: nginx
  template:
    metadata:
      labels:
        app: nginx
    spec:
      containers:
      - name: nginx
        image: nginx:1.14.2
        ports:
        - containerPort: 80
"#;
        let objects = parse_yaml(yaml).unwrap();
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].name(), "nginx-deployment");
        assert_eq!(objects[0].namespace(), Some("default"));

        if let K8sObject::Deployment(dep) = &objects[0].k8s_object {
            assert_eq!(dep.replicas, Some(3));
            assert!(dep.pod_spec.is_some());
            let pod_spec = dep.pod_spec.as_ref().unwrap();
            assert_eq!(pod_spec.containers.len(), 1);
            assert_eq!(pod_spec.containers[0].name, "nginx");
            assert_eq!(pod_spec.containers[0].image, Some("nginx:1.14.2".to_string()));
        } else {
            panic!("Expected Deployment");
        }
    }

    #[test]
    fn test_parse_multi_document() {
        let yaml = r#"
apiVersion: v1
kind: Service
metadata:
  name: my-service
spec:
  selector:
    app: nginx
  ports:
  - port: 80
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-deployment
spec:
  replicas: 1
  selector:
    matchLabels:
      app: nginx
  template:
    spec:
      containers:
      - name: nginx
        image: nginx:latest
"#;
        let objects = parse_yaml(yaml).unwrap();
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].name(), "my-service");
        assert_eq!(objects[1].name(), "my-deployment");
    }

    #[test]
    fn test_parse_security_context() {
        let yaml = r#"
apiVersion: v1
kind: Pod
metadata:
  name: security-pod
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 1000
  containers:
  - name: app
    image: myapp:1.0
    securityContext:
      privileged: false
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities:
        drop:
        - ALL
        add:
        - NET_BIND_SERVICE
"#;
        let objects = parse_yaml(yaml).unwrap();
        assert_eq!(objects.len(), 1);

        if let K8sObject::Pod(pod) = &objects[0].k8s_object {
            let spec = pod.spec.as_ref().unwrap();
            let psc = spec.security_context.as_ref().unwrap();
            assert_eq!(psc.run_as_non_root, Some(true));
            assert_eq!(psc.run_as_user, Some(1000));

            let csc = spec.containers[0].security_context.as_ref().unwrap();
            assert_eq!(csc.privileged, Some(false));
            assert_eq!(csc.allow_privilege_escalation, Some(false));
            assert_eq!(csc.read_only_root_filesystem, Some(true));

            let caps = csc.capabilities.as_ref().unwrap();
            assert_eq!(caps.drop, vec!["ALL"]);
            assert_eq!(caps.add, vec!["NET_BIND_SERVICE"]);
        } else {
            panic!("Expected Pod");
        }
    }

    #[test]
    fn test_parse_unknown_crd() {
        let yaml = r#"
apiVersion: custom.io/v1
kind: MyCustomResource
metadata:
  name: my-custom
  namespace: custom-ns
spec:
  customField: value
"#;
        let objects = parse_yaml(yaml).unwrap();
        assert_eq!(objects.len(), 1);

        if let K8sObject::Unknown(obj) = &objects[0].k8s_object {
            assert_eq!(obj.api_version, "custom.io/v1");
            assert_eq!(obj.kind, "MyCustomResource");
            assert_eq!(obj.name, "my-custom");
            assert_eq!(obj.namespace, Some("custom-ns".to_string()));
        } else {
            panic!("Expected Unknown");
        }
    }

    #[test]
    fn test_parse_empty_yaml() {
        let yaml = "";
        let objects = parse_yaml(yaml).unwrap();
        assert!(objects.is_empty());
    }

    #[test]
    fn test_parse_comment_only() {
        let yaml = "# This is a comment\n# Another comment";
        let objects = parse_yaml(yaml).unwrap();
        assert!(objects.is_empty());
    }
}
