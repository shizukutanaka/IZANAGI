//! cargo-make `Makefile.toml` 形式の検出と構造カウント。
//!
//! `[tasks.*]`/`[env]`/`[config]`/`[plugins]`/`[workspace]`/`[core]`
//! テーブルと `command`/`args`/`script`/`dependencies`/`run_task`/
//! `install_crate`/`category`/`condition`/`extend` 等のタスク定義キーを
//! 識別する。
//!
//! ```
//! let b = b"[env]\nBUILD_MODE = \"debug\"\n[config]\ndefault_to_workspace = false\n[tasks.build]\ncommand = \"cargo\"\nargs = [\"build\"]\ncategory = \"Build\"\n[tasks.test]\ndependencies = [\"build\"]\nrun_task = \"cargo-test\"\n";
//! assert!(izanagi_kit::cargomake::detect(b));
//! let c = izanagi_kit::cargomake::CargoMake::parse(b).unwrap();
//! assert_eq!(c.tables, 4);
//! ```

/// Parsed Makefile.toml summary.
#[derive(Debug, Clone)]
pub struct CargoMake {
    /// Recognized tables (`[tasks.*]`/`[env]`/`[config]`/...).
    pub tables: usize,
    /// Recognized option keys.
    pub keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// cargo-make table names (`tasks.*` matches by prefix).
const TABLES: &[&str] = &[
    "tasks",
    "env",
    "config",
    "plugins",
    "workspace",
    "core",
    "env_files",
    "cli",
    "logger",
    "run",
];

/// cargo-make option keys.
const KEYS: &[&str] = &[
    "alias",
    "args",
    "category",
    "clear",
    "command",
    "condition",
    "condition_script",
    "default_task",
    "default_to_workspace",
    "dependencies",
    "deprecated",
    "description",
    "disabled",
    "docs_category",
    "env",
    "env_files",
    "env_script",
    "extend",
    "ignore_errors",
    "install_crate",
    "install_crate_args",
    "install_script",
    "linux",
    "linux_alias",
    "mac",
    "mac_alias",
    "private",
    "run_task",
    "script",
    "script_runner",
    "skip_core_tasks",
    "skip_git_env_info",
    "init_task",
    "end_task",
    "on_error_task",
    "watch",
    "windows",
    "windows_alias",
    "workspace",
    "toolchain",
];

fn key_present(t: &str, k: &str) -> bool {
    t.lines().any(|l| {
        let tr = l.trim();
        !tr.starts_with('#') && tr.split('=').next().is_some_and(|p| p.trim() == k)
    })
}

fn table_hit(t: &str) -> usize {
    t.lines()
        .filter_map(|l| {
            let tr = l.trim();
            if tr.starts_with('[') && tr.ends_with(']') && !tr.starts_with("[[") {
                Some(tr[1..tr.len() - 1].trim())
            } else {
                None
            }
        })
        .filter(|n| {
            TABLES.iter().any(|p| {
                *n == *p
                    || (n.len() > p.len() && n.starts_with(*p) && n.as_bytes()[p.len()] == b'.')
            })
        })
        .count()
}

/// Detect a cargo-make config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let n = KEYS.iter().filter(|k| key_present(t, k)).count();
    table_hit(t) >= 1 || n >= 4
}

impl CargoMake {
    /// Count categories. Returns `None` when the input does not look like
    /// a cargo-make config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            tables: table_hit(t),
            keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
                if let Some(k) = tr.split('=').next() {
                    if KEYS.contains(&k.trim()) {
                        c.keys += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`CargoMake::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<CargoMake> {
    CargoMake::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# make tasks\n[env]\nBUILD_MODE = \"debug\"\nCARGO_MAKE_CI = \"true\"\n[config]\ndefault_to_workspace = false\nskip_core_tasks = true\n[tasks.build]\ncommand = \"cargo\"\nargs = [\"build\", \"--workspace\"]\ncategory = \"Build\"\ndescription = \"compile all\"\n[tasks.test]\ndependencies = [\"build\"]\nrun_task = \"cargo-test\"\ntoolchain = \"nightly\"\n[tasks.docs]\nscript = \"echo docs\"\nworkspace = false\n";
        assert!(detect(b));
        let c = CargoMake::parse(b).unwrap();
        assert_eq!(c.tables, 5);
        assert!(c.keys >= 11);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_tasks_only() {
        assert!(detect(
            b"[tasks.fmt]\ncommand = \"cargo\"\nargs = [\"fmt\"]\n"
        ));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[package]\nname = \"x\"\n"));
        assert!(!detect(b"CC=gcc\nall: build\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(CargoMake::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"[tasks.build]");
        assert!(!detect(&b));
    }
}
