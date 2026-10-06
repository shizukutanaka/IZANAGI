//! Limnoria/Supybot 設定ファイル検出モジュール。
//!
//! Supybot/Limnoria のレジストリファイルは `supybot.<path>: <value>`
//! 形式のドット区切りキー行で構成される。
//!
//! ```
//! let b = br#"supybot.nick: LamestBot
//! supybot.user: lamestbot
//! supybot.ident: limnoria
//! supybot.networks.freenode: True
//! supybot.networks.freenode.servers: irc.libera.chat:6697
//! supybot.plugins.ChannelLogger: True
//! "#;
//! let c = izanagi_kit::limnoria::parse(b);
//! assert!(izanagi_kit::limnoria::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with("###") || t.starts_with("//")
}

fn key_line(t: &str) -> bool {
    let Some(rest) = t.strip_prefix("supybot.") else {
        return false;
    };
    // must be dotted path ending with ':'
    rest.contains(':') && rest.chars().next().is_some_and(|c| c.is_alphanumeric())
}

/// `b` が Limnoria/Supybot 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if key_line(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// Limnoria 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct LimnoriaConf {
    /// `supybot.*` キー行数。
    pub keys: usize,
    /// `supybot.plugins.*` キー行数。
    pub plugin_keys: usize,
    /// `supybot.networks.*` キー行数。
    pub network_keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を Limnoria/Supybot 設定として統計する。
pub fn parse(b: &[u8]) -> LimnoriaConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = LimnoriaConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if key_line(tr) {
            c.keys += 1;
            if tr.starts_with("supybot.plugins.") {
                c.plugin_keys += 1;
            } else if tr.starts_with("supybot.networks.") {
                c.network_keys += 1;
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"supybot.nick: LamestBot
supybot.user: lamestbot
supybot.ident: limnoria
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_networks() {
        let b = br#"supybot.networks.freenode: True
supybot.networks.freenode.servers: irc.libera.chat:6697
supybot.networks.efnet: False
supybot.plugins.ChannelLogger: True
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.network_keys, 3);
        assert_eq!(c.plugin_keys, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"supybot.nick: x\n"));
        assert!(!detect(b"key: value\nother: value2\nthird: v3\n"));
        assert!(!detect(b"supybot\nfoo\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
