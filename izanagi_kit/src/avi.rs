//! RIFF AVI container walk (Microsoft AVI RIFF File Reference).
//!
//! AVI is a little-endian RIFF form: `RIFF <size32> 'AVI '`, then
//! `LIST`/`JUNK`/chunk triples. The `hdrl` list carries the `avih`
//! main header (56 bytes) and one `strl` list per stream, each with a
//! 56-byte `strh` stream header.
//!
//! ```
//! use izanagi_kit::avi::parse;
//! let mut d = b"RIFF".to_vec();
//! // see tests for a full fixture; here we only show the contract:
//! let _ = &mut d;
//! assert!(parse(&d).is_none()); // truncated RIFF
//! ```

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (*d.get(o)? as u32)
            | (*d.get(o + 1)? as u32) << 8
            | (*d.get(o + 2)? as u32) << 16
            | (*d.get(o + 3)? as u32) << 24,
    )
}

fn four(d: &[u8], o: usize) -> Option<[u8; 4]> {
    let mut v = [0u8; 4];
    v.copy_from_slice(d.get(o..o + 4)?);
    Some(v)
}

/// One stream's `strh` view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stream {
    /// `'vids'`, `'auds'`, `'txts'`, …
    pub kind: [u8; 4],
    /// Preferred handler / codec fourcc.
    pub handler: [u8; 4],
    /// Time scale (`rate / scale` = frames or samples per second).
    pub scale: u32,
    /// Rate numerator.
    pub rate: u32,
    /// Stream length in scale units.
    pub length: u32,
    /// Suggested decoder buffer size.
    pub suggested_buffer: u32,
}

/// Parsed AVI top-level fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Avi {
    /// Microseconds per frame from `avih`.
    pub usec_per_frame: u32,
    /// `avih` flags (`AVIF_HASINDEX` 0x10, `AVIF_ISINTERLEAVED` 0x100…).
    pub flags: u32,
    /// Total frames in the main video stream.
    pub total_frames: u32,
    /// Stream count declared by `avih`.
    pub declared_streams: u32,
    /// Suggested playback buffer.
    pub suggested_buffer: u32,
    /// Frame width.
    pub width: u32,
    /// Frame height.
    pub height: u32,
    /// Per-stream `strh` headers found inside `hdrl`.
    pub streams: Vec<Stream>,
    /// Offset of the `movi` list payload, when present.
    pub movi_at: Option<usize>,
}

fn walk_chunks(
    d: &[u8],
    mut at: usize,
    end: usize,
    mut f: impl FnMut(&[u8], usize, usize) -> bool,
) {
    while at + 8 <= end {
        if four(d, at).is_none() {
            break;
        }
        let size = match le32(d, at + 4) {
            Some(s) => s as usize,
            None => break,
        };
        let data = match at.checked_add(8) {
            Some(v) => v,
            None => break,
        };
        if data.checked_add(size).map_or(true, |e| e > end) {
            break;
        }
        if !f(d, at, data + size) {
            break;
        }
        at = data + size + (size & 1);
    }
}

/// Parse `RIFF … 'AVI '`, reading `avih` and each `strh` inside the
/// `hdrl` list and recording where `movi` starts. `None` on a bad
/// RIFF envelope, a missing `hdrl`/`avih`, or an overrun.
pub fn parse(d: &[u8]) -> Option<Avi> {
    if four(d, 0)? != *b"RIFF" {
        return None;
    }
    let end = 8usize.checked_add(le32(d, 4)? as usize)?;
    if end > d.len() || four(d, 8)? != *b"AVI " {
        return None;
    }
    let mut avih: Option<Avi> = None;
    let mut streams = Vec::new();
    let mut movi_at = None;
    walk_chunks(d, 12, end, |d, at, e| {
        let id = four(d, at).unwrap_or([0; 4]);
        let data = at + 8;
        if id == *b"LIST" && data + 4 <= e {
            let ty = four(d, data).unwrap_or([0; 4]);
            if ty == *b"hdrl" {
                walk_chunks(d, data + 4, e, |d, at, e2| {
                    let id = four(d, at).unwrap_or([0; 4]);
                    if id == *b"avih" && le32(d, at + 4).unwrap_or(0) >= 56 {
                        avih = Some(Avi {
                            usec_per_frame: le32(d, at + 8).unwrap_or(0),
                            flags: le32(d, at + 8 + 12).unwrap_or(0),
                            total_frames: le32(d, at + 8 + 16).unwrap_or(0),
                            declared_streams: le32(d, at + 8 + 24).unwrap_or(0),
                            suggested_buffer: le32(d, at + 8 + 28).unwrap_or(0),
                            width: le32(d, at + 8 + 32).unwrap_or(0),
                            height: le32(d, at + 8 + 36).unwrap_or(0),
                            streams: Vec::new(),
                            movi_at: None,
                        });
                    } else if id == *b"LIST" {
                        // sub-list: type word then chunks; find strh
                        let ty = four(d, at + 8).unwrap_or([0; 4]);
                        if ty == *b"strl" {
                            walk_chunks(d, at + 12, e2, |d, at, _e3| {
                                if four(d, at) == Some(*b"strh")
                                    && le32(d, at + 4).unwrap_or(0) >= 56
                                {
                                    let b = at + 8;
                                    streams.push(Stream {
                                        kind: four(d, b).unwrap_or([0; 4]),
                                        handler: four(d, b + 4).unwrap_or([0; 4]),
                                        scale: le32(d, b + 20).unwrap_or(0),
                                        rate: le32(d, b + 24).unwrap_or(0),
                                        length: le32(d, b + 32).unwrap_or(0),
                                        suggested_buffer: le32(d, b + 36).unwrap_or(0),
                                    });
                                }
                                true
                            });
                        }
                    }
                    true
                });
            } else if ty == *b"movi" {
                movi_at = Some(data + 4);
            }
        }
        true
    });
    let mut a = avih?;
    a.streams = streams;
    a.movi_at = movi_at;
    Some(a)
}

