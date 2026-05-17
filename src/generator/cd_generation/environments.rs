//! CD-12 — Environment Strategy Module
//!
//! Generates multi-environment deployment strategy with `needs:` chains,
//! `if:` conditions based on branch filters, and GitHub Environment
//! references for approval gates.
//!
//! For a typical setup with staging + production:
//!
//! ```yaml
//! jobs:
//!   deploy-staging:
//!     environment: staging
//!     if: github.ref == 'refs/heads/develop'
//!     ...
//!
//!   deploy-production:
//!     environment: production
//!     needs: deploy-staging
//!     if: github.ref == 'refs/heads/main'
//!     ...
//! ```

use super::schema::EnvironmentConfig;

// ── Types ─────────────────────────────────────────────────────────────────────

/// Represents a single environment job in the multi-env deploy chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentJob {
    /// Job id used in the YAML (e.g. `deploy-staging`).
    pub job_id: String,
    /// GitHub Environment name (e.g. `staging`).
    pub environment_name: String,
    /// The `if:` condition for this job (e.g. `github.ref == 'refs/heads/main'`).
    pub condition: Option<String>,
    /// The `needs:` dependency — previous job id in the chain.
    pub needs: Option<String>,
    /// Whether this environment requires manual approval (GitHub Environment protection rule).
    pub requires_approval: bool,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Generates the ordered list of environment jobs from the pipeline's
/// environment configs.
///
/// Each job depends on the previous one via `needs:`, creating a deploy chain.
pub fn generate_environment_jobs(environments: &[EnvironmentConfig]) -> Vec<EnvironmentJob> {
    let mut jobs = Vec::with_capacity(environments.len());

    for (i, env) in environments.iter().enumerate() {
        let job_id = format!("deploy-{}", env.name);
        let needs = if i > 0 {
            Some(format!("deploy-{}", environments[i - 1].name))
        } else {
            None
        };
        let condition = env.branch_filter.as_ref().map(|branch| {
            format!("github.ref == 'refs/heads/{branch}'")
        });

        jobs.push(EnvironmentJob {
            job_id,
            environment_name: env.name.clone(),
            condition,
            needs,
            requires_approval: env.requires_approval,
        });
    }

    jobs
}

/// Renders the `jobs:` block header for a single environment job.
///
/// This is a YAML snippet that goes at the start of each per-env deploy job.
pub fn render_environment_job_header(job: &EnvironmentJob) -> String {
    let mut yaml = format!("  {}:\n", job.job_id);
    yaml.push_str("    runs-on: ubuntu-latest\n");
    yaml.push_str(&format!(
        "    environment: {}\n",
        job.environment_name
    ));

    if let Some(ref needs) = job.needs {
        yaml.push_str(&format!("    needs: {needs}\n"));
    }

    if let Some(ref cond) = job.condition {
        yaml.push_str(&format!("    if: {cond}\n"));
    }

    yaml.push_str("    steps:\n");
    yaml
}

