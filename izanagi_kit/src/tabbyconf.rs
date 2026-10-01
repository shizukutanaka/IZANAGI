//! Tabby (旧 Terminus) `config.yaml` パーサ。
//!
//! `hotkeys:`/`terminal:`/`profiles:`/`profileDefaults:`/`appearance:`/`vault:` 等
//! 既知トップキーとネスト構造を計数する。
//!
//! ```
//! use izanagi_kit::tabbyconf;
//! let conf = b"hotkeys:\n  copy: ctrl-shift-c\nterminal:\n  fontSize: 14\nprofiles: []\n";
//! assert!(tabbyconf::detect(conf));
//! let c = tabbyconf::parse(conf).unwrap();
//! assert_eq!(c.known_tops, 3);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// col0 `key:` トップキー数。
    pub top_keys: usize,
    /// 既知トップキー数。
    pub known_tops: usize,
    /// インデント `key:` 行数。
    pub nested_keys: usize,
    /// `-` リスト項目数。
    pub list_items: usize,
}

const KNOWN_TOPS: &[&str] = &[
    "hotkeys",
    "terminal",
    "profiles",
    "profileDefaults",
    "profileGroups",
    "appearance",
    "vault",
    "configSync",
    "ssh",
    "telnet",
    "serial",
    "clickableLinks",
    "accessibility",
    "hacks",
    "updater",
    "commandline",
    "shell",
    "proxy",
    "sync",
    "remoteAuth",
    "portable",
    "scrollback",
    "badge",
    "spinner",
    "debug",
    "welcome",
    "window",
    "pluginBlacklist",
    "lastTab",
];

/// `config.yaml` (Tabby) らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => c.known_tops >= 2 || (c.known_tops >= 1 && c.nested_keys >= 4),
        None => false,
    }
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        top_keys: 0,
        known_tops: 0,
        nested_keys: 0,
        list_items: 0,
    };
    for l in s.lines() {
        let t = l.trim_end();
        let tr = t.trim_start();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if tr.starts_with("- ") || tr == "-" {
            c.list_items += 1;
            continue;
        }
        let Some(colon) = tr.find(':') else {
            continue;
        };
        let key = tr[..colon].trim();
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            continue;
        }
        if t.starts_with(tr) {
            c.top_keys += 1;
            if KNOWN_TOPS.contains(&key) {
                c.known_tops += 1;
            }
        } else {
            c.nested_keys += 1;
        }
    }
    (c.top_keys > 0 || c.nested_keys > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"hotkeys:\n  copy: ctrl-shift-c\n  paste: ctrl-shift-v\nterminal:\n  fontSize: 14\n  ligatures: true\nprofiles: []\nappearance:\n  theme: Hyper\n";

    #[test]
    fn detects_tabby() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.top_keys, 4);
        assert_eq!(c.known_tops, 4);
        assert_eq!(c.nested_keys, 5);
    }

    #[test]
    fn rejects_other_yaml() {
        let y = b"name: x\nservices:\n  web:\n    image: nginx\n";
        assert!(!detect(y));
    }
}
