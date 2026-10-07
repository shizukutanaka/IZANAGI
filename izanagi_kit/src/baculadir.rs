//! Bacula/Bareos `*-dir.conf`/`*.conf` 検出モジュール。
//!
//! Bacula の設定は `ResourceType { ... }` ブロック形式で、
//! リソース型は `Director`/`Catalog`/`Storage`/`FileSet`/`Client`/
//! `Job`/`JobDefs`/`Schedule`/`Pool`/`Console`/`Counter`/`Messages`/
//! `Device`/`Autochanger`/`Profile`/`FileDaemon`/`SD`/`FD`/`UserId`/
//! `Cloud`/`Accurate`/`Base`/`Exclude`/`Include`/`Options`、
//! ブロック内は `Name = ...`/`Working Directory = ...`/`Address = ...`/
//! `Password = ...`/`Maximum Concurrent Jobs = ...` 等の
//! `key = value` 行、`@file` インクルードで構成される。
//!
//! ```
//! let b = b"Director {\n\
//!           Name = bacula-dir\n\
//!           DIRport = 9101\n\
//!           WorkingDirectory = \"/var/lib/bacula\"\n\
//!           PidDirectory = \"/run/bacula\"\n\
//!           Maximum Concurrent Jobs = 20\n\
//!           Password = \"secret\"\n\
//!           }\n\
//!           JobDefs {\n\
//!           Name = \"DefaultJob\"\n\
//!           Type = Backup\n\
//!           Schedule = \"WeeklyCycle\"\n\
//!           }\n";
//! let c = izanagi_kit::baculadir::parse(b);
//! assert!(izanagi_kit::baculadir::detect(b));
//! assert_eq!(c.resources, 2);
//! ```

const RESOURCES: &[&str] = &[
    "Autochanger",
    "Catalog",
    "Client",
    "Cloud",
    "Console",
    "Counter",
    "Device",
    "Director",
    "FileDaemon",
    "FileSet",
    "Job",
    "JobDefs",
    "Messages",
    "Pool",
    "Profile",
    "Schedule",
    "Storage",
    "UserId",
];

fn is_resource(t: &str) -> bool {
    if let Some(head) = t.strip_suffix('{').map(|h| h.trim_end()) {
        RESOURCES.contains(&head)
    } else {
        RESOURCES.contains(&t)
    }
}

fn is_kv(t: &str) -> bool {
    if let Some((k, v)) = t.split_once('=') {
        !k.trim().is_empty()
            && !v.trim().is_empty()
            && k.trim()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == ' ')
    } else {
        false
    }
}

/// `b` が Bacula 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut res = 0usize;
    let mut kvs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_resource(tr) {
            res += 1;
        } else if is_kv(tr) {
            kvs += 1;
        }
    }
    (res >= 1 && kvs >= 2) || res >= 2
}

/// Bacula 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct BaculaDir {
    /// リソースブロック数。
    pub resources: usize,
    /// `key = value` 行数。
    pub kvs: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を Bacula 設定として統計する。
pub fn parse(b: &[u8]) -> BaculaDir {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = BaculaDir::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_resource(tr) {
            c.resources += 1;
        } else if is_kv(tr) {
            c.kvs += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"Director {\nName = d\nWorkingDirectory = /w\n}\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.resources, 1);
    }

    #[test]
    fn detects_multi() {
        let b = b"Job {\n}\nPool {\n}\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"Director {\n}\n"));
        assert!(!detect(b"Director {\nName = x\n}\n"));
        assert!(!detect(b"[section]\nkey = x\nfoo = y\n"));
        assert!(!detect(b"key=value\nfoo=bar\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.resources, 0);
    }
}
