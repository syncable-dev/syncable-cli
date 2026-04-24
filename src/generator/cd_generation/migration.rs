//! CD-10 — Database Migration Step Generator
//!
//! Generates GitHub Actions YAML snippets for running database migrations
//! before the deployment step. The migration tool is detected by context
//! collection (CD-02) and stored in `CdContext.migration_tool`.
//!
//! | Tool              | Detection                   | Command                        |
//! |-------------------|-----------------------------|--------------------------------|
//! | Flyway            | `flyway.conf`               | `flyway migrate`               |
//! | Liquibase         | `liquibase.properties`      | `liquibase update`             |
//! | Alembic           | `alembic.ini`               | `alembic upgrade head`         |
//! | Django            | `manage.py`                 | `python manage.py migrate`     |
//! | Prisma            | `schema.prisma`             | `npx prisma migrate deploy`    |
//! | sqlx              | `sqlx-data.json` / `.sqlx/` | `sqlx migrate run`             |
//! | Diesel            | `diesel.toml`               | `diesel migration run`         |
//!
//! For Hetzner VPS targets, the migration command is executed via SSH.

use super::context::MigrationTool;
use super::schema::MigrationStep;

// ── Public API ────────────────────────────────────────────────────────────────

/// Generates a `MigrationStep` for the detected migration tool.
///
/// Returns `None` when no migration tool was detected.
///
/// The `via_ssh` flag is set when the target is a Hetzner VPS (migration
/// must run on the remote host rather than in the runner).
pub fn generate_migration_step(
    tool: Option<&MigrationTool>,
    via_ssh: bool,
) -> Option<MigrationStep> {
    let tool = tool?;
    let command = migration_command(tool);

    Some(MigrationStep {
        tool: tool.clone(),
        command,
        via_ssh,
    })
}

/// Returns the canonical migration command for the given tool.
pub fn migration_command(tool: &MigrationTool) -> String {
    match tool {
        MigrationTool::Flyway => "flyway migrate".to_string(),
        MigrationTool::Liquibase => "liquibase update".to_string(),
        MigrationTool::Alembic => "alembic upgrade head".to_string(),
        MigrationTool::DjangoMigrations => "python manage.py migrate --noinput".to_string(),
        MigrationTool::Prisma => "npx prisma migrate deploy".to_string(),
        MigrationTool::Sqlx => "sqlx migrate run".to_string(),
        MigrationTool::Diesel => "diesel migration run".to_string(),
    }
}

/// Renders the migration step as a GitHub Actions YAML snippet.
///
/// When `via_ssh` is true, wraps the command in an SSH invocation.
pub fn render_migration_yaml(step: &MigrationStep) -> String {
    if step.via_ssh {
        format!(
            "\
      - name: Run database migrations ({tool}) via SSH
        run: |
          ssh ${{{{ secrets.SSH_USER }}}}@${{{{ secrets.SSH_HOST }}}} << 'MIGRATE_EOF'
            cd /opt/app && {command}
          MIGRATE_EOF
        env:
          DATABASE_URL: ${{{{ secrets.DATABASE_URL }}}}\n",
            tool = step.tool,
            command = step.command,
        )
    } else {
        format!(
            "\
      - name: Run database migrations ({tool})
        run: {command}
        env:
          DATABASE_URL: ${{{{ secrets.DATABASE_URL }}}}\n",
            tool = step.tool,
            command = step.command,
        )
    }
}

/// Returns secrets required for the migration step.
pub fn migration_required_secrets(step: &MigrationStep) -> Vec<String> {
    let mut secrets = vec!["DATABASE_URL".to_string()];
    if step.via_ssh {
        secrets.push("SSH_USER".to_string());
        secrets.push("SSH_HOST".to_string());
    }
    secrets
}

