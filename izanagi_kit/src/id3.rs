//! ID3 tag reading — `ID3v2` headers (v2.3/v2.4 frame walk with
//! synchsafe sizes) plus the trailing 128-byte `TAG` v1 block.
//! [`parse`] merges both into [`Tags`]: typed accessors for
//! title/artist/album/year/track/genre plus the raw `T***`/`COMM`
//! frame list. Encoding byte 0 (Latin-1) and 3 (UTF-8) decode
//! natively; UTF-16 payloads decode the BMP subset when a BOM or
//! leading zero pattern makes the order unambiguous.
//!
//! ```
//! use izanagi_kit::id3::parse;
//! let mut f = b"ID3\x04\x00\x00\x00\x00\x00\x1D".to_vec();
//! f.extend_from_slice(b"TIT2\x00\x00\x00\x0D\x00\x00\x03Song Title!\x00");
//! let t = parse(&f).unwrap();
//! assert_eq!(t.title.as_deref(), Some("Song Title!"));
//! ```

use std::collections::BTreeMap;

/// A parsed tag set: named fields (v1 and v2 fill the same slots) plus
/// the full v2 text-frame map keyed by frame id (`TIT2`, `TPE1`…).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tags {
    /// Song title (`TIT2` or v1 title).
    pub title: Option<String>,
    /// Artist (`TPE1` or v1 artist).
    pub artist: Option<String>,
    /// Album (`TALB` or v1 album).
    pub album: Option<String>,
    /// Year/date (`TDRC`/`TYER` or v1 year).
    pub year: Option<String>,
    /// Track number (`TRCK`, parsed before any `/` total).
    pub track: Option<u32>,
    /// Genre (`TCON`, or the v1 numeric code resolved to a name when
    /// it is a known id).
    pub genre: Option<String>,
    /// All decoded `T`-prefixed text frames, id → text.
    pub frames: BTreeMap<String, String>,
}

/// Winamp-era ID3v1 genre names (the 80 canonical ids).
const GENRES: &[&str] = &[
    "Blues",
    "Classic Rock",
    "Country",
    "Dance",
    "Disco",
    "Funk",
    "Grunge",
    "Hip-Hop",
    "Jazz",
    "Metal",
    "New Age",
    "Oldies",
    "Other",
    "Pop",
    "R&B",
    "Rap",
    "Reggae",
    "Rock",
    "Techno",
    "Industrial",
    "Alternative",
    "Ska",
    "Death Metal",
    "Pranks",
    "Soundtrack",
    "Euro-Techno",
    "Ambient",
    "Trip-Hop",
    "Vocal",
    "Jazz+Funk",
    "Fusion",
    "Trance",
    "Classical",
    "Instrumental",
    "Acid",
    "House",
    "Game",
    "Sound Clip",
    "Gospel",
    "Noise",
    "Alternative Rock",
    "Bass",
    "Soul",
    "Punk",
    "Space",
    "Meditative",
    "Instrumental Pop",
    "Instrumental Rock",
    "Ethnic",
    "Gothic",
    "Darkwave",
    "Techno-Industrial",
    "Electronic",
    "Pop-Folk",
    "Eurodance",
    "Dream",
    "Southern Rock",
    "Comedy",
    "Cult",
    "Gangsta",
    "Top 40",
    "Christian Rap",
    "Pop/Funk",
    "Jungle",
    "Native US",
    "Cabaret",
    "New Wave",
    "Psychedelic",
    "Rave",
    "Showtunes",
    "Trailer",
    "Lo-Fi",
    "Tribal",
    "Acid Punk",
    "Acid Jazz",
    "Polka",
    "Retro",
    "Musical",
    "Rock & Roll",
    "Hard Rock",
];

fn syncsafe(b: &[u8]) -> Option<u32> {
    if b.len() < 4 || b.iter().take(4).any(|&x| x & 0x80 != 0) {
        return None;
    }
    Some(((b[0] as u32) << 21) | ((b[1] as u32) << 14) | ((b[2] as u32) << 7) | b[3] as u32)
}

