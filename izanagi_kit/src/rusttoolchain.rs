//! `rust-toolchain.toml`/`rust-toolchain` 形式の検出と構造カウント。
//!
//! `[toolchain]` テーブル内の `channel`/`components`/`targets`/`profile`
//! キー、またはレガシー形式のチャンネル名単独行を識別する。
//!
//! ```
//! let b = b"[toolchain]\nchannel = \"nightly-2024-01-01\"\ncomponents = [\"rustfmt\", \"clippy\"]\ntargets = [\"wasm32-unknown-unknown\"]\nprofile = \"minimal\"\n";
//! assert!(izanagi_kit::rusttoolchain::detect(b));
//! let c = izanagi_kit::rusttoolchain::RustToolchain::parse(b).unwrap();
//! assert_eq!(c.keys, 4);
//! ```

/// Parsed rust-toolchain summary.
#[derive(Debug, Clone)]
pub struct RustToolchain {
    /// Recognized option keys (`channel`/`components`/`targets`/`profile`).
    pub keys: usize,
    /// `[toolchain]` table present.
    pub has_table: bool,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Toolchain keys.
const KEYS: &[&str] = &["channel", "components", "targets", "profile"];

fn key_present(t: &str, k: &str) -> bool {
    t.lines().any(|l| {
        let tr = l.trim();
        !tr.starts_with('#') && tr.split('=').next().is_some_and(|p| p.trim() == k)
    })
}

fn has_table(t: &str) -> bool {
    t.lines().any(|l| {
        let tr = l.trim();
        tr.starts_with('[') && tr.ends_with(']') && tr[1..tr.len() - 1].trim() == "toolchain"
    })
}

/// Detect a rust-toolchain file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if has_table(t) {
        return true;
    }
    let n = KEYS.iter().filter(|k| key_present(t, k)).count();
    (key_present(t, "channel") && n >= 2) || n >= 3
}

impl RustToolchain {
    /// Count categories. Returns `None` when the input does not look like
    /// a rust-toolchain file.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            has_table: has_table(t),
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

/// Convenience wrapper around [`RustToolchain::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<RustToolchain> {
    RustToolchain::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"[toolchain]\nchannel = \"1.78.0\"\ncomponents = [\"rustfmt\", \"clippy\", \"miri\"]\ntargets = [\"x86_64-unknown-linux-gnu\", \"wasm32-unknown-unknown\"]\nprofile = \"default\"\n";
        assert!(detect(b));
        let c = RustToolchain::parse(b).unwrap();
        assert_eq!(c.keys, 4);
        assert!(c.has_table);
        assert_eq!(c.assignments, 4);
    }

    #[test]
    fn detects_nightly_with_components_without_table() {
        let b = b"channel = \"nightly\"\ncomponents = [\"rust-analyzer\"]\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated_table() {
        assert!(!detect(b"[toolchain.other]\nx = 1\n"));
        assert!(!detect(b"[package]\nname = \"y\"\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(RustToolchain::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"[toolchain]");
        assert!(!detect(&b));
    }
}
