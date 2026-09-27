//! LRC lyrics — `[mm:ss.xx]` timestamps plus `[key:value]` metadata.
//!
//! Each lyric line carries one or more timestamps (minutes:seconds with
//! 2- or 3-digit fraction); the same text is emitted once per stamp.
//! `ti`, `ar`, `al`, `by` etc. go to [`Lrc::meta`]; `[offset:ms]` to
//! [`Lrc::offset_ms`]. Malformed lines are skipped, not fatal.
//!
//! ```
//! use izanagi_kit::lrc::parse;
//!
//! let l = parse(b"[ti:Song]\n[00:12.50]first line\n[00:13.00][00:14.00]twice\n").unwrap();
//! assert_eq!(l.meta[0], ("ti".into(), "Song".into()));
//! assert_eq!(l.lines[0].millis, 12_500);
//! assert_eq!(l.lines.len(), 3);
//! ```

/// One timed lyric line.
#[derive(Clone, Debug)]
pub struct LrcLine {
    /// Time since song start in milliseconds.
    pub millis: u64,
    /// Lyric text after the last stamp (may be empty).
    pub text: String,
}

/// A parsed `.lrc` file.
#[derive(Clone, Debug)]
pub struct Lrc {
    /// `[key:value]` metadata pairs in file order.
    pub meta: Vec<(String, String)>,
    /// Global `[offset:ms]` shift (default 0).
    pub offset_ms: i64,
    /// Timed lines, in file order (one entry per stamp).
    pub lines: Vec<LrcLine>,
}

fn stamp(s: &str) -> Option<(u64, usize)> {
    // s starts at "mm:ss.xx]" — returns (millis, bytes consumed incl ']').
    let close = s.find(']')?;
    let body = &s[..close];
    let (mm, rest) = body.split_once(':')?;
    let m: u64 = mm.parse().ok()?;
    let (ss, frac) = match rest.split_once(['.', ':']) {
        Some((a, b)) => (a, Some(b)),
        None => (rest, None),
    };
    let sec: u64 = ss.parse().ok()?;
    if sec > 59 {
        return None;
    }
    let ms = match frac {
        None => 0,
        Some(f) => match f.len() {
            2 => f.parse::<u64>().ok()? * 10,
            3 => f.parse::<u64>().ok()?,
            1 => f.parse::<u64>().ok()? * 100,
            _ => return None,
        },
    };
    Some(((m * 60 + sec) * 1000 + ms, close + 1))
}

/// Parse an LRC document. `None` on invalid UTF-8 or no content.
pub fn parse(d: &[u8]) -> Option<Lrc> {
    let text = std::str::from_utf8(d).ok()?;
    let mut meta = Vec::new();
    let mut lines = Vec::new();
    let mut offset_ms: i64 = 0;
    for line in text.lines() {
        let t = line.trim_end();
        if t.is_empty() {
            continue;
        }
        let mut rest = t;
        let mut stamps = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            match stamp(stripped) {
                Some((ms, used)) => {
                    stamps.push(ms);
                    rest = &stripped[used..];
                }
                None => break,
            }
        }
        if stamps.is_empty() {
            // metadata `[key:value]` or junk
            if let Some(inner) = t.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                if let Some((k, v)) = inner.split_once(':') {
                    let k = k.trim().to_lowercase();
                    let v = v.trim().to_string();
                    if k == "offset" {
                        offset_ms = v.parse().ok()?;
                    } else {
                        meta.push((k, v));
                    }
                }
            }
            continue;
        }
        let text = rest.trim().to_string();
        for ms in stamps {
            lines.push(LrcLine {
                millis: ms,
                text: text.clone(),
            });
        }
    }
    if meta.is_empty() && lines.is_empty() {
        return None;
    }
    Some(Lrc {
        meta,
        offset_ms,
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let l = parse(
            b"[ti:T]\n[ar:A]\n[offset:+5]\n[01:02.34]hello\n[00:10.5][00:11]x\nno brackets\n",
        )
        .unwrap();
        assert_eq!(l.meta.len(), 2);
        assert_eq!(l.offset_ms, 5);
        assert_eq!(l.lines[0].millis, 62_340);
        assert_eq!(l.lines[0].text, "hello");
        assert_eq!(l.lines[2].millis, 11_000);
        assert_eq!(l.lines.len(), 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain text only\n").is_none());
        assert!(parse(b"[offset:notanumber]\n[00:01.00]x\n").is_none());
    }
}