fn plain(b: &[u8]) -> Option<u32> {
    if b.len() < 4 {
        return None;
    }
    Some(((b[0] as u32) << 24) | ((b[1] as u32) << 16) | ((b[2] as u32) << 8) | b[3] as u32)
}

/// Decode a text frame payload: byte 0 is the encoding
/// (0 Latin-1, 1 UTF-16 w/ BOM, 2 UTF-16BE, 3 UTF-8).
fn decode_text(d: &[u8]) -> Option<String> {
    let (&enc, body) = d.split_first()?;
    let s = match enc {
        0 => body.iter().map(|&b| b as char).collect::<String>(),
        1 => {
            // UTF-16 with BOM
            let le = match body.get(..2) {
                Some(b"\xFF\xFE") => true,
                Some(b"\xFE\xFF") => false,
                _ => return None,
            };
            utf16(&body[2..], le)?
        }
        2 => utf16(body, false)?,
        3 => String::from_utf8(body.to_vec()).ok()?,
        _ => return None,
    };
    Some(s.trim_end_matches('\0').to_string())
}

fn utf16(d: &[u8], le: bool) -> Option<String> {
    let mut out = String::new();
    let mut i = 0;
    while i + 1 < d.len() {
        let (a, b) = (d[i] as u16, d[i + 1] as u16);
        let v = if le { a | (b << 8) } else { (a << 8) | b };
        if v == 0 {
            break;
        }
        out.push(char::from_u32(v as u32).unwrap_or('\u{FFFD}'));
        i += 2;
    }
    Some(out)
}

fn v1(d: &[u8], t: &mut Tags) {
    if d.len() < 128 {
        return;
    }
    let tag = &d[d.len() - 128..];
    if &tag[..3] != b"TAG" {
        return;
    }
    let take = |a: usize, b: usize| {
        let end = tag[a..b].iter().position(|&c| c == 0).unwrap_or(b - a);
        let s = String::from_utf8_lossy(&tag[a..a + end]).to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    };
    if t.title.is_none() {
        t.title = take(3, 33);
    }
    if t.artist.is_none() {
        t.artist = take(33, 63);
    }
    if t.album.is_none() {
        t.album = take(63, 93);
    }
    if t.year.is_none() {
        t.year = take(93, 97);
    }
    // v1.1: comment byte 28=0, byte 29=track
    if t.track.is_none() && tag[125] == 0 && tag[126] != 0 {
        t.track = Some(tag[126] as u32);
    }
    if t.genre.is_none() {
        t.genre = GENRES.get(tag[127] as usize).map(|s| s.to_string());
    }
}

