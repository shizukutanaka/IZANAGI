//! FLAC metadata: the `fLaC` marker then the metadata-block chain
//! (`STREAMINFO`, `PADDING`, `APPLICATION`, `SEEKTABLE`,
//! `VORBIS_COMMENT`, `CUESHEET`, `PICTURE`) — each block is a
//! last-flag + type byte + BE24 length. `streaminfo` unpacks the
//! 64-bit sample-rate/channels/bps/samples field; `comments`
//! decodes Vorbis comments. Audio frames themselves are out of
//! scope.
//!
//! ```
//! use izanagi_kit::flac;
//! let mut f = b"fLaC".to_vec();
//! // STREAMINFO, last=1, len=34 — all-zero payload parses to 0s
//! f.extend_from_slice(&[0x80, 0, 0, 34]);
//! f.extend_from_slice(&[0; 34]);
//! let fl = flac::parse(&f).unwrap();
//! assert_eq!(fl.blocks.len(), 1);
//! assert_eq!(fl.blocks[0].name(), "streaminfo");
//! let si = flac::streaminfo(&f, &fl).unwrap();
//! assert_eq!(si.sample_rate, 0);
//! ```

fn r16(d: &[u8], at: usize) -> Option<u32> {
    Some((*d.get(at)? as u32) << 8 | *d.get(at + 1)? as u32)
}
fn r24(d: &[u8], at: usize) -> Option<u32> {
    Some(r16(d, at)? << 8 | *d.get(at + 2)? as u32)
}
fn rl32(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at + 4)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// One metadata block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// Block type code (0=STREAMINFO, …, 6=PICTURE).
    pub ty: u8,
    /// File offset of the payload.
    pub offset: usize,
    /// Payload size.
    pub size: usize,
    /// Whether the header's last-block flag was set.
    pub last: bool,
}

impl Block {
    /// Canonical block-type name.
    pub fn name(&self) -> &'static str {
        match self.ty {
            0 => "streaminfo",
            1 => "padding",
            2 => "application",
            3 => "seektable",
            4 => "vorbis-comment",
            5 => "cuesheet",
            6 => "picture",
            _ => "unknown",
        }
    }
}

/// A parsed FLAC metadata region.
#[derive(Clone, Debug)]
pub struct Flac {
    /// Metadata blocks in file order (`audio_offset` starts after).
    pub blocks: Vec<Block>,
    /// File offset of the first audio frame.
    pub audio_offset: usize,
}

/// Unpacked STREAMINFO.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamInfo {
    /// Minimum block size (samples).
    pub min_block: u16,
    /// Maximum block size.
    pub max_block: u16,
    /// Minimum frame size (bytes, 0 = unknown).
    pub min_frame: u32,
    /// Maximum frame size.
    pub max_frame: u32,
    /// Sample rate in Hz (≤ 655350).
    pub sample_rate: u32,
    /// Channel count (1–8).
    pub channels: u8,
    /// Bits per sample (4–32).
    pub bits_per_sample: u8,
    /// Total inter-channel samples (36-bit).
    pub total_samples: u64,
    /// MD5 of the unencoded audio (all-zero = unset).
    pub md5: [u8; 16],
}

/// Parse `fLaC` + the metadata chain. Rejects a missing marker, a
/// first block that isn't STREAMINFO (required by the spec), a
/// truncated block, or block type 127 (forbidden).
pub fn parse(d: &[u8]) -> Option<Flac> {
    if d.get(0..4)? != b"fLaC" {
        return None;
    }
    let mut blocks = Vec::new();
    let mut i = 4usize;
    loop {
        let h = *d.get(i)?;
        let last = h & 0x80 != 0;
        let ty = h & 0x7F;
        if ty == 127 {
            return None;
        }
        let size = r24(d, i + 1)? as usize;
        let offset = i + 4;
        if offset.checked_add(size)? > d.len() {
            return None;
        }
        if blocks.is_empty() && ty != 0 {
            return None; // first block must be STREAMINFO
        }
        blocks.push(Block {
            ty,
            offset,
            size,
            last,
        });
        i = offset + size;
        if last {
            break;
        }
    }
    Some(Flac {
        blocks,
        audio_offset: i,
    })
}

/// Decode the first STREAMINFO block.
pub fn streaminfo(d: &[u8], f: &Flac) -> Option<StreamInfo> {
    let b = f.blocks.first().filter(|b| b.ty == 0)?;
    if b.size < 34 || b.offset.checked_add(34)? > d.len() {
        return None;
    }
    let at = b.offset;
    let min_block = r16(d, at)? as u16;
    let max_block = r16(d, at + 2)? as u16;
    let min_frame = r24(d, at + 4)?;
    let max_frame = r24(d, at + 7)?;
    // packed 64-bit field at +10..+18
    let mut packed = 0u64;
    for i in 0..8 {
        packed = (packed << 8) | *d.get(at + 10 + i)? as u64;
    }
    let sample_rate = ((packed >> 44) & 0xFFFFF) as u32;
    let channels = (((packed >> 41) & 7) + 1) as u8;
    let bits_per_sample = (((packed >> 36) & 0x1F) + 1) as u8;
    let total_samples = packed & 0xF_FFFF_FFFF;
    let mut md5 = [0u8; 16];
    md5.copy_from_slice(d.get(at + 18..at + 34)?);
    Some(StreamInfo {
        min_block,
        max_block,
        min_frame,
        max_frame,
        sample_rate,
        channels,
        bits_per_sample,
        total_samples,
        md5,
    })
}

