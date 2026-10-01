//! aerc `aerc.conf`/`binds.conf`/`accounts.conf` の検出・カウント。
//!
//! INI 風だが `[ui:subject=…]` のようなコンテキスト付きセクションを持ち、
//! 代入は `key=value`。
//!
//! ```
//! let cfg = b"[ui]\nindex-format=%Z %-25.25n %s\nsidebar-width=30\nempty-message=(no messages)\n\
//!             this-tab-aerc=true\nmouse-enabled=false\n\n\
//!             [viewer]\npager=less -R\nalternatives=text/plain,text/html\n";
//! assert!(izanagi_kit::aercconf::detect(cfg));
//! let c = izanagi_kit::aercconf::parse(cfg).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.entries, 7);
//! assert_eq!(c.bool_entries, 2);
//! ```

/// aerc の既知セクション名 (コンテキスト `:` の前半)。
const KNOWN_SECTIONS: &[&str] = &[
    "general",
    "ui",
    "viewer",
    "composer",
    "filters",
    "triggers",
    "statusline",
    "templates",
    "linter",
    "cols",
    "columns",
    "binds",
    "compose",
    "terminal",
    "accounts",
];

fn key_of(s: &str) -> Option<(&str, &str)> {
    let i = s.find('=')?;
    let k = s[..i].trim();
    if k.is_empty()
        || !k
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
    {
        return None;
    }
    Some((k, s[i + 1..].trim()))
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知 aerc セクション数。
    pub sections: usize,
    /// `[ui:…]`/`[viewer:…]` 等コンテキスト付きセクション数。
    pub context_sections: usize,
    /// `key=value` 行数 (既知セクション内)。
    pub entries: usize,
    /// `=true`/`=false` 行数。
    pub bool_entries: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

/// `b` が aerc 設定形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.sections >= 1 && c.entries >= 2
}

/// `b` を aerc 設定として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        context_sections: 0,
        entries: 0,
        bool_entries: 0,
        comments: 0,
    };
    let mut in_known = false;
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') {
            let inner = s
                .strip_prefix('[')
                .unwrap_or(s)
                .split(']')
                .next()
                .unwrap_or("")
                .trim();
            let head = inner.split(':').next().unwrap_or("").trim();
            in_known = KNOWN_SECTIONS.contains(&head);
            if in_known {
                c.sections += 1;
                if inner.contains(':') {
                    c.context_sections += 1;
                }
            }
            continue;
        }
        if !in_known {
            continue;
        }
        if let Some((_k, v)) = key_of(s) {
            c.entries += 1;
            if matches!(v, "true" | "false") {
                c.bool_entries += 1;
            }
        }
    }
    if c.sections == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_aerc_conf() {
        let cfg = b"# aerc config\n[ui]\nindex-format=%Z %-25.25n %s\n\
                    styleset-name=default\nmouse-enabled=true\n\n\
                    [ui:account=Work]\nindex-format=custom\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.context_sections, 1);
        assert_eq!(c.entries, 4);
        assert_eq!(c.bool_entries, 1);
    }

    #[test]
    fn rejects_plain_ini() {
        assert!(!detect(b"[section]\nfoo=bar\nbaz=quux\n"));
    }
}