/// Renders all environment job headers as a complete multi-job `jobs:` block.
pub fn render_environment_jobs_yaml(jobs: &[EnvironmentJob]) -> String {
    let mut yaml = String::new();
    for job in jobs {
        yaml.push_str(&render_environment_job_header(job));
    }
    yaml
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::cd_generation::schema::EnvironmentConfig;

    fn sample_environments() -> Vec<EnvironmentConfig> {
        vec![
            EnvironmentConfig {
                name: "staging".to_string(),
                branch_filter: Some("develop".to_string()),
                requires_approval: false,
                app_url: None,
                namespace: None,
                replicas: Some(1),
            },
            EnvironmentConfig {
                name: "production".to_string(),
                branch_filter: Some("main".to_string()),
                requires_approval: true,
                app_url: None,
                namespace: None,
                replicas: Some(2),
            },
        ]
    }

    fn three_environments() -> Vec<EnvironmentConfig> {
        vec![
            EnvironmentConfig {
                name: "dev".to_string(),
                branch_filter: Some("develop".to_string()),
                requires_approval: false,
                app_url: None,
                namespace: Some("dev".to_string()),
                replicas: Some(1),
            },
            EnvironmentConfig {
                name: "staging".to_string(),
                branch_filter: Some("develop".to_string()),
                requires_approval: false,
                app_url: None,
                namespace: Some("staging".to_string()),
                replicas: Some(1),
            },
            EnvironmentConfig {
                name: "production".to_string(),
                branch_filter: Some("main".to_string()),
                requires_approval: true,
                app_url: None,
                namespace: Some("production".to_string()),
                replicas: Some(2),
            },
        ]
    }

    #[test]
    fn generates_correct_number_of_jobs() {
        let envs = sample_environments();
        let jobs = generate_environment_jobs(&envs);
        assert_eq!(jobs.len(), 2);
    }

    #[test]
    fn first_job_has_no_needs() {
        let envs = sample_environments();
        let jobs = generate_environment_jobs(&envs);
        assert!(jobs[0].needs.is_none());
    }

    #[test]
    fn second_job_needs_first() {
        let envs = sample_environments();
        let jobs = generate_environment_jobs(&envs);
        assert_eq!(jobs[1].needs.as_deref(), Some("deploy-staging"));
    }

    #[test]
    fn job_id_uses_env_name() {
        let envs = sample_environments();
        let jobs = generate_environment_jobs(&envs);
        assert_eq!(jobs[0].job_id, "deploy-staging");
        assert_eq!(jobs[1].job_id, "deploy-production");
    }

    #[test]
    fn branch_filter_becomes_condition() {
        let envs = sample_environments();
        let jobs = generate_environment_jobs(&envs);
        assert_eq!(
            jobs[0].condition.as_deref(),
            Some("github.ref == 'refs/heads/develop'")
        );
        assert_eq!(
            jobs[1].condition.as_deref(),
            Some("github.ref == 'refs/heads/main'")
        );
    }

    #[test]
    fn production_requires_approval() {
        let envs = sample_environments();
        let jobs = generate_environment_jobs(&envs);
        assert!(!jobs[0].requires_approval);
        assert!(jobs[1].requires_approval);
    }

    #[test]
    fn three_env_chain_has_correct_needs() {
        let envs = three_environments();
        let jobs = generate_environment_jobs(&envs);
        assert_eq!(jobs.len(), 3);
        assert!(jobs[0].needs.is_none());
        assert_eq!(jobs[1].needs.as_deref(), Some("deploy-dev"));
        assert_eq!(jobs[2].needs.as_deref(), Some("deploy-staging"));
    }

    #[test]
    fn no_condition_when_no_branch_filter() {
        let envs = vec![EnvironmentConfig {
            name: "custom".to_string(),
            branch_filter: None,
            requires_approval: false,
            app_url: None,
            namespace: None,
            replicas: None,
        }];
        let jobs = generate_environment_jobs(&envs);
        assert!(jobs[0].condition.is_none());
    }

    #[test]
    fn render_header_contains_environment() {
        let envs = sample_environments();
        let jobs = generate_environment_jobs(&envs);
        let yaml = render_environment_job_header(&jobs[0]);
        assert!(yaml.contains("environment: staging"));
        assert!(yaml.contains("deploy-staging:"));
        assert!(yaml.contains("runs-on: ubuntu-latest"));
    }

    #[test]
    fn render_header_contains_needs() {
        let envs = sample_environments();
        let jobs = generate_environment_jobs(&envs);
        let yaml = render_environment_job_header(&jobs[1]);
        assert!(yaml.contains("needs: deploy-staging"));
    }

    #[test]
    fn render_header_contains_condition() {
        let envs = sample_environments();
        let jobs = generate_environment_jobs(&envs);
        let yaml = render_environment_job_header(&jobs[0]);
        assert!(yaml.contains("if: github.ref == 'refs/heads/develop'"));
    }

    #[test]
    fn render_all_jobs_produces_both() {
        let envs = sample_environments();
        let jobs = generate_environment_jobs(&envs);
        let yaml = render_environment_jobs_yaml(&jobs);
        assert!(yaml.contains("deploy-staging:"));
        assert!(yaml.contains("deploy-production:"));
    }

    #[test]
    fn empty_environments_produces_empty_jobs() {
        let jobs = generate_environment_jobs(&[]);
        assert!(jobs.is_empty());
    }
}