/// `true` when `avih` flags include `AVIF_ISINTERLEAVED` (0x00000100).
pub fn is_interleaved(a: &Avi) -> bool {
    a.flags & 0x0000_0100 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w32(d: &mut Vec<u8>, v: u32) {
        d.extend_from_slice(&[v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8]);
    }

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut d = id.to_vec();
        w32(&mut d, body.len() as u32);
        d.extend_from_slice(body);
        if body.len() & 1 == 1 {
            d.push(0);
        }
        d
    }

    fn list(ty: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut d = b"LIST".to_vec();
        w32(&mut d, (4 + body.len()) as u32);
        d.extend_from_slice(ty);
        d.extend_from_slice(body);
        d
    }

    fn fixture() -> Vec<u8> {
        let mut avih = Vec::new();
        w32(&mut avih, 33_333); // usec/frame ~30fps
        w32(&mut avih, 0);
        w32(&mut avih, 0);
        w32(&mut avih, 0x110); // flags: HASINDEX|ISINTERLEAVED
        w32(&mut avih, 300); // total frames
        w32(&mut avih, 0);
        w32(&mut avih, 2); // streams
        w32(&mut avih, 0);
        w32(&mut avih, 320);
        w32(&mut avih, 240);
        for _ in 0..4 {
            w32(&mut avih, 0);
        }

        let mut strh = Vec::new();
        strh.extend_from_slice(b"vids");
        strh.extend_from_slice(b"DIB ");
        w32(&mut strh, 0); // flags
        w32(&mut strh, 0); // priority+language
        w32(&mut strh, 0); // init frames
        w32(&mut strh, 1); // scale
        w32(&mut strh, 30); // rate
        w32(&mut strh, 0); // start
        w32(&mut strh, 300); // length
        w32(&mut strh, 76800); // suggested buffer
        w32(&mut strh, 0xFFFF_FFFF); // quality
        w32(&mut strh, 0); // sample size
        for _ in 0..4 {
            strh.extend_from_slice(&[0, 0]); // rcFrame i16s
        }

        let strl = list(b"strl", &chunk(b"strh", &strh));
        let mut hdrl_body = chunk(b"avih", &avih);
        hdrl_body.extend_from_slice(&strl);
        let hdrl = list(b"hdrl", &hdrl_body);
        let movi = list(b"movi", b"00dc\x04\0\0\0abcd");

        let mut riff_body = b"AVI ".to_vec();
        riff_body.extend_from_slice(&hdrl);
        riff_body.extend_from_slice(&movi);
        let mut d = b"RIFF".to_vec();
        w32(&mut d, riff_body.len() as u32);
        d.extend_from_slice(&riff_body);
        d
    }

    #[test]
    fn fields_and_streams() {
        let d = fixture();
        let a = parse(&d).unwrap();
        assert_eq!(a.usec_per_frame, 33_333);
        assert_eq!(a.total_frames, 300);
        assert_eq!(a.declared_streams, 2);
        assert_eq!((a.width, a.height), (320, 240));
        assert!(is_interleaved(&a));
        assert_eq!(a.streams.len(), 1);
        let s = a.streams[0];
        assert_eq!(s.kind, *b"vids");
        assert_eq!(s.handler, *b"DIB ");
        assert_eq!((s.scale, s.rate, s.length), (1, 30, 300));
        assert!(a.movi_at.is_some());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"RIFF\0\0\0\0WAVE").is_none());
        let mut d = fixture();
        d[7] = 0xFF; // size overrun
        assert!(parse(&d).is_none());
        // no avih → None
        let mut d2 = b"RIFF".to_vec();
        w32(&mut d2, 4 + 8 + 4);
        d2.extend_from_slice(b"AVI ");
        d2.extend_from_slice(&list(b"INFO", b"strz"));
        assert!(parse(&d2).is_none());
    }
}
