//! Textile markup scanner (RedCloth syntax).
//!
//! Textile is a humane web text format: block elements start with a
//! signature like `h1.`–`h6.`, `p.`, `bq.`, `bc.`, `notextile.` at the
//! start of a line, inline spans use `*strong*`, `_emphasis_`,
//! `??citation??`, and tables use `|cell|cell|` rows.
//!
//! ```
//! let d = b"h1. Title\n\np. A paragraph with *strong* text.\n\nbq. quoted\n";
//! let t = izanagi_kit::textile::parse(d).unwrap();
//! assert_eq!(t.headings, 1);
//! assert!(t.blockquotes > 0);
//! assert!(t.strong_spans > 0);
//! ```
//!
//! Reference: Textile syntax reference (textile-lang.com / RedCloth
//! `doc/REFERENCE` — the `hN.`/`p.`/`bq.`/`bc.` block signature set).

/// Parsed Textile document statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Textile {
    /// `h1.`–`h6.` block count.
    pub headings: usize,
    /// `p.` paragraph-marker count.
    pub paragraphs: usize,
    /// `bq.` blockquote count.
    pub blockquotes: usize,
    /// `bc.`/`pre.` code-block count.
    pub code_blocks: usize,
    /// `*…*` strong-span count.
    pub strong_spans: usize,
    /// `_…_` emphasis-span count.
    pub emphasis_spans: usize,
    /// `|…|` table-row count.
    pub table_rows: usize,
}

fn count_block(text: &str, sig: &str) -> usize {
    text.lines()
        .filter(|l| l.trim_start().starts_with(sig))
        .count()
}

fn count_inline(text: &str, open: &str) -> usize {
    let mut n = 0;
    let mut rest = text;
    while let Some(i) = rest.find(open) {
        rest = &rest[i + open.len()..];
        match rest.find(open) {
            Some(j) => {
                n += 1;
                rest = &rest[j + open.len()..];
            }
            None => break,
        }
    }
    n
}

/// Parse a Textile document; `None` when no Textile block marker exists.
pub fn parse(d: &[u8]) -> Option<Textile> {
    let text = core::str::from_utf8(d).ok()?;
    let headings: usize = ["h1.", "h2.", "h3.", "h4.", "h5.", "h6."]
        .iter()
        .map(|sig| count_block(text, sig))
        .sum();
    let paragraphs = count_block(text, "p.");
    let blockquotes = count_block(text, "bq.");
    let code_blocks = count_block(text, "bc.") + count_block(text, "pre.");
    let table_rows = text
        .lines()
        .filter(|l| {
            let l = l.trim();
            l.starts_with('|') && l.ends_with('|') && l.len() > 2
        })
        .count();
    if headings + paragraphs + blockquotes + code_blocks + table_rows == 0 {
        return None;
    }
    Some(Textile {
        headings,
        paragraphs,
        blockquotes,
        code_blocks,
        strong_spans: count_inline(text, "*"),
        emphasis_spans: count_inline(text, "_"),
        table_rows,
    })
}

/// `true` if the buffer looks like a Textile document.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] =
        b"h1. Title\n\np. A *strong* and _em_ paragraph.\n\nbq. quoted\nbc. code\n|a|b|\n";

    #[test]
    fn parses() {
        let t = parse(DOC).unwrap();
        assert_eq!(t.headings, 1);
        assert_eq!(t.paragraphs, 1);
        assert_eq!(t.blockquotes, 1);
        assert_eq!(t.code_blocks, 1);
        assert_eq!(t.table_rows, 1);
        assert_eq!(t.strong_spans, 1);
        assert_eq!(t.emphasis_spans, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain text\nno markers\n").is_none());
        assert!(parse(&[0xff, 0x00]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"hello world"));
    }
}
