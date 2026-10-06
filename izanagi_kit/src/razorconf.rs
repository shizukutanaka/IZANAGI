//! `razor-agent.conf` (Vipul's Razor) 検出モジュール。
//!
//! Razor の設定は `key = value` 形式で、`razorhome`/`listfile`/
//! `logfile`/`debuglevel`/`razordiscovery`/`discover`/`logic_method`/
//! `whitelist`/`nominn`/`min`/`se`/`cf`/`see_ping`/`ignorelist`/
//! `use_engines`/`engine_*_max_cf`/`engine_*_max_policy`/
//! `engine_4`/`engine_8`/`engine_16`/`engine_32`/`engine_64`/
//! `so_keepalive`/`time`/`rulebase`/`policy`/`sleeptime`/`count`/
//! `razorzone`/`serv`/`server`/`port`/`version`/`conf`/`eh`
//! 等のキーで構成される。
//!
//! ```
//! let b = b"razorhome = /etc/razor\n\
//!           logfile = /var/log/razor-agent.log\n\
//!           debuglevel = 3\n\
//!           logic_method = 4\n\
//!           razordiscovery = discovery.razor.cloudmark.com\n";
//! let c = izanagi_kit::razorconf::parse(b);
//! assert!(izanagi_kit::razorconf::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "cf",
    "conf",
    "count",
    "debuglevel",
    "discover",
    "eh",
    "ignorelist",
    "listfile",
    "logfile",
    "logic_method",
    "min",
    "nominn",
    "policy",
    "port",
    "razordiscovery",
    "razorhome",
    "razorzone",
    "rulebase",
    "se",
    "see_ping",
    "serv",
    "server",
    "sleeptime",
    "so_keepalive",
    "time",
    "use_engines",
    "version",
    "whitelist",
];

fn is_key(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
        || (k.starts_with("engine_") && (k.ends_with("_max_cf") || k.ends_with("_max_policy")))
        || matches!(
            k,
            "engine_4" | "engine_8" | "engine_16" | "engine_32" | "engine_64"
        )
}

/// `b` が razor 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if tr.contains('=') && is_key(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// razor 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct RazorConf {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を razor 設定として統計する。
pub fn parse(b: &[u8]) -> RazorConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = RazorConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if tr.contains('=') && is_key(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"razorhome = /etc/razor\nlogfile = /x\ndebuglevel = 3\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn engine_keys() {
        let b = b"use_engines = 4,8\nengine_4_max_cf = 90\nengine_8_max_cf = 80\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"razorhome = x\nlistfile = y\n"));
        assert!(!detect(b"key = value\nfoo = bar\nbaz = quux\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
