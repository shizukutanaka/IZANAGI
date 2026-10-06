//! bacon `bacon.toml` 形式の検出と構造カウント。
//!
//! `[jobs.*]`/`[keybindings]`/`[export]`/`[vigil]`/`[plugins]` テーブルと
//! `default_job`/`watch`/`summary`/`wrapping`/`command`/`need_stdout`/
//! `on_success`/`on_failure` 等のジョブ制御キーを識別する。
//!
//! ```
//! let b = b"default_job = \"check\"\n[jobs.check]\ncommand = [\"cargo\", \"check\", \"--color\", \"always\"]\nneed_stdout = false\nwatch = [\"src\"]\n[keybindings]\ni = \"job:check\"\n";
//! assert!(izanagi_kit::bacon::detect(b));
//! let c = izanagi_kit::bacon::Bacon::parse(b).unwrap();
//! assert_eq!(c.tables, 2);
//! ```

/// Parsed bacon.toml summary.
#[derive(Debug, Clone)]
pub struct Bacon {
    /// Recognized tables (`[jobs.*]`/`[keybindings]`/...).
    pub tables: usize,
    /// Recognized option keys.
    pub keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// bacon table names (`jobs.*` matches by prefix).
const TABLES: &[&str] = &[
    "jobs",
    "keybindings",
    "export",
    "vigil",
    "plugins",
    "keymap",
    "bacon",
    "summary",
    "wrapping",
    "search",
];

/// bacon option keys.
const KEYS: &[&str] = &[
    "analyze",
    "args",
    "command",
    "contract",
    "default_job",
    "enabled",
    "env",
    "expand",
    "expansion",
    "export",
    "focus",
    "grace_period",
    "help_line",
    "ignore",
    "ignore_changes",
    "ignore_lines",
    "init",
    "job",
    "jobs",
    "keybindings",
    "kill",
    "list_files",
    "need_stdout",
    "notify",
    "on_change_strategy",
    "on_failure",
    "on_success",
    "paths",
    "playground",
    "show",
    "summary",
    "toggle_levels",
    "watch",
    "wrapping",
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

/// Detect a bacon config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if table_hit(t) >= 1 {
        return true;
    }
    let n = KEYS.iter().filter(|k| key_present(t, k)).count();
    (key_present(t, "default_job") && n >= 2) || n >= 4
}

impl Bacon {
    /// Count categories. Returns `None` when the input does not look like
    /// a bacon config.
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

/// Convenience wrapper around [`Bacon::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<Bacon> {
    Bacon::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"default_job = \"check\"\nsummary = true\nwrapping = false\non_change_strategy = \"kill_then_spawn\"\n[jobs.check]\ncommand = [\"cargo\", \"check\", \"--color\", \"always\"]\nneed_stdout = false\nwatch = [\"src\"]\nenv.RUST_BACKTRACE = \"1\"\n[jobs.clippy]\ncommand = [\"cargo\", \"clippy\"]\nneed_stdout = false\n[keybindings]\ni = \"job:check\"\n";
        assert!(detect(b));
        let c = Bacon::parse(b).unwrap();
        assert_eq!(c.tables, 3);
        assert!(c.keys >= 9);
    }

    #[test]
    fn detects_jobs_only() {
        assert!(detect(
            b"[jobs.clippy]\ncommand = [\"cargo\", \"clippy\"]\n"
        ));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[package]\nname = \"x\"\nwatch = true\n"));
        assert!(!detect(b"command = [\"ls\"]\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(Bacon::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"[jobs.check]");
        assert!(!detect(&b));
    }
}