/// Parse the leading ID3v2 tag and/or the trailing ID3v1 tag of an
/// MP3 stream. `None` only when neither tag exists at all — a
/// truncated v2 tag yields whatever frames were readable plus v1.
pub fn parse(d: &[u8]) -> Option<Tags> {
    let mut t = Tags::default();
    let mut any = false;
    if d.len() >= 10 && &d[..3] == b"ID3" {
        let major = d[3];
        if !(3..=4).contains(&major) {
            return None;
        }
        let size = syncsafe(&d[6..10])? as usize;
        let end = 10usize.checked_add(size)?.min(d.len());
        let mut i = 10;
        // extended header (v2.4 flag 0x40 / v2.3 flag 0x40)
        if d[4] & 0x40 != 0 {
            if let Some(ext) = if major == 3 {
                plain(&d[i..])
            } else {
                syncsafe(&d[i..])
            } {
                i += ext as usize + if major == 3 { 4 } else { 0 };
            }
        }
        while i + 10 <= end {
            let id = &d[i..i + 4];
            if id.iter().all(|&c| c == 0)
                || !id
                    .iter()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            {
                break;
            }
            let fid = String::from_utf8_lossy(id).into_owned();
            let fsize = if major == 4 {
                syncsafe(&d[i + 4..i + 8])? as usize
            } else {
                plain(&d[i + 4..i + 8])? as usize
            };
            if fsize == 0 || i + 10 + fsize > end {
                break;
            }
            let body = &d[i + 10..i + 10 + fsize];
            if fid.starts_with('T') && fid.len() == 4 {
                if let Some(txt) = decode_text(body) {
                    match fid.as_str() {
                        "TIT2" if t.title.is_none() => t.title = Some(txt.clone()),
                        "TPE1" if t.artist.is_none() => t.artist = Some(txt.clone()),
                        "TALB" if t.album.is_none() => t.album = Some(txt.clone()),
                        "TDRC" | "TYER" if t.year.is_none() => t.year = Some(txt.clone()),
                        "TRCK" if t.track.is_none() => {
                            t.track = txt.split('/').next()?.parse().ok();
                        }
                        "TCON" if t.genre.is_none() => t.genre = Some(txt.clone()),
                        _ => {}
                    }
                    t.frames.insert(fid, txt);
                }
            }
            i += 10 + fsize;
        }
        any = true;
    }
    v1(d, &mut t);
    if any || t.title.is_some() || t.artist.is_some() {
        Some(t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v2(frames: &[(&str, &[u8])]) -> Vec<u8> {
        let mut body = Vec::new();
        for (id, payload) in frames {
            body.extend_from_slice(id.as_bytes());
            let n = payload.len();
            body.extend_from_slice(&[
                ((n >> 21) & 0x7F) as u8,
                ((n >> 14) & 0x7F) as u8,
                ((n >> 7) & 0x7F) as u8,
                (n & 0x7F) as u8,
            ]);
            body.extend_from_slice(&[0, 0]); // flags
            body.extend_from_slice(payload);
        }
        let mut d = b"ID3\x04\x00\x00".to_vec();
        let n = body.len();
        d.extend_from_slice(&[
            ((n >> 21) & 0x7F) as u8,
            ((n >> 14) & 0x7F) as u8,
            ((n >> 7) & 0x7F) as u8,
            (n & 0x7F) as u8,
        ]);
        d.extend_from_slice(&body);
        d
    }

    #[test]
    fn v2_text_frames() {
        let d = v2(&[
            ("TIT2", &[3, b'S', b'o', b'n', b'g']),
            ("TPE1", &[0, b'A', b'r', b't']),
            ("TRCK", &[3, b'3', b'/', b'1', b'2']),
            ("TCON", &[3, b'R', b'o', b'c', b'k']),
        ]);
        let t = parse(&d).unwrap();
        assert_eq!(t.title.as_deref(), Some("Song"));
        assert_eq!(t.artist.as_deref(), Some("Art"));
        assert_eq!(t.track, Some(3));
        assert_eq!(t.genre.as_deref(), Some("Rock"));
        assert_eq!(t.frames.get("TIT2").unwrap(), "Song");
    }

    #[test]
    fn utf16_bom() {
        // UTF-16LE BOM + "Hi" = FF FE 48 00 69 00
        let d = v2(&[("TIT2", &[1, 0xFF, 0xFE, 0x48, 0, 0x69, 0])]);
        assert_eq!(parse(&d).unwrap().title.as_deref(), Some("Hi"));
    }

    #[test]
    fn v1_tag() {
        let mut d = vec![0u8; 128];
        d[0] = b'T';
        d[1] = b'A';
        d[2] = b'G';
        d[3..8].copy_from_slice(b"Title");
        d[33..36].copy_from_slice(b"Art");
        d[125] = 0;
        d[126] = 5;
        d[127] = 17; // Rock
        let t = parse(&d).unwrap();
        assert_eq!(t.title.as_deref(), Some("Title"));
        assert_eq!(t.artist.as_deref(), Some("Art"));
        assert_eq!(t.track, Some(5));
        assert_eq!(t.genre.as_deref(), Some("Rock"));
    }

    #[test]
    fn synchsafe_boundaries() {
        // size with a high bit set is invalid
        let mut d = b"ID3\x04\x00\x00".to_vec();
        d.extend_from_slice(&[0x80, 0, 0, 0]);
        assert!(parse(&d).is_none());
    }

    #[test]
    fn bad_inputs() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\xFF\xFBmp3data").is_none());
    }

    #[test]
    fn determinism() {
        let d = v2(&[("TIT2", &[3, b'x'])]);
        assert_eq!(parse(&d), parse(&d));
    }
}
