//! codespell `.codespellrc`/`[codespell]` セクションの検出と構造カウント。
//!
//! INI 形式の `skip`/`ignore-words`/`quiet-level`/`check-filenames`/
//! `check-hidden`/`regex`/`builtin`/`exclude-file` 等のオプションを識別する。
//! `setup.cfg`/`pyproject.toml` の `[codespell]`/`[tool.codespell]` も同一形式。
//!
//! ```
//! let c = izanagi_kit::codespell::parse(
//!     b"[codespell]\nskip = .git,*.lock\nignore-words-list = teh,fo\nquiet-level = 3\ncheck-filenames = \n").unwrap();
//! assert!(c.options >= 4);
//! assert!(izanagi_kit::codespell::detect(
//!     b"[codespell]\nskip = .git\nquiet-level = 3\n"));
//! ```

/// codespell 既知オプション。
const KEYS: &[&str] = &[
    "builtin",
    "check-filenames",
    "check-hidden",
    "chinese-characters",
    "clear-banner",
    "context",
    "count",
    "dictionary",
    "enable-colors",
    "exclude-file",
    "hard-encoding-detection",
    "ignore-multiline-regex",
    "ignore-regex",
    "ignore-words",
    "ignore-words-list",
    "interactive",
    "level",
    "quiet-level",
    "regex",
    "skip",
    "stdin-single-line",
    "uri-ignore-words-list",
    "write-changes",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[codespell]`/`[tool.codespell]` セクション行数。
    pub sections: usize,
    /// 既知オプション行数。
    pub options: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn is_codespell_section(t: &str) -> bool {
    t == "[codespell]" || t == "[tool.codespell]"
}

fn known_key(t: &str) -> bool {
    let Some(eq) = t.find('=') else { return false };
    let k = t[..eq].trim();
    KEYS.contains(&k)
}

/// `.codespellrc` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut in_sec = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        if t.starts_with('[') {
            in_sec = is_codespell_section(t);
            continue;
        }
        if in_sec && known_key(t) {
            hits += 1;
            if hits >= 2 {
                return true;
            }
        }
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_sec = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') {
            in_sec = is_codespell_section(t);
            c.sections += 1;
            continue;
        }
        if in_sec && known_key(t) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# codespell\n[codespell]\nskip = .git,*.lock,go.sum\nignore-words-list = teh,fo,ba\nquiet-level = 3\ncheck-filenames = \ncheck-hidden = true\nbuiltin = clear,rare\n\n[other]\nkey = 1\n";

    #[test]
    fn codespell() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.options, 6);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_codespell() {
        assert!(!detect(b"[flake8]\nmax-line-length = 100\n"));
        assert!(parse(b"text\n").is_none());
    }
}