/// Renders secrets documentation for the migration step.
pub fn migration_secrets_doc(step: &MigrationStep) -> String {
    let mut doc = format!(
        "\
### `DATABASE_URL` *(required)*

Database connection string used by `{}` for running migrations.

**Where to set:** Repository → Settings → Secrets and variables → Actions

**Format examples:**
- PostgreSQL: `postgresql://user:pass@host:5432/dbname`
- MySQL: `mysql://user:pass@host:3306/dbname`
- SQLite: `sqlite:./db.sqlite`\n",
        step.tool
    );

    if step.via_ssh {
        doc.push_str(
            "\n\
**Note:** This secret is passed as an environment variable to the SSH session.
Ensure the database is reachable from the VPS, not from the GitHub Actions runner.\n",
        );
    }

    doc
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── generate_migration_step ───────────────────────────────────────

    #[test]
    fn none_tool_returns_none() {
        assert!(generate_migration_step(None, false).is_none());
    }

    #[test]
    fn prisma_returns_some() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), false);
        assert!(step.is_some());
    }

    #[test]
    fn prisma_command() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), false).unwrap();
        assert_eq!(step.command, "npx prisma migrate deploy");
    }

    #[test]
    fn diesel_command() {
        let step = generate_migration_step(Some(&MigrationTool::Diesel), false).unwrap();
        assert_eq!(step.command, "diesel migration run");
    }

    #[test]
    fn alembic_command() {
        let step = generate_migration_step(Some(&MigrationTool::Alembic), false).unwrap();
        assert_eq!(step.command, "alembic upgrade head");
    }

    #[test]
    fn django_command_has_noinput() {
        let step = generate_migration_step(Some(&MigrationTool::DjangoMigrations), false).unwrap();
        assert!(step.command.contains("--noinput"));
    }

    #[test]
    fn flyway_command() {
        let step = generate_migration_step(Some(&MigrationTool::Flyway), false).unwrap();
        assert_eq!(step.command, "flyway migrate");
    }

    #[test]
    fn liquibase_command() {
        let step = generate_migration_step(Some(&MigrationTool::Liquibase), false).unwrap();
        assert_eq!(step.command, "liquibase update");
    }

    #[test]
    fn sqlx_command() {
        let step = generate_migration_step(Some(&MigrationTool::Sqlx), false).unwrap();
        assert_eq!(step.command, "sqlx migrate run");
    }

    #[test]
    fn via_ssh_flag_preserved() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), true).unwrap();
        assert!(step.via_ssh);
    }

    #[test]
    fn not_via_ssh_by_default() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), false).unwrap();
        assert!(!step.via_ssh);
    }

    // ── migration_command ─────────────────────────────────────────────

    #[test]
    fn all_tools_produce_nonempty_command() {
        let tools = [
            MigrationTool::Flyway,
            MigrationTool::Liquibase,
            MigrationTool::Alembic,
            MigrationTool::DjangoMigrations,
            MigrationTool::Prisma,
            MigrationTool::Sqlx,
            MigrationTool::Diesel,
        ];
        for tool in &tools {
            let cmd = migration_command(tool);
            assert!(!cmd.is_empty(), "Empty command for {tool}");
        }
    }

    // ── render_migration_yaml ─────────────────────────────────────────

    #[test]
    fn local_yaml_contains_run_command() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), false).unwrap();
        let yaml = render_migration_yaml(&step);
        assert!(yaml.contains("npx prisma migrate deploy"));
    }

    #[test]
    fn local_yaml_references_database_url() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), false).unwrap();
        let yaml = render_migration_yaml(&step);
        assert!(yaml.contains("secrets.DATABASE_URL"));
    }

    #[test]
    fn local_yaml_does_not_contain_ssh() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), false).unwrap();
        let yaml = render_migration_yaml(&step);
        assert!(!yaml.contains("ssh"));
    }

    #[test]
    fn ssh_yaml_contains_ssh_command() {
        let step = generate_migration_step(Some(&MigrationTool::Alembic), true).unwrap();
        let yaml = render_migration_yaml(&step);
        assert!(yaml.contains("ssh"));
    }

    #[test]
    fn ssh_yaml_references_ssh_secrets() {
        let step = generate_migration_step(Some(&MigrationTool::Alembic), true).unwrap();
        let yaml = render_migration_yaml(&step);
        assert!(yaml.contains("secrets.SSH_USER"));
        assert!(yaml.contains("secrets.SSH_HOST"));
    }

    #[test]
    fn ssh_yaml_contains_migration_command() {
        let step = generate_migration_step(Some(&MigrationTool::Alembic), true).unwrap();
        let yaml = render_migration_yaml(&step);
        assert!(yaml.contains("alembic upgrade head"));
    }

    #[test]
    fn yaml_step_name_contains_tool_name() {
        let step = generate_migration_step(Some(&MigrationTool::Diesel), false).unwrap();
        let yaml = render_migration_yaml(&step);
        assert!(yaml.contains("diesel"));
    }

    // ── migration_required_secrets ────────────────────────────────────

    #[test]
    fn local_requires_database_url() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), false).unwrap();
        let secrets = migration_required_secrets(&step);
        assert!(secrets.contains(&"DATABASE_URL".to_string()));
        assert_eq!(secrets.len(), 1);
    }

    #[test]
    fn ssh_requires_database_url_and_ssh_secrets() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), true).unwrap();
        let secrets = migration_required_secrets(&step);
        assert!(secrets.contains(&"DATABASE_URL".to_string()));
        assert!(secrets.contains(&"SSH_USER".to_string()));
        assert!(secrets.contains(&"SSH_HOST".to_string()));
        assert_eq!(secrets.len(), 3);
    }

    // ── migration_secrets_doc ─────────────────────────────────────────

    #[test]
    fn secrets_doc_mentions_database_url() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), false).unwrap();
        let doc = migration_secrets_doc(&step);
        assert!(doc.contains("DATABASE_URL"));
    }

    #[test]
    fn secrets_doc_mentions_tool_name() {
        let step = generate_migration_step(Some(&MigrationTool::Diesel), false).unwrap();
        let doc = migration_secrets_doc(&step);
        assert!(doc.contains("diesel"));
    }

    #[test]
    fn ssh_secrets_doc_mentions_vpn_note() {
        let step = generate_migration_step(Some(&MigrationTool::Prisma), true).unwrap();
        let doc = migration_secrets_doc(&step);
        assert!(doc.contains("VPS"));
    }

    #[test]
    fn secrets_doc_contains_format_examples() {
        let step = generate_migration_step(Some(&MigrationTool::Sqlx), false).unwrap();
        let doc = migration_secrets_doc(&step);
        assert!(doc.contains("postgresql://"));
    }
}
