//! Dclint-RS: Native Rust Docker Compose Linter
//!
//! A Rust translation of the docker-compose-linter project.
//!
//! # Attribution
//!
//! This module is a derivative work based on [docker-compose-linter](https://github.com/zavoloklom/docker-compose-linter),
//! originally written in TypeScript by Sergey Kupletsky.
//!
//! **Original Project:** <https://github.com/zavoloklom/docker-compose-linter>
//! **Original License:** MIT
//!
//! # Features
//!
//! - Docker Compose YAML parsing with position tracking
//! - 15 configurable linting rules (DCL001-DCL015)
//! - Auto-fix capability for 8 rules
//! - Multiple output formats (JSON, Stylish, GitHub Actions, etc.)
//! - Comment-based rule disabling
//!
//! # Example
//!
//! ```rust,ignore
//! use syncable_cli::analyzer::dclint::{lint, DclintConfig, LintResult};
//!
//! let compose = r#"
//! services:
//!   web:
//!     image: nginx:latest
//!     ports:
//!       - "8080:80"
//! "#;
//!
//! let config = DclintConfig::default();
//! let result = lint(compose, &config);
//!
//! for failure in result.failures {
//!     println!("{}: {} - {}", failure.line, failure.code, failure.message);
//! }
//! ```
//!
//! # Rules
//!
//! | Code   | Name                                    | Fixable | Description                                    |
//! |--------|-----------------------------------------|---------|------------------------------------------------|
//! | DCL001 | no-build-and-image                      | No      | Service cannot have both build and image       |
//! | DCL002 | no-duplicate-container-names            | No      | Container names must be unique                 |
//! | DCL003 | no-duplicate-exported-ports             | No      | Exported ports must be unique                  |
//! | DCL004 | no-quotes-in-volumes                    | Yes     | Volume paths should not be quoted              |
//! | DCL005 | no-unbound-port-interfaces              | No      | Ports should bind to specific interface        |
//! | DCL006 | no-version-field                        | Yes     | Version field is deprecated                    |
//! | DCL007 | require-project-name-field              | No      | Require top-level name field                   |
//! | DCL008 | require-quotes-in-ports                 | Yes     | Port mappings should be quoted                 |
//! | DCL009 | service-container-name-regex            | No      | Container name format validation               |
//! | DCL010 | service-dependencies-alphabetical-order | Yes     | Sort depends_on alphabetically                 |
//! | DCL011 | service-image-require-explicit-tag      | No      | Images need explicit tags                      |
//! | DCL012 | service-keys-order                      | Yes     | Service keys in standard order                 |
//! | DCL013 | service-ports-alphabetical-order        | Yes     | Sort ports alphabetically                      |
//! | DCL014 | services-alphabetical-order             | Yes     | Sort services alphabetically                   |
//! | DCL015 | top-level-properties-order              | Yes     | Top-level keys in standard order               |

pub mod config;
pub mod formatter;
pub mod lint;
pub mod parser;
pub mod pragma;
pub mod rules;
pub mod types;

// Re-export main types and functions
pub use config::DclintConfig;
pub use formatter::{OutputFormat, format_result, format_result_to_string, format_results};
pub use lint::{LintResult, fix_content, fix_file, lint, lint_file, lint_with_path};
pub use types::{CheckFailure, ConfigLevel, RuleCategory, RuleCode, RuleMeta, Severity};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_basic() {
        let yaml = r#"
services:
  web:
    image: nginx:1.25
"#;
        let result = lint(yaml, &DclintConfig::default());
        assert!(result.parse_errors.is_empty());
    }

    #[test]
    fn test_lint_with_errors() {
        let yaml = r#"
services:
  web:
    build: .
    image: nginx
"#;
        let result = lint(yaml, &DclintConfig::default());
        assert!(result.parse_errors.is_empty());
        // Should catch DCL001 and DCL011
        assert!(result.failures.iter().any(|f| f.code.as_str() == "DCL001"));
    }

    #[test]
    fn test_config_ignore() {
        let yaml = r#"
services:
  web:
    build: .
    image: nginx
"#;
        let config = DclintConfig::default().ignore("DCL001");
        let result = lint(yaml, &config);
        assert!(!result.failures.iter().any(|f| f.code.as_str() == "DCL001"));
    }

    #[test]
    fn test_format_json() {
        let yaml = r#"
services:
  web:
    image: nginx
"#;
        let result = lint(yaml, &DclintConfig::default());
        let output = format_result(&result, OutputFormat::Json);
        assert!(output.contains("filePath"));
    }
}
