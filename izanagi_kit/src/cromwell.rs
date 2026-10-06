//! `cromwell.conf` 検出モジュール。
//!
//! Cromwell の HOCON 設定は `backend`/`call_caching`/`system.`/
//! `database`/`workflow-options`/`engine`/`services`/`docker`/
//! `webservice` 等のブロック・ドットキーで構成される。
//!
//! ```
//! let b = br#"include required(classpath("application"))
//! system {
//!   job-cache.enabled = true
//! }
//! backend {
//!   default = "Local"
//!   providers {
//!     Local {
//!       actor-factory = "cromwell.backend.impl.sfs.config.ConfigBackendLifecycleActorFactory"
//!     }
//!   }
//! }
//! call_caching.enabled = true
//! "#;
//! let c = izanagi_kit::cromwell::parse(b);
//! assert!(izanagi_kit::cromwell::detect(b));
//! assert_eq!(c.blocks, 3);
//! ```

const BLOCKS: &[&str] = &[
    "backend",
    "call_caching",
    "database",
    "docker",
    "engine",
    "filesystems",
    "keys",
    "services",
    "system",
    "webservice",
    "workflow-options",
    "workflow-options.",
];

fn is_comment(t: &str) -> bool {
    t.starts_with("//") || t.starts_with('#')
}

fn block_line(t: &str) -> bool {
    BLOCKS.iter().any(|b| {
        t == *b
            || (t.starts_with(&format!("{b} ")) || t.starts_with(&format!("{b}{{")))
                && t.contains('{')
    }) || t == "include required"
        || t.starts_with("include required(")
}

fn dotted_key(t: &str) -> bool {
    let Some(eq) = t.find('=') else {
        return false;
    };
    let key = t[..eq].trim();
    key.starts_with("system.")
        || key.starts_with("call_caching.")
        || key.starts_with("backend.")
        || key.starts_with("workflow-options.")
        || key.starts_with("database.")
        || key.starts_with("engine.")
}

/// `b` が cromwell.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut blocks = 0usize;
    let mut dotted = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if block_line(tr) {
            blocks += 1;
        } else if dotted_key(tr) {
            dotted += 1;
        }
    }
    blocks >= 2 || (blocks >= 1 && dotted >= 1) || dotted >= 2
}

/// cromwell.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct CromwellConf {
    /// `backend {`/`system {` 等ブロック行数。
    pub blocks: usize,
    /// `system.*`/`call_caching.*` 等ドットキー行数。
    pub dotted_keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を cromwell.conf として統計する。
pub fn parse(b: &[u8]) -> CromwellConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = CromwellConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if block_line(tr) {
            c.blocks += 1;
        } else if dotted_key(tr) {
            c.dotted_keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"backend {
  default = "Local"
}
system {
  max-concurrent-workflows = 5
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.blocks, 2);
    }

    #[test]
    fn detects_dotted() {
        let b = br#"system.io-rate = 100
system.new-workflow-poll-rate = 5
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.dotted_keys, 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"backend {\n}\n"));
        assert!(!detect(b"foo = 1\nbar = 2\nbaz = 3\n"));
        assert!(!detect(b"system.x = 1\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.blocks, 0);
    }
}
