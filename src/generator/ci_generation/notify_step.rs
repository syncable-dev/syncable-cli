//! CI-24 — Notification Step (CI Failure)
//!
//! Optional step emitted when `--notify` is passed on the CLI or `notify =
//! true` is set in `.syncable.ci.toml`.  The rendered step fires only on job
//! failure (`if: failure()`) and requires two repository secrets.
//!
//! ## Generated YAML (GitHub Actions)
//!
//! ```yaml
//! - name: Notify on failure
//!   if: failure()
//!   uses: slackapi/slack-github-action@v2
//!   with:
//!     channel-id: ${{ secrets.SLACK_CHANNEL_ID }}
//!     slack-bot-token: ${{ secrets.SLACK_BOT_TOKEN }}
//!     payload: |
//!       {"text": "❌ CI failed on `${{ github.ref_name }}` — ${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}"}
//! ```
//!
//! Both `SLACK_BOT_TOKEN` and `SLACK_CHANNEL_ID` are appended as *optional*
//! entries in `SECRETS_REQUIRED.md` so the user knows exactly where to
//! configure them.

// ── Public types ──────────────────────────────────────────────────────────────

/// A resolved Slack notification step, ready for YAML rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyStep {
    /// Repository secret name for the Slack bot token.
    pub token_secret: String,
    /// Repository secret name for the Slack channel ID.
    pub channel_secret: String,
}

impl Default for NotifyStep {
    fn default() -> Self {
        Self {
            token_secret: "SLACK_BOT_TOKEN".to_string(),
            channel_secret: "SLACK_CHANNEL_ID".to_string(),
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns `Some(NotifyStep)` when `enabled` is true, `None` otherwise.
///
/// Template builders call this with the resolved `notify` flag so the step
/// is omitted from YAML when notifications are not requested.
pub fn generate_notify_step(enabled: bool) -> Option<NotifyStep> {
    if enabled { Some(NotifyStep::default()) } else { None }
}

/// Renders the notify step as a GitHub Actions YAML step snippet.
///
/// The step is conditionally gated with `if: failure()` and references both
/// secrets via `${{ secrets.* }}` expressions so no secret values appear in
/// the generated file.
pub fn render_notify_yaml(step: &NotifyStep) -> String {
    format!(
        "\
      - name: Notify on failure
        if: failure()
        uses: slackapi/slack-github-action@v2
        with:
          channel-id: ${{{{ secrets.{channel} }}}}
          slack-bot-token: ${{{{ secrets.{token} }}}}
          payload: |
            {{\"text\": \"\\u274c CI failed on `${{{{ github.ref_name }}}}` \\u2014 ${{{{ github.server_url }}}}/${{{{ github.repository }}}}/actions/runs/${{{{ github.run_id }}}}\"}}\n",
        channel = step.channel_secret,
        token = step.token_secret,
    )
}

/// Renders the `SLACK_BOT_TOKEN` and `SLACK_CHANNEL_ID` entries for
/// `SECRETS_REQUIRED.md`.
pub fn notify_secrets_doc_entries(step: &NotifyStep) -> String {
    format!(
        "\
### `{token}` *(optional)*

Slack bot OAuth token used by the CI failure notification step.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:** <https://api.slack.com/apps> → your app → OAuth & Permissions → Bot User OAuth Token

---

### `{channel}` *(optional)*

Slack channel ID that receives CI failure notifications.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**How to obtain:** Right-click a channel in Slack → Copy link — the ID is the last path segment (e.g. `C0123ABCDEF`).\n",
        token = step.token_secret,
        channel = step.channel_secret,
    )
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── generate_notify_step ──────────────────────────────────────────

    #[test]
    fn test_returns_none_when_disabled() {
        assert!(generate_notify_step(false).is_none());
    }

    #[test]
    fn test_returns_some_when_enabled() {
        assert!(generate_notify_step(true).is_some());
    }

    #[test]
    fn test_default_secret_names() {
        let step = generate_notify_step(true).unwrap();
        assert_eq!(step.token_secret, "SLACK_BOT_TOKEN");
        assert_eq!(step.channel_secret, "SLACK_CHANNEL_ID");
    }

    // ── render_notify_yaml ────────────────────────────────────────────

    #[test]
    fn test_yaml_contains_action_reference() {
        let step = generate_notify_step(true).unwrap();
        let yaml = render_notify_yaml(&step);
        assert!(yaml.contains("slackapi/slack-github-action@v2"));
    }

    #[test]
    fn test_yaml_gated_on_failure() {
        let step = generate_notify_step(true).unwrap();
        let yaml = render_notify_yaml(&step);
        assert!(yaml.contains("if: failure()"));
    }

    #[test]
    fn test_yaml_references_channel_secret() {
        let step = generate_notify_step(true).unwrap();
        let yaml = render_notify_yaml(&step);
        assert!(yaml.contains("SLACK_CHANNEL_ID"));
    }

    #[test]
    fn test_yaml_references_token_secret() {
        let step = generate_notify_step(true).unwrap();
        let yaml = render_notify_yaml(&step);
        assert!(yaml.contains("SLACK_BOT_TOKEN"));
    }

    #[test]
    fn test_yaml_contains_payload_with_run_id() {
        let step = generate_notify_step(true).unwrap();
        let yaml = render_notify_yaml(&step);
        assert!(yaml.contains("github.run_id"));
    }

    #[test]
    fn test_yaml_no_hardcoded_secret_values() {
        let step = generate_notify_step(true).unwrap();
        let yaml = render_notify_yaml(&step);
        // Ensure secrets are referenced, not embedded
        assert!(!yaml.contains("xoxb-"));
        assert!(!yaml.contains("xapp-"));
    }

    #[test]
    fn test_custom_secret_names_propagated() {
        let step = NotifyStep {
            token_secret: "MY_SLACK_TOKEN".to_string(),
            channel_secret: "MY_SLACK_CHANNEL".to_string(),
        };
        let yaml = render_notify_yaml(&step);
        assert!(yaml.contains("MY_SLACK_TOKEN"));
        assert!(yaml.contains("MY_SLACK_CHANNEL"));
        assert!(!yaml.contains("SLACK_BOT_TOKEN"));
    }

    // ── notify_secrets_doc_entries ────────────────────────────────────

    #[test]
    fn test_secrets_doc_contains_both_secrets() {
        let step = generate_notify_step(true).unwrap();
        let doc = notify_secrets_doc_entries(&step);
        assert!(doc.contains("SLACK_BOT_TOKEN"));
        assert!(doc.contains("SLACK_CHANNEL_ID"));
    }

    #[test]
    fn test_secrets_doc_marks_both_as_optional() {
        let step = generate_notify_step(true).unwrap();
        let doc = notify_secrets_doc_entries(&step);
        assert_eq!(doc.matches("optional").count(), 2);
    }

    #[test]
    fn test_secrets_doc_includes_setup_instructions() {
        let step = generate_notify_step(true).unwrap();
        let doc = notify_secrets_doc_entries(&step);
        assert!(doc.contains("api.slack.com"));
    }
}
