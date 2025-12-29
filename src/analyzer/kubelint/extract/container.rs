//! Container extraction utilities.

use crate::analyzer::kubelint::context::object::*;

/// Extract all containers from a PodSpec (containers + init containers).
pub fn extract_all_containers(pod_spec: &PodSpec) -> Vec<&ContainerSpec> {
    let mut containers: Vec<&ContainerSpec> = pod_spec.containers.iter().collect();
    containers.extend(pod_spec.init_containers.iter());
    containers
}

/// Alias for extract_all_containers.
pub fn all_containers(pod_spec: &PodSpec) -> Vec<&ContainerSpec> {
    extract_all_containers(pod_spec)
}

/// Extract only regular containers (not init containers).
pub fn extract_containers(pod_spec: &PodSpec) -> Vec<&ContainerSpec> {
    pod_spec.containers.iter().collect()
}

/// Alias for extract_containers.
pub fn containers(pod_spec: &PodSpec) -> Vec<&ContainerSpec> {
    extract_containers(pod_spec)
}

/// Extract only init containers.
pub fn extract_init_containers(pod_spec: &PodSpec) -> Vec<&ContainerSpec> {
    pod_spec.init_containers.iter().collect()
}

/// Alias for extract_init_containers.
pub fn init_containers(pod_spec: &PodSpec) -> Vec<&ContainerSpec> {
    extract_init_containers(pod_spec)
}
