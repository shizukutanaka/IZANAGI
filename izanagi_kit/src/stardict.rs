//! StarDict `.ifo` — the `key=value` header of a StarDict dictionary:
//! `StarDict's dict ifo file`, `version=…`, `bookname=…`,
//! `wordcount=…`, `idxfilesize=…`, optional `synwordcount=`,
//! `idxoffsetbits=64` (v3), `sametypesequence=…`, `author=`.
//!
//! ```
//! use izanagi_kit::stardict::{detect, parse};
//!
//! let d = b"StarDict's dict ifo file\nversion=2\x2e4\x2e2\nbookname=WordNet\n\
//! wordcount=147306\nidxfilesize=2380176\nsametypesequence=m\n";
//! assert!(detect(d));
//! let i = parse(d).unwrap();
//! assert_eq!(i.bookname.as_deref(), Some("WordNet"));
//! assert_eq!(i.wordcount, Some(147306));
//! ```

/// Parsed StarDict `.ifo` census.
#[derive(Debug, Clone, PartialEq)]
pub struct Stardict {
    /// `bookname` value.
    pub bookname: Option<String>,
    /// `version` value (`2.4.2` / `3.0.1`).
    pub version: Option<String>,
    /// `wordcount` entry count.
    pub wordcount: Option<u32>,
    /// `.idx` file byte size.
    pub idxfilesize: Option<u32>,
    /// `.syn` synonym count (absent for v2 or no-synonym dicts).
    pub synwordcount: Option<u32>,
    /// `idxoffsetbits` (`32`/`64`, v3 large dictionaries).
    pub idxoffsetbits: Option<u32>,
    /// `sametypesequence` (entry payload type codes).
    pub sametypesequence: Option<String>,
    /// `author` value.
    pub author: Option<String>,
    /// `description` value.
    pub description: Option<String>,
    /// Total `key=value` pairs (including unknown keys).
    pub pairs: u32,
}

/// `true` on the `StarDict's dict ifo file` banner + `bookname=`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("StarDict's dict ifo file") && s.contains("bookname=")
}

/// Census; `None` without the ifo banner.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Stardict> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut i = Stardict {
        bookname: None,
        version: None,
        wordcount: None,
        idxfilesize: None,
        synwordcount: None,
        idxoffsetbits: None,
        sametypesequence: None,
        author: None,
        description: None,
        pairs: 0,
    };
    for line in s.lines() {
        let t = line.trim();
        let Some(eq) = t.find('=') else {
            continue;
        };
        let (k, v) = (&t[..eq], t[eq + 1..].trim());
        i.pairs += 1;
        match k {
            "bookname" => i.bookname = Some(v.to_string()),
            "version" => i.version = Some(v.to_string()),
            "wordcount" => i.wordcount = v.parse().ok(),
            "idxfilesize" => i.idxfilesize = v.parse().ok(),
            "synwordcount" => i.synwordcount = v.parse().ok(),
            "idxoffsetbits" => i.idxoffsetbits = v.parse().ok(),
            "sametypesequence" => i.sametypesequence = Some(v.to_string()),
            "author" => i.author = Some(v.to_string()),
            "description" => i.description = Some(v.to_string()),
            _ => {}
        }
    }
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"StarDict's dict ifo file\nversion=2\x2e4\x2e2\nbookname=WordNet\n\
wordcount=147306\nidxfilesize=2380176\nsametypesequence=m\nauthor=Me\ndescription=Sample\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"bookname=x"));
        assert!(!detect(b"StarDict's dict ifo file"));
    }

    #[test]
    fn parses() {
        let i = parse(D).unwrap();
        assert_eq!(i.bookname.as_deref(), Some("WordNet"));
        assert_eq!(i.version.as_deref(), Some("2.4.2"));
        assert_eq!(i.wordcount, Some(147306));
        assert_eq!(i.idxfilesize, Some(2380176));
        assert_eq!(i.sametypesequence.as_deref(), Some("m"));
        assert_eq!(i.author.as_deref(), Some("Me"));
        assert_eq!(i.description.as_deref(), Some("Sample"));
        assert_eq!(i.pairs, 7);
        assert!(i.synwordcount.is_none());
    }

    #[test]
    fn v3_fields() {
        let d = b"StarDict's dict ifo file\nversion=3\x2e0\x2e1\nbookname=B\n\
wordcount=1\nidxfilesize=2\nsynwordcount=3\nidxoffsetbits=64\n";
        let i = parse(d).unwrap();
        assert_eq!(i.synwordcount, Some(3));
        assert_eq!(i.idxoffsetbits, Some(64));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
