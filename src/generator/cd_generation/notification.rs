//! CD-21 — Deployment Notifications (Slack)
//!
//! Generates a Slack notification step that fires on success/failure of the
//! deploy job.  Uses `slackapi/slack-github-action@v2` with a payload that
//! includes repo name, environment, branch, commit SHA, and status emoji.
//!
//! ```yaml
//! - name: Notify Slack
//!   if: always()
//!   uses: slackapi/slack-github-action@v2
//!   with:
//!     webhook: ${{ secrets.SLACK_WEBHOOK_URL }}
//!     payload: |
//!       {"text":"✅ *my-app* deployed to *production* …"}
//! ```

use super::schema::NotificationStep;

// ── Public API ────────────────────────────────────────────────────────────────

/// Build a `NotificationStep` from user preferences.
pub fn generate_notification_step(
    webhook_secret_name: &str,
    on_success: bool,
    on_failure: bool,
) -> NotificationStep {
    NotificationStep {
        channel_type: "slack".to_string(),
        webhook_secret: webhook_secret_name.to_string(),
        on_success,
        on_failure,
    }
}

/// Renders a Slack notification step as a GitHub Actions YAML snippet.
///
/// The step uses `if: always()` so it fires regardless of prior step outcomes.
/// The payload JSON includes dynamic GitHub context expressions.
pub fn render_notification_yaml(step: &NotificationStep) -> String {
    let condition = notification_condition(step.on_success, step.on_failure);

    let mut yaml = String::new();
    yaml.push_str("      - name: Notify Slack\n");
    yaml.push_str(&format!("        if: {condition}\n"));
    yaml.push_str("        uses: slackapi/slack-github-action@v2\n");
    yaml.push_str("        with:\n");
    yaml.push_str(&format!(
        "          webhook: ${{{{ secrets.{} }}}}\n",
        step.webhook_secret
    ));
    yaml.push_str("          webhook-type: incoming-webhook\n");
    yaml.push_str("          payload: |\n");
    yaml.push_str("            {\n");
    yaml.push_str("              \"text\": \"${{ job.status == 'success' && '✅' || '❌' }} *${{ github.repository }}* deploy to *${{ github.ref_name }}* — ${{ job.status }}\\nCommit: `${{ github.sha }}` by ${{ github.actor }}\"\n");
    yaml.push_str("            }\n");

    yaml
}

/// Returns a short summary of the notification configuration.
pub fn notification_summary(step: &NotificationStep) -> String {
    let events: Vec<&str> = [
        step.on_success.then_some("success"),
        step.on_failure.then_some("failure"),
    ]
    .into_iter()
    .flatten()
    .collect();

    format!(
        "{} notification via {} on: {}",
        step.channel_type,
        step.webhook_secret,
        events.join(", ")
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Determine the `if:` condition based on success/failure flags.
fn notification_condition(on_success: bool, on_failure: bool) -> &'static str {
    match (on_success, on_failure) {
        (true, true) => "always()",
        (true, false) => "success()",
        (false, true) => "failure()",
        // If neither flag is set we still emit, but guard with always()
        (false, false) => "always()",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_step_sets_channel_type() {
        let step = generate_notification_step("SLACK_WEBHOOK_URL", true, true);
        assert_eq!(step.channel_type, "slack");
    }

    #[test]
    fn generate_step_webhook_secret() {
        let step = generate_notification_step("DEPLOY_SLACK_HOOK", true, false);
        assert_eq!(step.webhook_secret, "DEPLOY_SLACK_HOOK");
    }

    #[test]
    fn generate_step_on_success_flag() {
        let step = generate_notification_step("HOOK", true, false);
        assert!(step.on_success);
        assert!(!step.on_failure);
    }

    #[test]
    fn generate_step_on_failure_flag() {
        let step = generate_notification_step("HOOK", false, true);
        assert!(!step.on_success);
        assert!(step.on_failure);
    }

    #[test]
    fn yaml_contains_slack_action() {
        let step = generate_notification_step("SLACK_WEBHOOK_URL", true, true);
        let yaml = render_notification_yaml(&step);
        assert!(yaml.contains("slackapi/slack-github-action@v2"));
    }

    #[test]
    fn yaml_always_condition_when_both() {
        let step = generate_notification_step("HOOK", true, true);
        let yaml = render_notification_yaml(&step);
        assert!(yaml.contains("if: always()"));
    }

    #[test]
    fn yaml_success_condition_when_success_only() {
        let step = generate_notification_step("HOOK", true, false);
        let yaml = render_notification_yaml(&step);
        assert!(yaml.contains("if: success()"));
    }

    #[test]
    fn yaml_failure_condition_when_failure_only() {
        let step = generate_notification_step("HOOK", false, true);
        let yaml = render_notification_yaml(&step);
        assert!(yaml.contains("if: failure()"));
    }

    #[test]
    fn yaml_references_webhook_secret() {
        let step = generate_notification_step("MY_SLACK_HOOK", true, true);
        let yaml = render_notification_yaml(&step);
        assert!(yaml.contains("secrets.MY_SLACK_HOOK"));
    }

    #[test]
    fn yaml_contains_payload() {
        let step = generate_notification_step("HOOK", true, true);
        let yaml = render_notification_yaml(&step);
        assert!(yaml.contains("payload:"));
        assert!(yaml.contains("github.repository"));
    }

    #[test]
    fn yaml_payload_includes_status_emoji() {
        let step = generate_notification_step("HOOK", true, true);
        let yaml = render_notification_yaml(&step);
        assert!(yaml.contains("✅"));
        assert!(yaml.contains("❌"));
    }

    #[test]
    fn summary_both_events() {
        let step = generate_notification_step("HOOK", true, true);
        let summary = notification_summary(&step);
        assert!(summary.contains("success"));
        assert!(summary.contains("failure"));
    }

    #[test]
    fn summary_success_only() {
        let step = generate_notification_step("HOOK", true, false);
        let summary = notification_summary(&step);
        assert!(summary.contains("success"));
        assert!(!summary.contains("failure"));
    }

    #[test]
    fn summary_failure_only() {
        let step = generate_notification_step("HOOK", false, true);
        let summary = notification_summary(&step);
        assert!(!summary.contains("success"));
        assert!(summary.contains("failure"));
    }

    #[test]
    fn condition_helper_both() {
        assert_eq!(notification_condition(true, true), "always()");
    }

    #[test]
    fn condition_helper_success() {
        assert_eq!(notification_condition(true, false), "success()");
    }

    #[test]
    fn condition_helper_failure() {
        assert_eq!(notification_condition(false, true), "failure()");
    }

    #[test]
    fn condition_helper_neither() {
        assert_eq!(notification_condition(false, false), "always()");
    }
}
