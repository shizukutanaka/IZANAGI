//! ZNC `znc.conf` — `<Tag arg>`/`</Tag>` ブロック + `Key = Value`
//! の XML 風形式 (ただし属性ではなく位置引数のみ)。
//!
//! ```
//! let cfg = b"<Global>\n        ServerThrottle = 10\n</Global>\n<User shizuku>\n        Admin = true\n        Nick = shizuku\n        <Pass password>\n                Method = sha256\n        </Pass>\n        <Network libera>\n                Server = irc.libera.chat +6697\n        </Network>\n</User>\n";
//! assert!(izanagi_kit::znc::detect(cfg));
//! let c = izanagi_kit::znc::parse(cfg).unwrap();
//! assert_eq!(c.open_tags, 4);
//! assert_eq!(c.entries, 5);
//! ```

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// `<Name …>` 開始タグ数。
    pub open_tags: usize,
    /// `</Name>` 終了タグ数。
    pub close_tags: usize,
    /// 引数付き開始タグ数。
    pub arg_tags: usize,
    /// `Key = Value` 行数。
    pub entries: usize,
    /// `//` コメント行数。
    pub comments: usize,
}

/// `t` が `<Name` または `<Name arg>` 形式の開始タグか
/// (arg に `=` を含まない = XML 属性のない ZNC 風)。
fn is_open_tag(t: &str) -> bool {
    let Some(inner) = t.strip_prefix('<') else {
        return false;
    };
    if inner.starts_with('/') || inner.starts_with('?') || inner.starts_with('!') {
        return false;
    }
    let inner = inner.strip_suffix('>').unwrap_or(inner);
    let mut it = inner.split_whitespace();
    let Some(name) = it.next() else {
        return false;
    };
    if name.is_empty() || !name.bytes().all(|ch| ch.is_ascii_alphabetic()) {
        return false;
    }
    // 残りは位置引数のみ (`=` を含む属性は ZNC では使わない)。
    it.all(|a| !a.contains('='))
}

/// `t` が `</Name>` 終了タグか。
fn is_close_tag(t: &str) -> bool {
    let Some(inner) = t.strip_prefix("</") else {
        return false;
    };
    let Some(name) = inner.strip_suffix('>') else {
        return false;
    };
    !name.is_empty() && name.bytes().all(|ch| ch.is_ascii_alphabetic())
}

/// ZNC conf らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.open_tags >= 2 && c.close_tags >= 2 && c.entries >= 3
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
        open_tags: 0,
        close_tags: 0,
        arg_tags: 0,
        entries: 0,
        comments: 0,
    };
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        if t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if is_close_tag(t) {
            c.close_tags += 1;
            continue;
        }
        if is_open_tag(t) {
            c.open_tags += 1;
            if t.contains(' ') {
                c.arg_tags += 1;
            }
            continue;
        }
        if let Some((key, _)) = t.split_once('=') {
            let key = key.trim();
            if !key.is_empty()
                && key
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-')
            {
                c.entries += 1;
            }
        }
    }
    if c.open_tags == 0 && c.entries == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"<Listener l0>\n        Port = 6697\n        IPv4 = true\n        SSL = true\n        AllowIRC = true\n</Listener>\n<User ume>\n        Admin = true\n        Nick = ume\n        <Network libera>\n                Server = irc.libera.chat +6697\n                <Chan #iz>\n                </Chan>\n        </Network>\n</User>\n";

    #[test]
    fn detects_znc() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.open_tags, 4);
        assert_eq!(c.close_tags, 4);
        assert_eq!(c.arg_tags, 4);
        assert_eq!(c.entries, 7);
    }

    #[test]
    fn rejects_real_xml() {
        // XML 属性 (`=` 付き) は ZNC タグとみなさない。
        assert!(!detect(
            b"<app name=\"x\"><item id=\"1\">v</item><item id=\"2\">w</item></app>"
        ));
        assert!(!detect(b"key = value\nfoo = bar\n"));
    }
}