/// Vorbis comment block decoded into `(vendor, [(key, value)])`.
/// Key=value pairs are UTF-8; a malformed pair is skipped rather
/// than failing the whole block.
pub fn comments(d: &[u8], f: &Flac) -> Option<(String, Vec<(String, String)>)> {
    let b = f.blocks.iter().find(|b| b.ty == 4)?;
    let at = b.offset;
    let vlen = rl32(d, at)? as usize;
    let vend = at.checked_add(4)?.checked_add(vlen)?;
    let vendor = std::str::from_utf8(d.get(at + 4..vend)?).ok()?.to_string();
    let count = rl32(d, vend)? as usize;
    let mut i = vend + 4;
    let mut out = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        let n = rl32(d, i)? as usize;
        i = i.checked_add(4)?;
        let end = i.checked_add(n)?;
        if end > d.len() {
            break;
        }
        if let Ok(s) = std::str::from_utf8(&d[i..end]) {
            if let Some(eq) = s.find('=') {
                out.push((s[..eq].to_string(), s[eq + 1..].to_string()));
            }
        }
        i = end;
    }
    Some((vendor, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// fLaC + STREAMINFO(34) + VORBIS_COMMENT + PADDING(last).
    fn file() -> Vec<u8> {
        let mut f = b"fLaC".to_vec();
        f.extend_from_slice(&[0x00, 0, 0, 34]); // STREAMINFO
        let mut si = vec![0u8; 34];
        // min/max block 4096, packed: 44100/2ch/16bit/1000 samples
        si[0] = 0x10;
        si[2] = 0x10;
        let packed: u64 = (44100u64 << 44) | (1u64 << 41) | (15u64 << 36) | 1000;
        for i in 0..8 {
            si[10 + i] = (packed >> (56 - i * 8)) as u8;
        }
        f.extend_from_slice(&si);
        // VORBIS_COMMENT: vendor "v", comments ["TITLE=x","ARTIST=y"]
        let mut vc = Vec::new();
        vc.extend_from_slice(&[1, 0, 0, 0]);
        vc.extend_from_slice(b"v");
        vc.extend_from_slice(&[2, 0, 0, 0]);
        vc.extend_from_slice(&[7, 0, 0, 0]);
        vc.extend_from_slice(b"TITLE=x");
        vc.extend_from_slice(&[8, 0, 0, 0]);
        vc.extend_from_slice(b"ARTIST=y");
        f.extend_from_slice(&[
            0x04,
            (vc.len() >> 16) as u8,
            (vc.len() >> 8) as u8,
            vc.len() as u8,
        ]);
        f.extend_from_slice(&vc);
        f.extend_from_slice(&[0x81, 0, 0, 8]); // PADDING last, 8B
        f.extend_from_slice(&[0; 8]);
        f.extend_from_slice(&[0xFF, 0xF8]); // audio sync
        f
    }

    #[test]
    fn full_walk() {
        let d = file();
        let fl = parse(&d).unwrap();
        assert_eq!(fl.blocks.len(), 3);
        assert_eq!(fl.blocks[1].name(), "vorbis-comment");
        assert_eq!(d[fl.audio_offset], 0xFF);
        let si = streaminfo(&d, &fl).unwrap();
        assert_eq!(si.min_block, 0x1000);
        assert_eq!(si.sample_rate, 44100);
        assert_eq!(si.channels, 2);
        assert_eq!(si.bits_per_sample, 16);
        assert_eq!(si.total_samples, 1000);
        let (vendor, kv) = comments(&d, &fl).unwrap();
        assert_eq!(vendor, "v");
        assert_eq!(
            kv,
            vec![
                ("TITLE".to_string(), "x".to_string()),
                ("ARTIST".to_string(), "y".to_string())
            ]
        );
    }

    #[test]
    fn malformed_rejected() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"fLaX").is_none());
        let mut f = file();
        f[4] = 0x01; // first block must be STREAMINFO
        assert!(parse(&f).is_none());
        let mut g = file();
        g[4] = 0xFF; // forbidden type 127
        assert!(parse(&g).is_none());
        let mut h = file();
        h[6] = 0xFF; // length overruns file
        assert!(parse(&h).is_none());
        let mut t = file();
        t.truncate(t.len() - 4); // mid-block truncation
        assert!(parse(&t).is_none());
    }

    #[test]
    fn missing_last_block_is_none() {
        // last flag never set → parse hits EOF and returns None
        let mut f = b"fLaC".to_vec();
        f.extend_from_slice(&[0x00, 0, 0, 34]);
        f.extend_from_slice(&[0; 34]);
        assert!(parse(&f).is_none());
    }
}
