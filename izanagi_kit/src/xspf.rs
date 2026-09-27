//! XSPF — XML Shareable Playlist Format ("spiff").
//!
//! `<playlist version="1">` holds optional `<title>`/`<creator>` and a
//! `<trackList>` of `<track>` elements with `<location>` (required),
//! `<title>`, `<creator>`, `<album>`, `<duration>` (ms).
//!
//! ```
//! use izanagi_kit::xspf::parse;
//!
//! let x = parse(br#"<playlist version="1"><title>T</title><trackList><track><location>a.mp3</location><title>A</title></track></trackList></playlist>"#).unwrap();
//! assert_eq!(x.version, "1");
//! assert_eq!(x.tracks[0].location.as_deref(), Some("a.mp3"));
//! ```

/// One `<track>`.
#[derive(Clone, Debug)]
pub struct Track {
    /// `<location>` — the media URI (optional in this parser).
    pub location: Option<String>,
    /// `<title>`.
    pub title: Option<String>,
    /// `<creator>` (artist).
    pub creator: Option<String>,
    /// `<album>`.
    pub album: Option<String>,
}

/// A parsed `<playlist>`.
#[derive(Clone, Debug)]
pub struct Xspf {
    /// `version` attribute (`"0"` or `"1"`).
    pub version: String,
    /// Playlist-level `<title>`.
    pub title: Option<String>,
    /// Tracks in document order.
    pub tracks: Vec<Track>,
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let mut pos = 0;
    while let Some(rel) = tag[pos..].find(key) {
        let i = pos + rel;
        let prev = tag.as_bytes().get(i.wrapping_sub(1));
        if i > 0 && prev.map(|b| b.is_ascii_alphanumeric() || *b == b'-') == Some(true) {
            pos = i + key.len();
            continue;
        }
        let mut j = i + key.len();
        let b = tag.as_bytes();
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        if b.get(j) != Some(&b'=') {
            pos = i + key.len();
            continue;
        }
        j += 1;
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        let q = *b.get(j)?;
        if q != b'"' && q != b'\'' {
            return None;
        }
        let end = tag[j + 1..].find(q as char)? + j + 1;
        return Some(unescape(&tag[j + 1..end]));
    }
    None
}

/// Inner text of `<tag ..>...</tag>` at `from` (self-closing → "").
fn element(d: &str, tag: &str, from: usize, end: usize) -> Option<(String, String, usize)> {
    let open = d[from..end].find('<')? + from;
    let gt = d[open..end].find('>')? + open;
    if !d[open + 1..].starts_with(tag) {
        return None;
    }
    let head = d[open + 1..gt].to_string();
    if head.ends_with('/') {
        return Some((head, String::new(), gt + 1));
    }
    let close_tag = format!("</{tag}>");
    let close = d[gt..end].find(&close_tag)? + gt;
    let text = unescape(&d[gt + 1..close]);
    Some((head, text, close + close_tag.len()))
}

/// Parse an XSPF document. `None` without `<playlist>` / `version`.
pub fn parse(d: &[u8]) -> Option<Xspf> {
    let text = std::str::from_utf8(d).ok()?;
    let end = text.len();
    let p = text.find("<playlist")?;
    let gt = text[p..].find('>')? + p;
    let version = attr(&text[p..gt], "version")?;
    let mut i = gt + 1;
    let mut title = None;
    let mut tracks = Vec::new();
    let mut in_tracklist = false;
    while i < end {
        let Some(rel) = text[i..end].find('<') else {
            break;
        };
        let open = rel + i;
        let rest = &text[open..];
        if rest.starts_with("</playlist") {
            break;
        }
        if rest.starts_with("<trackList") {
            in_tracklist = true;
            i = text[open..end].find('>')? + open + 1;
            continue;
        }
        if rest.starts_with("</trackList") {
            in_tracklist = false;
            i = open + 11;
            continue;
        }
        if in_tracklist && rest.starts_with("<track") {
            let close = text[open..end].find("</track>")? + open;
            let mut j = text[open..end].find('>')? + open + 1;
            let mut tr = Track {
                location: None,
                title: None,
                creator: None,
                album: None,
            };
            while j < close {
                let o = match text[j..close].find('<') {
                    Some(r) => j + r,
                    None => break,
                };
                let mut found = false;
                for tag in ["location", "title", "creator", "album"] {
                    if let Some((_h, t, nj)) = element(text, tag, o, close) {
                        match tag {
                            "location" => tr.location = Some(t),
                            "title" => tr.title = Some(t),
                            "creator" => tr.creator = Some(t),
                            _ => tr.album = Some(t),
                        }
                        j = nj;
                        found = true;
                        break;
                    }
                }
                if !found {
                    j = text[o..close].find('>')? + o + 1;
                }
            }
            tracks.push(tr);
            i = close + 8;
            continue;
        }
        if !in_tracklist && rest.starts_with("<title") {
            if let Some((_h, t, nj)) = element(text, "title", open, end) {
                title = Some(t);
                i = nj;
                continue;
            }
        }
        i = open + 1;
    }
    Some(Xspf {
        version,
        title,
        tracks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let x = parse(br#"<playlist version="1"><title>L</title><trackList><track><location>u</location><creator>c</creator><album>al</album><meta>x</meta></track><track><location>v</location></track></trackList></playlist>"#).unwrap();
        assert_eq!(x.title.as_deref(), Some("L"));
        assert_eq!(x.tracks.len(), 2);
        assert_eq!(x.tracks[0].creator.as_deref(), Some("c"));
        assert_eq!(x.tracks[0].album.as_deref(), Some("al"));
        assert!(x.tracks[1].title.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<playlist></playlist>").is_none());
        assert!(parse(b"plain text").is_none());
    }
}
