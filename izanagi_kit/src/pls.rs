//! PLS playlist (Shoutcast-style INI playlist).
//!
//! A `[playlist]` section holds numbered triples `FileN=` / `TitleN=` /
//! `LengthN=` (length in seconds, `-1` for streams) plus
//! `NumberOfEntries` and `Version`. Keys and the section name are
//! case-insensitive; anything outside the `[playlist]` section is
//! ignored.
//!
//! ```
//! use izanagi_kit::pls::parse;
//!
//! let p = parse(b"[playlist]\nFile1=http://x/a.mp3\nTitle1=A\nLength1=-1\nFile2=b.mp3\nNumberOfEntries=2\nVersion=2\n").unwrap();
//! assert_eq!(p.entries.len(), 2);
//! assert_eq!(p.entries[0].title.as_deref(), Some("A"));
//! assert_eq!(p.entries[1].file.as_deref(), Some("b.mp3"));
//! ```

/// One playlist track.
#[derive(Clone, Debug)]
pub struct PlsEntry {
    /// `FileN` — URL or path.
    pub file: Option<String>,
    /// `TitleN`.
    pub title: Option<String>,
    /// `LengthN` in seconds (`-1` = stream).
    pub length: Option<i64>,
}

/// A parsed `.pls` file.
#[derive(Clone, Debug)]
pub struct Pls {
    /// `Version` value (usually `2`).
    pub version: Option<String>,
    /// Tracks indexed by their `N` suffix, in ascending order.
    pub entries: Vec<PlsEntry>,
}

/// Parse a PLS file. `None` when there is no `[playlist]` section.
pub fn parse(d: &[u8]) -> Option<Pls> {
    let text = std::str::from_utf8(d).ok()?;
    let mut in_playlist = false;
    let mut files: Vec<(usize, String)> = Vec::new();
    let mut titles: Vec<(usize, String)> = Vec::new();
    let mut lengths: Vec<(usize, i64)> = Vec::new();
    let mut version = None;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('[') {
            in_playlist = t
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .map(|s| s.trim().eq_ignore_ascii_case("playlist"))
                == Some(true);
            continue;
        }
        if !in_playlist {
            continue;
        }
        let (k, v) = t.split_once('=')?;
        let kl = k.trim().to_lowercase();
        let v = v.trim().to_string();
        let num = |prefix: &str, dst: &mut Vec<(usize, String)>| {
            if let Some(rest) = kl.strip_prefix(prefix) {
                if let Ok(n) = rest.parse::<usize>() {
                    dst.push((n, v.clone()));
                    return true;
                }
            }
            false
        };
        if num("file", &mut files) {
            continue;
        }
        if num("title", &mut titles) {
            continue;
        }
        if let Some(rest) = kl.strip_prefix("length") {
            if let Ok(n) = rest.parse::<usize>() {
                lengths.push((n, v.parse().ok()?));
                continue;
            }
        }
        if kl == "version" {
            version = Some(v);
        }
    }
    if files.is_empty() {
        return None;
    }
    let n = files.iter().map(|f| f.0).max()?;
    let lookup =
        |v: &[(usize, String)], i: usize| v.iter().find(|(n2, _)| *n2 == i).map(|(_, s)| s.clone());
    let lookuplen =
        |v: &[(usize, i64)], i: usize| v.iter().find(|(n2, _)| *n2 == i).map(|(_, s)| *s);
    let mut entries = Vec::new();
    for i in 1..=n {
        entries.push(PlsEntry {
            file: lookup(&files, i),
            title: lookup(&titles, i),
            length: lookuplen(&lengths, i),
        });
    }
    Some(Pls { version, entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let p = parse(
            b"[playlist]\nFile1=a\nFile2=b\nTitle2=B\nLength2=180\nnumberofentries=2\nVersion=2\n",
        )
        .unwrap();
        assert_eq!(p.entries.len(), 2);
        assert_eq!(p.entries[1].title.as_deref(), Some("B"));
        assert_eq!(p.entries[1].length, Some(180));
        assert_eq!(p.version.as_deref(), Some("2"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"[other]\nFile1=a\n").is_none());
        assert!(parse(b"[playlist]\n").is_none());
        assert!(parse(b"[playlist]\nno-equals\nFile1=a\n").is_none());
    }
}
