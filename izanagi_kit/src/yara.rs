//! YARA rule file scanner.
//!
//! A `.yar` source contains `import "module"` directives followed by
//! `rule NAME [: tag ...] { sections }` blocks, each with optional
//! `meta:` / `strings:` / `condition:` sections. Scanning is
//! string- and comment-aware so braces inside `"..."` and `//`/`/**/`
//! comments do not confuse the block matcher.
//!
//! ```
//! let src = br#"import "pe"
//! rule evil : crime {
//!   meta: author = "x"
//!   strings: $a = "MZ"
//!   condition: $a
//! }
//! "#;
//! let y = izanagi_kit::yara::parse(src).unwrap();
//! assert_eq!(y.imports, 1);
//! assert_eq!(y.rules, ["evil"]);
//! assert_eq!(y.string_defs, 1);
//! ```
//!
//! Reference: YARA documentation "Writing rules" (VirusTotal/YARA
//! repo) — rule grammar, section names, and `import`/`include`.

/// Parsed `.yar` statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Yara {
    /// `import "..."` directive count.
    pub imports: usize,
    /// `include "..."` directive count.
    pub includes: usize,
    /// Rule names in declaration order.
    pub rules: Vec<String>,
    /// Total tag entries across all rules.
    pub tags: usize,
    /// `meta:` section count.
    pub meta_sections: usize,
    /// `strings:` section count.
    pub string_sections: usize,
    /// `condition:` section count.
    pub condition_sections: usize,
    /// `$name =` string-definition count.
    pub string_defs: usize,
}

fn ident(b: &[u8], i: usize) -> Option<(usize, String)> {
    let st = i;
    let mut j = i;
    while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_' || b[j] == b'*') {
        j += 1;
    }
    if j == st {
        None
    } else {
        Some((j, String::from_utf8_lossy(&b[st..j]).into_owned()))
    }
}

fn ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Find the matching `}` for the `{` at `b[open]`, skipping
/// quoted strings, line comments and block comments.
fn match_brace(b: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
            }
            b'/' if i + 1 < b.len() && b[i + 1] == b'/' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < b.len() && b[i + 1] == b'*' => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 1;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn count_section(body: &[u8], name: &[u8]) -> usize {
    usize::from(body.windows(name.len()).any(|w| w == name))
}

/// Count `$name =` string definitions (whitespace-preceded `$`, then
/// identifier, then `=`).
fn count_string_defs(body: &[u8]) -> usize {
    let mut n = 0usize;
    let mut i = 0usize;
    while i < body.len() {
        if body[i] != b'$'
            || !body[..i]
                .last()
                .map(|c| c.is_ascii_whitespace() || *c == b',')
                .unwrap_or(true)
        {
            i += 1;
            continue;
        }
        let (e, _) = match ident(body, i + 1) {
            Some(x) => x,
            None => {
                i += 1;
                continue;
            }
        };
        let p = ws(body, e);
        if body.get(p) == Some(&b'=') {
            n += 1;
        }
        i = e;
    }
    n
}

/// Parse a YARA source file; `None` when no `rule` block exists.
pub fn parse(d: &[u8]) -> Option<Yara> {
    let mut out = Yara {
        imports: 0,
        includes: 0,
        rules: Vec::new(),
        tags: 0,
        meta_sections: 0,
        string_sections: 0,
        condition_sections: 0,
        string_defs: 0,
    };
    let mut i = 0usize;
    while i < d.len() {
        // directive or rule at (possibly whitespace-preceded) word start
        let j = ws(d, i);
        let (after_word, word) = match ident(d, j) {
            Some(x) => x,
            None => {
                i += 1;
                continue;
            }
        };
        let k = ws(d, after_word);
        match word.as_str() {
            "import" | "include" => {
                if d.get(k) == Some(&b'"') {
                    if word == "import" {
                        out.imports += 1;
                    } else {
                        out.includes += 1;
                    }
                }
                i = k;
            }
            "private" | "global" => {
                i = k; // modifier prefixes `rule`
            }
            "rule" => {
                let (after_name, name) = ident(d, k)?;
                let mut p = ws(d, after_name);
                if d.get(p) == Some(&b':') {
                    p = ws(d, p + 1);
                    while let Some((e, _)) = ident(d, p) {
                        out.tags += 1;
                        p = ws(d, e);
                    }
                }
                if d.get(p) != Some(&b'{') {
                    i = p;
                    continue;
                }
                let end = match_brace(d, p)?;
                let body = &d[p + 1..end];
                out.rules.push(name);
                out.meta_sections += count_section(body, b"meta:");
                out.string_sections += count_section(body, b"strings:");
                out.condition_sections += count_section(body, b"condition:");
                out.string_defs += count_string_defs(body);
                i = end + 1;
            }
            _ => {
                i = k;
            }
        }
    }
    if out.rules.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// `true` if the buffer looks like a YARA rule file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = br#"import "pe"
include "a.yar"
// comment with { brace
private rule r1 : t1 t2 {
  meta: author = "x"
  strings: $a = "M{Z}" // } inside string
  condition: $a
}
rule r2 { strings: $b = "{x}" condition: $b }
"#;

    #[test]
    fn parses() {
        let y = parse(SRC).unwrap();
        assert_eq!(y.imports, 1);
        assert_eq!(y.includes, 1);
        assert_eq!(y.rules, ["r1", "r2"]);
        assert_eq!(y.tags, 2);
        assert_eq!(y.meta_sections, 1);
        assert_eq!(y.string_sections, 2);
        assert_eq!(y.condition_sections, 2);
        assert_eq!(y.string_defs, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"rule").is_none());
        assert!(parse(b"import \"pe\"").is_none()); // directive only, no rule
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(!detect(b"rule x {")); // unterminated
    }
}
