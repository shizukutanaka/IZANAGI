//! YUV4MPEG2 (`.y4m`) — the raw-video pipe format used by mjpegtools /
//! ffmpeg: a `YUV4MPEG2 ` header line of space-separated tags
//! (`W`/`H`/`F`/`I`/`A`/`C`/`XYSCSS`), then `FRAME` lines each
//! preceding one frame of raw data.
//!
//! ```
//! use izanagi_kit::y4m::{detect, parse};
//!
//! let d = b"YUV4MPEG2 W4 H2 F30000:1001 Ip A1:1 C420jpeg\nFRAME\n************";
//! assert!(detect(d));
//! let y = parse(d).unwrap();
//! assert_eq!(y.width, 4);
//! assert_eq!(y.height, 2);
//! assert_eq!(y.fps_num, 30000);
//! assert_eq!(y.fps_den, 1001);
//! assert_eq!(y.colorspace.as_deref(), Some("420jpeg"));
//! assert_eq!(y.frames, 1);
//! ```

/// Parsed YUV4MPEG2 header + frame census.
#[derive(Debug, Clone, PartialEq)]
pub struct Y4m {
    /// `W` tag.
    pub width: u32,
    /// `H` tag.
    pub height: u32,
    /// `F` numerator (frames per second).
    pub fps_num: u32,
    /// `F` denominator.
    pub fps_den: u32,
    /// `I` interlace tag (`p`, `t`, `b`, `?`).
    pub interlace: Option<char>,
    /// `A` pixel aspect as written (`"1:1"`, …).
    pub aspect: Option<String>,
    /// `C` colorspace tag (`420`, `420jpeg`, `422`, `444`, `mono`, …).
    pub colorspace: Option<String>,
    /// `XYSCSS` override, when present.
    pub xyscss: Option<String>,
    /// `X` comment tags concatenated with `; `.
    pub comments: Vec<String>,
    /// `FRAME` lines counted.
    pub frames: u32,
}

/// `true` on the `YUV4MPEG2` magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(b"YUV4MPEG2")
}

fn tag_num(t: &str) -> u32 {
    t[1..].parse().unwrap_or(0)
}

/// Pixel bytes per frame from `C`/`XYSCSS` + `W`/`H`; `None` when the
/// chroma layout is unknown (frames then fall back to marker count).
fn frame_bytes(w: u32, h: u32, c: Option<&str>) -> Option<usize> {
    let luma = u64::from(w) * u64::from(h);
    let cl = c?.to_ascii_lowercase();
    // units of half-luma: mono=2, 4:2:0/4:1:1=3, 4:2:2=4, 4:4:4=6, 4:4:4:4=8
    let ratio2: u64 = if cl.starts_with("mono") {
        2
    } else if cl.starts_with("411") || cl.starts_with("420") {
        3
    } else if cl.starts_with("422") {
        4
    } else if cl.starts_with("4444") || cl.contains("alpha") {
        8
    } else if cl.starts_with("444") {
        6
    } else {
        return None;
    };
    let wide = cl.contains("p10")
        || cl.contains("p12")
        || cl.contains("p14")
        || cl.contains("p16")
        || cl.ends_with("-16");
    usize::try_from(luma * ratio2 / 2 * if wide { 2 } else { 1 }).ok()
}

/// Parses the header line + walks `FRAME` records; `None` on bad magic.
/// Frame payloads are raw bytes — only the header must be UTF-8.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Y4m> {
    if !detect(b) {
        return None;
    }
    let eol = b.iter().position(|&c| c == b'\n')?;
    let header = core::str::from_utf8(&b[..eol]).ok()?;
    let mut y = Y4m {
        width: 0,
        height: 0,
        fps_num: 0,
        fps_den: 0,
        interlace: None,
        aspect: None,
        colorspace: None,
        xyscss: None,
        comments: Vec::new(),
        frames: 0,
    };
    for tag in header.split_whitespace().skip(1) {
        match tag.chars().next() {
            Some('W') => y.width = tag_num(tag),
            Some('H') => y.height = tag_num(tag),
            Some('F') => {
                let (n, d) = tag[1..].split_once(':').unwrap_or((&tag[1..], "1"));
                y.fps_num = n.parse().unwrap_or(0);
                y.fps_den = d.parse().unwrap_or(0);
            }
            Some('I') => y.interlace = tag[1..].chars().next(),
            Some('A') => y.aspect = Some(tag[1..].to_string()),
            Some('C') => y.colorspace = Some(tag[1..].to_string()),
            Some('X') => {
                if let Some(v) = tag.strip_prefix("XYSCSS=") {
                    y.xyscss = Some(v.to_string());
                } else {
                    y.comments.push(tag[1..].to_string());
                }
            }
            _ => {}
        }
    }
    let rest = &b[eol + 1..];
    let cs = y.xyscss.as_deref().or(y.colorspace.as_deref());
    match frame_bytes(y.width, y.height, cs) {
        Some(sz) => {
            // skip each frame's pixel payload so `FRAME`-looking bytes
            // inside the image data cannot invent frames
            let mut pos = 0usize;
            while rest[pos..].starts_with(b"FRAME") {
                y.frames += 1;
                let nl = rest[pos..]
                    .iter()
                    .position(|&c| c == b'\n')
                    .map_or(rest.len(), |e| pos + e + 1);
                pos = nl.saturating_add(sz);
                if pos > rest.len() {
                    break;
                }
            }
        }
        None => {
            y.frames = u32::try_from(
                rest.starts_with(b"FRAME") as usize
                    + rest.windows(6).filter(|w| *w == b"\nFRAME").count(),
            )
            .unwrap_or(u32::MAX);
        }
    }
    Some(y)
}

#[cfg(test)]
mod tests {
    use super::*;

    // W4 H2 C420 → 12 pixel bytes per frame; the first carries 0xFF
    // (non-UTF-8) and `\nFRAME` decoy bytes to prove payload skipping.
    const D: &[u8] = b"YUV4MPEG2 W4 H2 F30000:1001 Ip A1:1 C420jpeg Xfoo\nFRAME\n************FRAME\n\xff\xfe\nFRAME\n********";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"YUV4MPEG"));
    }

    #[test]
    fn parses() {
        let y = parse(D).unwrap();
        assert_eq!(y.width, 4);
        assert_eq!(y.height, 2);
        assert_eq!(y.fps_num, 30000);
        assert_eq!(y.fps_den, 1001);
        assert_eq!(y.interlace, Some('p'));
        assert_eq!(y.aspect.as_deref(), Some("1:1"));
        assert_eq!(y.colorspace.as_deref(), Some("420jpeg"));
        assert_eq!(y.xyscss, None);
        assert_eq!(y.comments, vec!["foo".to_string()]);
        assert_eq!(y.frames, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"yuv4mpeg2 x").is_none());
        assert!(parse(b"").is_none());
    }
}
