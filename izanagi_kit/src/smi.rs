//! SAMI (Synchronized Accessible Media Interchange, `.smi`) —
//! Microsoft's HTML-like caption format: `<SAMI>` wrapper with
//! `<SYNC Start=…ms>` timing marks and `<P>` text.
//!
//! ```
//! let d = b"<SAMI><HEAD></HEAD><BODY><SYNC Start=1000><P Class=ENCC>hi</P><SYNC Start=2000><P Class=ENCC>bye</P></BODY></SAMI>";
//! let s = izanagi_kit::smi::parse(d).unwrap();
//! assert_eq!(s.syncs, 2);
//! ```

/// Parsed SAMI summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Smi {
    /// `<SYNC` marks.
    pub syncs: usize,
    /// `Start=` milliseconds on the first `<SYNC>`.
    pub first_start_ms: u64,
    /// `<P ` / `<P>` text blocks.
    pub paras: usize,
}

fn first_sync_ms(s: &str) -> Option<u64> {
    let pos = s.find("SYNC")?;
    let rest = &s[pos..];
    let p = rest.find("START")?;
    let after = rest[p + 5..].trim_start_matches([' ', '=']);
    let end = after
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after.len());
    after[..end].parse().ok()
}

/// Parse a SAMI file (case-insensitive tags); `None` without `<SAMI>`.
pub fn parse(d: &[u8]) -> Option<Smi> {
    let raw = std::str::from_utf8(d).ok()?;
    let upper = raw.to_uppercase();
    if !upper.contains("<SAMI") {
        return None;
    }
    let first_start_ms = first_sync_ms(&upper)?;
    Some(Smi {
        syncs: upper.matches("<SYNC").count(),
        first_start_ms,
        paras: upper.matches("<P ").count() + upper.matches("<P>").count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"<SAMI><BODY><SYNC Start=500><P Class=CC>hi</P><SYNC Start=1500><P>bye</P></BODY></SAMI>";
        let s = parse(d).unwrap();
        assert_eq!((s.syncs, s.paras, s.first_start_ms), (2, 2, 500));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html><body/></html>").is_none());
        assert!(parse(b"<SAMI><BODY>no syncs</BODY></SAMI>").is_none());
    }
}
