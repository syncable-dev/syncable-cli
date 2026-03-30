//! CI-04 — Dependency cache strategy resolver.
//!
//! Maps a `CiContext`'s package manager and lock file to the GitHub Actions
//! `actions/cache` step configuration. Returns `None` when no lock file is
//! present so the caller can omit the step entirely.

use serde::Serialize;

use crate::generator::ci_generation::context::{CiContext, PackageManager};

// ── Public types ──────────────────────────────────────────────────────────────

/// Cache step configuration for `actions/cache`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CacheConfig {
    /// Directories the runner should persist between jobs.
    pub paths: Vec<String>,
    /// Primary cache key — busted when the lock file changes.
    pub key: String,
    /// Fallback prefix used when no exact key match exists.
    pub restore_keys: Vec<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns a `CacheConfig` for the detected package manager, or `None` when
/// no lock file was found (caller should omit the cache step entirely).
pub fn resolve_cache(ctx: &CiContext) -> Option<CacheConfig> {
    // Without a verified lock file on disk the cache key expression is
    // meaningless — skip the step rather than emit a broken config.
    ctx.lock_file.as_ref()?;

    Some(match ctx.package_manager {
        PackageManager::Npm => CacheConfig {
            paths: vec!["~/.npm".into()],
            key: "npm-${{ runner.os }}-${{ hashFiles('**/package-lock.json') }}".into(),
            restore_keys: vec!["npm-${{ runner.os }}-".into()],
        },
        PackageManager::Yarn => CacheConfig {
            paths: vec![".yarn/cache".into(), ".yarn/unplugged".into()],
            key: "yarn-${{ runner.os }}-${{ hashFiles('**/yarn.lock') }}".into(),
            restore_keys: vec!["yarn-${{ runner.os }}-".into()],
        },
        PackageManager::Pnpm => CacheConfig {
            paths: vec!["~/.pnpm-store".into()],
            key: "pnpm-${{ runner.os }}-${{ hashFiles('**/pnpm-lock.yaml') }}".into(),
            restore_keys: vec!["pnpm-${{ runner.os }}-".into()],
        },
        PackageManager::Bun => CacheConfig {
            paths: vec!["~/.bun/install/cache".into()],
            key: "bun-${{ runner.os }}-${{ hashFiles('**/bun.lock*') }}".into(),
            restore_keys: vec!["bun-${{ runner.os }}-".into()],
        },
        PackageManager::Pip => CacheConfig {
            paths: vec!["~/.cache/pip".into()],
            key: "pip-${{ runner.os }}-${{ hashFiles('**/requirements*.txt') }}".into(),
            restore_keys: vec!["pip-${{ runner.os }}-".into()],
        },
        PackageManager::Uv => CacheConfig {
            paths: vec!["~/.cache/uv".into()],
            key: "uv-${{ runner.os }}-${{ hashFiles('**/uv.lock') }}".into(),
            restore_keys: vec!["uv-${{ runner.os }}-".into()],
        },
        PackageManager::Poetry => CacheConfig {
            paths: vec!["~/.cache/pypoetry".into()],
            key: "poetry-${{ runner.os }}-${{ hashFiles('**/poetry.lock') }}".into(),
            restore_keys: vec!["poetry-${{ runner.os }}-".into()],
        },
        PackageManager::Cargo => CacheConfig {
            paths: vec![
                "~/.cargo/registry/index".into(),
                "~/.cargo/registry/cache".into(),
                "~/.cargo/git/db".into(),
                "target/".into(),
            ],
            key: "cargo-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}".into(),
            restore_keys: vec!["cargo-${{ runner.os }}-".into()],
        },
        PackageManager::GoMod => CacheConfig {
            paths: vec!["~/go/pkg/mod".into(), "~/.cache/go-build".into()],
            key: "go-${{ runner.os }}-${{ hashFiles('**/go.sum') }}".into(),
            restore_keys: vec!["go-${{ runner.os }}-".into()],
        },
        PackageManager::Maven => CacheConfig {
            paths: vec!["~/.m2/repository".into()],
            key: "maven-${{ runner.os }}-${{ hashFiles('**/pom.xml') }}".into(),
            restore_keys: vec!["maven-${{ runner.os }}-".into()],
        },
        PackageManager::Gradle => CacheConfig {
            paths: vec!["~/.gradle/caches".into(), "~/.gradle/wrapper".into()],
            key: "gradle-${{ runner.os }}-${{ hashFiles('**/*.gradle*', '**/gradle-wrapper.properties') }}".into(),
            restore_keys: vec!["gradle-${{ runner.os }}-".into()],
        },
        PackageManager::Unknown => return None,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::ci_generation::context::{Linter, PackageManager, TestFramework};
    use crate::generator::ci_generation::test_helpers::make_base_ctx;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_ctx(pm: PackageManager, lock_file: Option<PathBuf>, root: &std::path::Path) -> CiContext {
        CiContext { package_manager: pm, lock_file, ..make_base_ctx(root, "") }
    }

    fn ctx_with_lock(pm: PackageManager, lock_name: &str) -> (CiContext, TempDir) {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join(lock_name);
        std::fs::write(&lock_path, "").unwrap();
        let ctx = make_ctx(pm, Some(lock_path), dir.path());
        (ctx, dir)
    }

    // ── Happy-path per package manager ────────────────────────────────────────

    #[test]
    fn npm_cache() {
        let (ctx, _dir) = ctx_with_lock(PackageManager::Npm, "package-lock.json");
        let cfg = resolve_cache(&ctx).unwrap();
        assert_eq!(cfg.paths, vec!["~/.npm"]);
        assert!(cfg.key.contains("package-lock.json"));
        assert_eq!(cfg.restore_keys, vec!["npm-${{ runner.os }}-"]);
    }

    #[test]
    fn yarn_cache() {
        let (ctx, _dir) = ctx_with_lock(PackageManager::Yarn, "yarn.lock");
        let cfg = resolve_cache(&ctx).unwrap();
        assert!(cfg.paths.contains(&".yarn/cache".to_string()));
        assert!(cfg.key.contains("yarn.lock"));
    }

    #[test]
    fn pnpm_cache() {
        let (ctx, _dir) = ctx_with_lock(PackageManager::Pnpm, "pnpm-lock.yaml");
        let cfg = resolve_cache(&ctx).unwrap();
        assert_eq!(cfg.paths, vec!["~/.pnpm-store"]);
        assert!(cfg.key.contains("pnpm-lock.yaml"));
    }

    #[test]
    fn cargo_cache_has_target_dir() {
        let (ctx, _dir) = ctx_with_lock(PackageManager::Cargo, "Cargo.lock");
        let cfg = resolve_cache(&ctx).unwrap();
        assert!(cfg.paths.contains(&"target/".to_string()));
        assert!(cfg.key.contains("Cargo.lock"));
    }

    #[test]
    fn go_cache_has_build_cache() {
        let (ctx, _dir) = ctx_with_lock(PackageManager::GoMod, "go.sum");
        let cfg = resolve_cache(&ctx).unwrap();
        assert!(cfg.paths.contains(&"~/.cache/go-build".to_string()));
        assert!(cfg.key.contains("go.sum"));
    }

    #[test]
    fn poetry_cache() {
        let (ctx, _dir) = ctx_with_lock(PackageManager::Poetry, "poetry.lock");
        let cfg = resolve_cache(&ctx).unwrap();
        assert_eq!(cfg.paths, vec!["~/.cache/pypoetry"]);
        assert!(cfg.key.contains("poetry.lock"));
    }

    #[test]
    fn maven_cache() {
        let (ctx, _dir) = ctx_with_lock(PackageManager::Maven, "pom.xml");
        let cfg = resolve_cache(&ctx).unwrap();
        assert_eq!(cfg.paths, vec!["~/.m2/repository"]);
        assert!(cfg.key.contains("pom.xml"));
    }

    #[test]
    fn gradle_cache_includes_wrapper() {
        let (ctx, _dir) = ctx_with_lock(PackageManager::Gradle, "build.gradle");
        let cfg = resolve_cache(&ctx).unwrap();
        assert!(cfg.paths.contains(&"~/.gradle/wrapper".to_string()));
    }

    // ── Skip-cache cases ──────────────────────────────────────────────────────

    #[test]
    fn no_lock_file_returns_none() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(PackageManager::Npm, None, dir.path());
        assert!(resolve_cache(&ctx).is_none());
    }

    #[test]
    fn unknown_pm_returns_none() {
        let dir = TempDir::new().unwrap();
        // Even with a lock_file path set, Unknown PM should return None
        let lock = dir.path().join("some.lock");
        std::fs::write(&lock, "").unwrap();
        let ctx = make_ctx(PackageManager::Unknown, Some(lock), dir.path());
        assert!(resolve_cache(&ctx).is_none());
    }
}
