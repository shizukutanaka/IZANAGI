//! cross-rs `Cross.toml` 形式の検出と構造カウント。
//!
//! `[build]`/`[build.env]`/`[build.dockerfile]`/`[target.*]` テーブルと
//! `xargo`/`docker-in-docker`/`pre-build`/`image`/`runner`/`engine`/
//! `default-target`/`volumes`/`passthrough` 等のクロスコンパイル
//! コンテナ設定キーを識別する。
//!
//! ```
//! let b = b"[build]\ndefault-target = \"aarch64-unknown-linux-gnu\"\nxargo = false\n[build.env]\npassthrough = [\"RUST_BACKTRACE\"]\n[target.aarch64-unknown-linux-gnu]\nimage = \"cross:latest\"\npre-build = [\"apt-get update\"]\nrunner = \"qemu-aarch64\"\n";
//! assert!(izanagi_kit::crossconf::detect(b));
//! let c = izanagi_kit::crossconf::CrossConf::parse(b).unwrap();
//! assert_eq!(c.tables, 3);
//! ```

/// Parsed Cross.toml summary.
#[derive(Debug, Clone)]
pub struct CrossConf {
    /// Recognized tables (`[build.*]`/`[target.*]`).
    pub tables: usize,
    /// Recognized option keys.
    pub keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// cross table names (`target.*`/`build.*` match by prefix).
const TABLES: &[&str] = &["build", "target"];

/// cross option keys.
const KEYS: &[&str] = &[
    "context",
    "default-target",
    "docker-in-docker",
    "dockerfile",
    "engine",
    "env",
    "environment",
    "image",
    "passthrough",
    "pre-build",
    "pull",
    "remote",
    "runner",
    "target",
    "toolchain",
    "volumes",
    "xargo",
];

/// Strong cross markers.
const STRONG: &[&str] = &[
    "xargo",
    "docker-in-docker",
    "pre-build",
    "runner",
    "default-target",
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

/// Detect a Cross.toml file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let tables = table_hit(t);
    if tables >= 1 && STRONG.iter().any(|k| key_present(t, k)) {
        return true;
    }
    tables >= 2 || KEYS.iter().filter(|k| key_present(t, k)).count() >= 4
}

impl CrossConf {
    /// Count categories. Returns `None` when the input does not look like
    /// a Cross.toml file.
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

/// Convenience wrapper around [`CrossConf::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<CrossConf> {
    CrossConf::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# cross setup\n[build]\ndefault-target = \"aarch64-unknown-linux-gnu\"\nxargo = false\ndocker-in-docker = false\n[build.env]\npassthrough = [\"RUST_BACKTRACE\", \"QEMU_STRACE\"]\nvolumes = [\"CARGO_CACHE\"]\n[build.dockerfile]\nfile = \"Dockerfile.cross\"\ncontext = \"./docker\"\n[target.aarch64-unknown-linux-gnu]\nimage = \"cross:stable\"\npre-build = [\"dpkg --add-architecture arm64\", \"apt-get update\"]\nrunner = \"qemu-aarch64\"\n";
        assert!(detect(b));
        let c = CrossConf::parse(b).unwrap();
        assert_eq!(c.tables, 4);
        assert!(c.keys >= 9);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_target_table_with_strong_key() {
        assert!(detect(
            b"[target.armv7-unknown-linux-gnueabihf]\npre-build = [\"apt-get update\"]\n"
        ));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[package]\nname = \"x\"\ntarget = \"y\"\n"));
        assert!(!detect(b"[build]\nfoo = 1\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(CrossConf::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"[target.x]");
        assert!(!detect(&b));
    }
}
