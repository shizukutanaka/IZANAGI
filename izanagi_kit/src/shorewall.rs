//! Shorewall 設定ファイル (`/etc/shorewall/*`: rules/policy/zones/interfaces/
//! masq/nat/routestopped/params 等) の解析。
//!
//! `ACTION SOURCE DEST PROTO DPORT` ルール行、`<ゾーン>` 定義行、
//! `source dest policy log` ポリシー行の形を持つ設定を検出し、
//! アクション・ゾーン・ポリシーの数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::shorewall;
//!
//! let text = br#"#ACTION SOURCE DEST PROTO DPORT
//! ACCEPT net $FW tcp 22
//! DNAT net loc:10.0.0.5 tcp 80
//! DROP net all
//! "#;
//!
//! assert!(shorewall::detect(text));
//! let c = shorewall::parse(text).unwrap();
//! assert_eq!(c.entries, 3);
//! assert_eq!(c.accepts, 1);
//! ```

/// shorewall rules/policy で使われる既知アクション。
const KNOWN_ACTIONS: &[&str] = &[
    "ACCEPT",
    "ACCEPT+",
    "ACCEPT!",
    "NONET",
    "DROP",
    "DROP!",
    "REJECT",
    "REJECT+",
    "REJECT!",
    "DNAT",
    "DNAT-",
    "REDIRECT",
    "MASQUERADE",
    "SNAT",
    "SAME",
    "NOTRACK",
    "CONTINUE",
    "CONTINUE!",
    "RETURN",
    "QUEUE",
    "NFQUEUE",
    "NFLOG",
    "ULOG",
    "LOG",
    "COUNT",
    "HL",
    "MARK",
    "CONNMARK",
    "IMQ",
    "IPMARK",
    "TARPIT",
    "BLACKLIST",
    "WHITELIST",
    "RESTORE",
    "SAVE",
    "AUDIT",
    "SECTION",
    "COMMENT",
    "INLINE",
    "POLICY",
    "OPTION",
    "ACCOUNTING",
    "MARKCAP",
    "BYPASS",
    "DISALLOW",
];

/// shorewall ゾーン名として典型な接尾辞。
const ZONE_WORDS: &[&str] = &[
    "fw",
    "net",
    "loc",
    "dmz",
    "vpn",
    "all",
    "shorewall",
    "ipv4",
    "ipv6",
];

/// shorewall 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// データ行総数 (コメント・ヘッダ・空行を除く)。
    pub entries: usize,
    /// `ACCEPT*` アクション行数。
    pub accepts: usize,
    /// `DROP`/`REJECT`/`TARPIT`/`BLACKLIST`/`DISALLOW` 系アクション行数。
    pub drops: usize,
    /// `DNAT`/`REDIRECT`/`SNAT`/`MASQUERADE`/`SAME`/`NOTRACK` 系行数。
    pub nat_actions: usize,
    /// `LOG`/`NFLOG`/`ULOG`/`COUNT`/`AUDIT`/`MARK`/`HL`/`CONNMARK`/`IPMARK`/`QUEUE`/`NFQUEUE`/`IMQ` 系行数。
    pub misc_actions: usize,
    /// `SECTION`/`COMMENT`/`INLINE`/`OPTION`/`POLICY`/`ACCOUNTING`/`?FORMAT`/`format` 系行数。
    pub special_lines: usize,
}

/// `b` が shorewall 設定ファイルらしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    let actions = c.accepts + c.drops + c.nat_actions + c.misc_actions;
    actions >= 2 || (actions >= 1 && c.special_lines >= 1)
}

/// shorewall 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        entries: 0,
        accepts: 0,
        drops: 0,
        nat_actions: 0,
        misc_actions: 0,
        special_lines: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') || line.starts_with('?') {
            continue;
        }
        let mut it = line.split_whitespace();
        let Some(first) = it.next() else {
            continue;
        };
        if first.starts_with('{') || first.starts_with('&') || first.starts_with('?') {
            continue;
        }
        let upper = first.to_ascii_uppercase();
        if upper.starts_with("?FORMAT") || upper == "FORMAT" {
            counts.special_lines += 1;
            saw_any = true;
            continue;
        }
        if KNOWN_ACTIONS.contains(&upper.as_str()) {
            counts.entries += 1;
            saw_any = true;
            match upper.as_str() {
                "ACCEPT" | "ACCEPT+" | "ACCEPT!" => counts.accepts += 1,
                "DROP" | "DROP!" | "REJECT" | "REJECT+" | "REJECT!" | "TARPIT" | "BLACKLIST"
                | "DISALLOW" | "NONET" => counts.drops += 1,
                "DNAT" | "DNAT-" | "REDIRECT" | "SNAT" | "MASQUERADE" | "SAME" | "NOTRACK" => {
                    counts.nat_actions += 1
                }
                "SECTION" | "COMMENT" | "INLINE" | "OPTION" | "POLICY" | "ACCOUNTING"
                | "MARKCAP" | "BYPASS" => {
                    counts.special_lines += 1;
                }
                _ => counts.misc_actions += 1,
            }
            continue;
        }
        // ゾーン/ポリシー行 (小文字先頭 + 2〜4欄)。
        if first
            .bytes()
            .all(|c| c.is_ascii_lowercase() || matches!(c, b'-' | b'_'))
        {
            let rest: Vec<&str> = it.collect();
            if (1..=3).contains(&rest.len())
                && rest.iter().all(|w| {
                    !w.is_empty()
                        && w.bytes().all(|c| {
                            c.is_ascii_alphanumeric()
                                || matches!(
                                    c,
                                    b'-' | b'_' | b':' | b'@' | b'!' | b'/' | b'.' | b'$' | b'~'
                                )
                        })
                })
            {
                counts.entries += 1;
                if ZONE_WORDS.contains(&first) || first.contains(':') {
                    counts.special_lines += 1;
                }
                saw_any = true;
            }
            continue;
        }
    }
    if !saw_any {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"#ACTION SOURCE DEST PROTO DPORT
ACCEPT net $FW tcp 22
ACCEPT net $FW tcp 80,443
DNAT net loc:10.0.0.5 tcp 8080
REDIRECT loc 3128 tcp 80 -
DROP net all
REJECT loc net tcp 25
LOG:DROP net all
"#;

    #[test]
    fn detects_shorewall() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 6);
        assert_eq!(c.accepts, 2);
        assert_eq!(c.drops, 2);
        assert_eq!(c.nat_actions, 2);
        assert_eq!(c.special_lines, 0);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"FOO bar baz"));
    }
}
