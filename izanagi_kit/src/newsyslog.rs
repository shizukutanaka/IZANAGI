//! BSD newsyslog 設定ファイル (`newsyslog.conf`) の解析。
//!
//! `<logfilename> [owner:group] <mode> <count> <size> <when> [flags]` の
//! 7〜8 欄を持つ設定を検出し、エントリ数・圧縮フラグ付き数などを整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::newsyslog;
//!
//! let text = br#"/var/log/messages    644  11  *  $T00  Z
//! /var/log/maillog      640  7   *  @T00  Z
//! "#;
//!
//! assert!(newsyslog::detect(text));
//! let c = newsyslog::parse(text).unwrap();
//! assert_eq!(c.entries, 2);
//! assert_eq!(c.compressed, 2);
//! ```

/// newsyslog.conf の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ログローテーションエントリ数。
    pub entries: usize,
    /// `owner:group` 欄を持つエントリ数。
    pub with_owner: usize,
    /// 圧縮フラグ (`Z`/`J`/`Y`/`X`/`T`) を持つエントリ数。
    pub compressed: usize,
    /// `when` が `@`/`*`/`$` で始まるエントリ数。
    pub timed: usize,
}

const COMPRESS_FLAGS: &[char] = &['Z', 'J', 'Y', 'X', 'T'];

fn looks_like_path(s: &str) -> bool {
    s.starts_with('/') || s.starts_with("/var") || s.starts_with('<')
}

fn is_mode(s: &str) -> bool {
    s.len() >= 3 && s.bytes().all(|c| matches!(c, b'0'..=b'7'))
}

fn is_when(s: &str) -> bool {
    s.starts_with('@')
        || s.starts_with('*')
        || s.starts_with('$')
        || s == "D0"
        || s.starts_with("D")
        || s.starts_with("W")
}

/// `b` が newsyslog.conf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.entries >= 2 || (c.entries >= 1 && c.timed >= 1)
}

fn parse_field_tokens(line: &str) -> Option<Vec<&str>> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 6 {
        return None;
    }
    Some(tokens)
}

/// newsyslog.conf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        entries: 0,
        with_owner: 0,
        compressed: 0,
        timed: 0,
    };
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(tokens) = parse_field_tokens(line) else {
            continue;
        };
        let mut i = 0;
        if !looks_like_path(tokens[i]) {
            continue;
        }
        i += 1;
        if i < tokens.len() && tokens[i].contains(':') {
            counts.with_owner += 1;
            i += 1;
        }
        if i < tokens.len() && is_mode(tokens[i]) {
            i += 1;
        } else {
            continue;
        }
        if i < tokens.len() && tokens[i].bytes().all(|c| c.is_ascii_digit()) {
            i += 1;
        } else {
            continue;
        }
        // size 欄は数値または `*`。
        if i < tokens.len() && (tokens[i].bytes().all(|c| c.is_ascii_digit()) || tokens[i] == "*") {
            i += 1;
        } else {
            continue;
        }
        if i < tokens.len() && is_when(tokens[i]) {
            counts.timed += 1;
            i += 1;
        }
        for tok in &tokens[i..] {
            if tok.chars().any(|c| COMPRESS_FLAGS.contains(&c)) {
                counts.compressed += 1;
                break;
            }
        }
        counts.entries += 1;
    }
    if counts.entries == 0 {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"# newsyslog.conf
/var/log/cron      root:wheel  600  3   100  *    Z
/var/log/daemon.log           644  5   *    $T00 Z
/var/log/maillog              640  7   *    @T00 B
/var/log/messages             644  11  *    $T00 Z
/var/log/ppp.log              640  3   500  *    J
"#;

    #[test]
    fn detects_newsyslog() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 5);
        assert_eq!(c.with_owner, 1);
        assert_eq!(c.compressed, 4);
        assert_eq!(c.timed, 5);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"a b c"));
    }
}
