//! Atari ST SNDH music file parser.
//!
//! SNDH is a tagged text header over 68000 machine code: after an
//! optional `Ice!` pack signature or a `BRA` branch (`0x60xx`), a
//! `SNDH` marker introduces tag fields like `TITL`, `COMM`, `RIPP`,
//! `CONV`, `YEAR`, `##NN` (subsong count), `!#NN` (default subsong),
//! `INIT`, `EXIT`, `TCxx` (time codes), ending with `HDNS`. This
//! parser locates `SNDH` within the first 32 bytes and reads the tag
//! stream, ending fields at the next *known* tag word.
//!
//! ```
//! let f = b"\x60\x0eSNDHTITLMusic COMMMe   ##03HDNS";
//! let s = izanagi_kit::sndh::parse(f).unwrap();
//! assert_eq!(s.title.as_deref(), Some("Music"));
//! assert_eq!(s.subsongs, 3);
//! ```

/// Parsed SNDH tag header.
#[derive(Debug, Clone, PartialEq)]
pub struct Sndh {
    /// `TITL` tag text (song title).
    pub title: Option<String>,
    /// `COMM` tag text (composer).
    pub composer: Option<String>,
    /// `RIPP` tag text (ripper handle).
    pub ripper: Option<String>,
    /// `CONV` tag text (converter handle).
    pub converter: Option<String>,
    /// `YEAR` tag text.
    pub year: Option<String>,
    /// `##` subsong count (default 1).
    pub subsongs: usize,
    /// `!#` default subsong index, `None` when absent.
    pub default_subsong: Option<u8>,
    /// Total recognised tag count.
    pub tags: usize,
}

/// True when `d[pos..]` starts a recognised SNDH tag word.
fn is_tag_at(d: &[u8], pos: usize) -> bool {
    if pos + 4 > d.len() {
        return false;
    }
    let t = &d[pos..pos + 4];
    matches!(
        t,
        b"TITL"
            | b"COMM"
            | b"RIPP"
            | b"CONV"
            | b"YEAR"
            | b"HDNS"
            | b"INIT"
            | b"EXIT"
            | b"PLAY"
            | b"ORGN"
            | b"SECO"
            | b"VBLK"
            | b"TIMC"
            | b"TAWH"
            | b"Time"
    ) || (t[0] == b'#' && t[1] == b'#')
        || (t[0] == b'!' && t[1] == b'#')
        || (t[0] == b'T' && t[1] == b'C' && t[2].is_ascii_digit() && t[3].is_ascii_digit())
}

/// Read a tag's text value starting right after a 4-char tag; the
/// value ends at the next recognised tag word or end of input.
fn tag_text(d: &[u8], pos: usize) -> (&str, usize) {
    let mut i = pos;
    while i < d.len() && !is_tag_at(d, i) {
        i += 1;
    }
    (std::str::from_utf8(&d[pos..i]).unwrap_or(""), i)
}

/// Parse an SNDH file; `None` when `SNDH` is absent within the first
/// 32 bytes.
pub fn parse(d: &[u8]) -> Option<Sndh> {
    let scan = d.len().min(32);
    let mut at = None;
    for i in 0..scan.saturating_sub(3) {
        if &d[i..i + 4] == b"SNDH" {
            at = Some(i + 4);
            break;
        }
    }
    let mut pos = at?;
    let mut s = Sndh {
        title: None,
        composer: None,
        ripper: None,
        converter: None,
        year: None,
        subsongs: 1,
        default_subsong: None,
        tags: 0,
    };
    while pos + 4 <= d.len() {
        let tag = &d[pos..pos + 4];
        if tag == b"HDNS" {
            s.tags += 1;
            break;
        }
        match tag {
            b"TITL" | b"COMM" | b"RIPP" | b"CONV" | b"YEAR" => {
                let (v, next) = tag_text(d, pos + 4);
                let v = v.trim_end_matches(&[' ', '\r', '\n', '\0'][..]);
                let v = if v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                };
                match tag {
                    b"TITL" => s.title = v,
                    b"COMM" => s.composer = v,
                    b"RIPP" => s.ripper = v,
                    b"CONV" => s.converter = v,
                    _ => s.year = v,
                }
                s.tags += 1;
                pos = next.max(pos + 4);
            }
            _ => {
                if tag[0] == b'#' && tag[1] == b'#' {
                    if let Ok(n) = std::str::from_utf8(&tag[2..]).unwrap_or("").parse::<u8>() {
                        s.subsongs = n.max(1) as usize;
                        s.tags += 1;
                    }
                } else if tag[0] == b'!' && tag[1] == b'#' {
                    if let Ok(n) = std::str::from_utf8(&tag[2..]).unwrap_or("").parse::<u8>() {
                        s.default_subsong = Some(n);
                        s.tags += 1;
                    }
                }
                pos += 4;
            }
        }
        // Bail if a scan ran to the end without progress.
        if pos + 4 > d.len() {
            break;
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"\x60\x0eSNDHTITLMusic COMMMe   ##03HDNS";
        let s = parse(f).unwrap();
        assert_eq!(s.title.as_deref(), Some("Music"));
        assert_eq!(s.composer.as_deref(), Some("Me"));
        assert_eq!(s.subsongs, 3);
        assert!(s.tags >= 3);
    }

    #[test]
    fn tags() {
        let f = b"SNDHTITLxRIPPyCONVzYEAR2024##02!#01HDNS";
        let s = parse(f).unwrap();
        assert_eq!(s.title.as_deref(), Some("x"));
        assert_eq!(s.ripper.as_deref(), Some("y"));
        assert_eq!(s.converter.as_deref(), Some("z"));
        assert_eq!(s.year.as_deref(), Some("2024"));
        assert_eq!(s.subsongs, 2);
        assert_eq!(s.default_subsong, Some(1));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"no marker here at all...............").is_none());
    }
}
