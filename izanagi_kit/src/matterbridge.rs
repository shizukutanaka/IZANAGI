//! Matterbridge `matterbridge.toml` — `[proto.name]` サービステーブルと
//! `[[gateway]]`/`[[gateway.inout]]` ゲートウェイ配列テーブル。
//!
//! ```
//! let cfg = b"[irc.libera]\nServer = \"irc.libera.chat:6697\"\nNick = \"bridgebot\"\n[discord.test]\nToken = \"XXX\"\n[[gateway]]\nname = \"gw1\"\n[[gateway.inout]]\naccount = \"irc.libera\"\nchannel = \"#iz\"\n";
//! assert!(izanagi_kit::matterbridge::detect(cfg));
//! let c = izanagi_kit::matterbridge::parse(cfg).unwrap();
//! assert_eq!(c.protocol_tables, 2);
//! assert_eq!(c.gateway_tables, 2);
//! ```

/// matterbridge がブリッジするプロトコルのテーブル接頭辞。
const PROTOCOLS: &[&str] = &[
    "irc.",
    "discord.",
    "slack.",
    "matrix.",
    "xmpp.",
    "telegram.",
    "mattermost.",
    "mumble.",
    "whatsapp.",
    "steam.",
    "rocketchat.",
    "gitter.",
    "keybase.",
    "zulip.",
    "teams.",
    "api.",
    "sshchat.",
    "mastodon.",
    "nctalk.",
    "ms teams.",
];

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// `[name]` テーブル数 (`[[` 除く)。
    pub tables: usize,
    /// `[[name]]` 配列テーブル数。
    pub arrays: usize,
    /// プロトコル接頭辞のテーブル数。
    pub protocol_tables: usize,
    /// `gateway` 系テーブル/配列数。
    pub gateway_tables: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// Matterbridge TOML らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    (c.gateway_tables >= 1 && c.protocol_tables >= 1) || c.protocol_tables >= 2
}

/// 行を走査して集計する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        lines: 0,
        tables: 0,
        arrays: 0,
        protocol_tables: 0,
        gateway_tables: 0,
        entries: 0,
        comments: 0,
    };
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if let Some(inner) = t.strip_prefix("[[").and_then(|x| x.strip_suffix("]]")) {
            let name = inner.trim();
            c.arrays += 1;
            if name == "gateway" || name.starts_with("gateway.") {
                c.gateway_tables += 1;
            }
            continue;
        }
        if let Some(inner) = t.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
            let name = inner.trim().trim_matches('"');
            if name.is_empty() {
                continue;
            }
            c.tables += 1;
            if name == "gateway" || name.starts_with("gateway.") {
                c.gateway_tables += 1;
            }
            let lower = name.to_ascii_lowercase();
            if PROTOCOLS.iter().any(|p| lower.starts_with(p)) {
                c.protocol_tables += 1;
            }
            continue;
        }
        if let Some((key, _)) = t.split_once('=') {
            let key = key.trim();
            if !key.is_empty()
                && key
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'.' || ch == b'-')
            {
                c.entries += 1;
            }
        }
    }
    if c.tables + c.arrays == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[irc.libera]\nServer = \"irc.libera.chat:6697\"\nNick = \"bridgebot\"\nRemoteNickFormat = \"[{PROTOCOL}] <{NICK}> \"\n[slack.team]\nToken = \"xoxb-1\"\n[[gateway]]\nname = \"main\"\n[[gateway.inout]]\naccount = \"irc.libera\"\nchannel = \"#iz\"\n[[gateway.inout]]\naccount = \"slack.team\"\nchannel = \"general\"\n";

    #[test]
    fn detects_matterbridge() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.tables, 2);
        assert_eq!(c.arrays, 3);
        assert_eq!(c.protocol_tables, 2);
        assert_eq!(c.gateway_tables, 3);
        assert_eq!(c.entries, 9);
    }

    #[test]
    fn rejects_plain_toml() {
        assert!(!detect(b"[package]\nname = \"app\"\nversion = \"1.0\"\n"));
        assert!(!detect(b"[[deps]]\nx = 1\n"));
    }
}
