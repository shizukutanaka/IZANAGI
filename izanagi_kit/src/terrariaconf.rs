//! Terraria `config.txt`/`serverconfig.txt` 検出モジュール。
//!
//! Terraria サーバ設定は平坦な `key=value` 形式で、`world=`/
//! `autocreate=`/`seed=`/`worldname=`/`difficulty=`/`maxplayers=`/
//! `port=`/`password=`/`motd=`/`worldpath=`/`banlist=`/`secure=`/
//! `language=`/`upnp=`/`npcstream=`/`priority=`/`journeypermission_*`/
//! `defaults-`?/`server-api-whitelist`?/`experimental-features`?
//! 等のキーで構成される(値は行ごと、`-`/`=` 区切りも `-key=value` の
//! CLI 直移植形式)。
//!
//! ```
//! let b = b"world=C:\\world.wld\n\
//!           autocreate=3\n\
//!           difficulty=0\n\
//!           maxplayers=8\n\
//!           port=7777\n\
//!           worldname=MyWorld\n";
//! let c = izanagi_kit::terrariaconf::parse(b);
//! assert!(izanagi_kit::terrariaconf::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

const KEYS: &[&str] = &[
    "autocreate",
    "banlist",
    "difficulty",
    "language",
    "maxplayers",
    "motd",
    "npcstream",
    "password",
    "port",
    "priority",
    "seed",
    "secure",
    "upnp",
    "world",
    "worldname",
    "worldpath",
];

fn is_key(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k) || k.starts_with("journeypermission_")
}

/// `b` が terraria config.txt に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim().trim_start_matches('-');
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if tr.contains('=') && is_key(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// terraria config.txt の統計。
#[derive(Debug, Default, Clone)]
pub struct TerrariaConf {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を terraria config.txt として統計する。
pub fn parse(b: &[u8]) -> TerrariaConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = TerrariaConf::default();
    for l in t.lines() {
        let tr = l.trim().trim_start_matches('-');
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
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
        let b = b"world=x\nautocreate=3\nmaxplayers=8\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn journeypermission() {
        let b = b"port=7777\njourneypermission_godmode=1\njourneypermission_time_setfrozen=1\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"world=x\nautocreate=3\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
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
