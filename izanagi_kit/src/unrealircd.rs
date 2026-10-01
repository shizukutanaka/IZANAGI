//! UnrealIRCd `unrealircd.conf` — `blockname { stmt; }` / `blockname arg { }`
//! ブロック + `;` 終端ステートメント形式。
//!
//! ```
//! let cfg = b"me {\n        name \"irc.example.net\";\n        info \"ExampleNet\";\n        sid \"001\";\n}\nclass clients {\n        pingfreq 90;\n        maxclients 500;\n}\nlisten {\n        ip *;\n        port 6667;\n}\n";
//! assert!(izanagi_kit::unrealircd::detect(cfg));
//! let c = izanagi_kit::unrealircd::parse(cfg).unwrap();
//! assert_eq!(c.known_blocks, 3);
//! assert_eq!(c.statements, 7);
//! ```

/// UnrealIRCd の既知ブロック名。
const KNOWN_BLOCKS: &[&str] = &[
    "me",
    "admin",
    "class",
    "oper",
    "listen",
    "link",
    "allow",
    "ban",
    "set",
    "log",
    "alias",
    "tld",
    "deny",
    "vhost",
    "spamfilter",
    "badword",
    "except",
    "ulines",
    "webirc",
    "snomask",
    "cap",
    "drpass",
    "reputation",
    "geoip",
    "security-group",
    "plaintext-policy",
    "official-channels",
    "require",
    "cloak-keys",
    "mask-server",
    "restrict-channelmodes",
    "extended-ban",
    "trace",
    "proxy",
];

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// `name {` / `name arg {` ブロック頭数。
    pub blocks: usize,
    /// 既知名のブロック数。
    pub known_blocks: usize,
    /// `;` 終端ステートメント行数。
    pub statements: usize,
    /// `include "…";` 行数。
    pub include_lines: usize,
    /// `//`・`/*` コメント行数。
    pub comments: usize,
}

/// `t` がブロック頭 (`word [args] {`) か。
fn is_block_head(t: &str) -> bool {
    let Some(head) = t.strip_suffix('{') else {
        return false;
    };
    let head = head.trim();
    let Some(word) = head.split_whitespace().next() else {
        return false;
    };
    // `=` 含まず、先頭語は識別子または引用文字列 (admin { "Name"; } 系)。
    word.bytes()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'"')
        || word == "*"
}

/// `t` からブロック名を取る。
fn block_name(t: &str) -> &str {
    let head = t.strip_suffix('{').unwrap_or(t);
    head.split_whitespace().next().unwrap_or("")
}

/// UnrealIRCd conf らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    (c.known_blocks >= 2 && c.statements >= 3) || (c.blocks >= 3 && c.statements >= 4)
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
        blocks: 0,
        known_blocks: 0,
        statements: 0,
        include_lines: 0,
        comments: 0,
    };
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        if t.starts_with("//") || t.starts_with("/*") || t.starts_with('*') {
            c.comments += 1;
            continue;
        }
        if t == "}" || t == "};" {
            continue;
        }
        if is_block_head(t) {
            c.blocks += 1;
            if KNOWN_BLOCKS.contains(&block_name(t)) {
                c.known_blocks += 1;
            }
            continue;
        }
        if t.ends_with(';') {
            c.statements += 1;
            if t.starts_with("include") {
                c.include_lines += 1;
            }
        }
    }
    if c.blocks == 0 && c.statements == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"// unrealircd.conf\ninclude \"modules.default.conf\";\nme {\n        name \"irc.example.net\";\n        info \"ExampleNet\";\n        sid \"001\";\n}\nadmin {\n        \"Ume\";\n        \"ume@example.net\";\n}\nclass clients {\n        pingfreq 90;\n        maxclients 500;\n}\noper shizuku {\n        class opers;\n        mask *@*;\n}\n";

    #[test]
    fn detects_unrealircd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.blocks, 4);
        assert_eq!(c.known_blocks, 4);
        assert_eq!(c.statements, 10);
        assert_eq!(c.include_lines, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"key = value;\nfoo = bar;\n"));
        assert!(!detect(b"{ \"a\": 1 }\n"));
    }
}
