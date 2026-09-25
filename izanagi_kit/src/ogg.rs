//! Ogg container pages: `OggS` + version + flags + granule +
//! serial + sequence + CRC + lacing table. `ogg` walks pages and
//! reassembles packets (a packet ends at a segment `< 255`, or at
//! page end when continued); the Ogg CRC-32 (poly `0x04C11DB7`,
//! non-reflected, init 0, no final XOR) is verified — not the
//! IEEE variant `crc` provides.
//!
//! ```
//! use izanagi_kit::ogg;
//! // one 2-byte segment, payload "hi"
//! let p = ogg::emit_page(2 /* BOS */, 0, 0x77, 0, &[2], b"hi");
//! let pages = ogg::pages(&p).unwrap();
//! assert_eq!(pages.len(), 1);
//! assert_eq!(pages[0].serial, 0x77);
//! assert_eq!(ogg::packets(&p).unwrap(), vec![b"hi".to_vec()]);
//! ```

fn rl32(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at + 4)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}
fn rl64(d: &[u8], at: usize) -> Option<u64> {
    let mut v = 0u64;
    for i in 0..8 {
        v |= (*d.get(at + i)? as u64) << (i * 8);
    }
    Some(v)
}

/// Ogg page CRC-32 (Vorbis spec Appendix B — MSB-first poly).
fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    for (i, e) in t.iter_mut().enumerate() {
        let mut c = (i as u32) << 24;
        for _ in 0..8 {
            c = if c & 0x8000_0000 != 0 {
                (c << 1) ^ 0x04C1_1DB7
            } else {
                c << 1
            };
        }
        *e = c;
    }
    t
}

/// Ogg CRC-32 over `data` (no reflection, init 0, no final XOR).
pub fn ogg_crc(data: &[u8]) -> u32 {
    let t = crc_table();
    let mut c = 0u32;
    for &b in data {
        c = (c << 8) ^ t[((c >> 24) as u8 ^ b) as usize];
    }
    c
}

/// One Ogg page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    /// Header type flags: bit0 continued, bit1 BOS, bit2 EOS.
    pub flags: u8,
    /// Granule position (codec-defined; -1 = unset).
    pub granule: u64,
    /// Bitstream serial number.
    pub serial: u32,
    /// Page sequence number.
    pub seq: u32,
    /// File offset of the payload.
    pub offset: usize,
    /// Payload length (sum of the lacing table).
    pub size: usize,
    /// Lacing values (each ≤ 255).
    pub segments: Vec<u8>,
}

impl Page {
    /// Page is a continuation of a packet from the previous page.
    pub fn continued(&self) -> bool {
        self.flags & 1 != 0
    }
    /// Beginning-of-stream page.
    pub fn bos(&self) -> bool {
        self.flags & 2 != 0
    }
    /// End-of-stream page.
    pub fn eos(&self) -> bool {
        self.flags & 4 != 0
    }
}

/// Walk Ogg pages; `None` on bad magic/version, CRC mismatch, or
/// truncation. Each page's full extent is validated.
pub fn pages(d: &[u8]) -> Option<Vec<Page>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        if d.get(i..i + 4)? != b"OggS" {
            return None;
        }
        if d.get(i + 4)? != &0u8 {
            return None; // version must be 0
        }
        let flags = *d.get(i + 5)?;
        let granule = rl64(d, i + 6)?;
        let serial = rl32(d, i + 14)?;
        let seq = rl32(d, i + 18)?;
        let crc = rl32(d, i + 22)?;
        let nsegs = *d.get(i + 26)? as usize;
        let segs_start = i + 27;
        let seg_bytes = d.get(segs_start..segs_start.checked_add(nsegs)?)?;
        let size: usize = seg_bytes.iter().map(|&s| s as usize).sum();
        let offset = segs_start.checked_add(nsegs)?;
        let end = offset.checked_add(size)?;
        if end > d.len() {
            return None;
        }
        // CRC covers the whole page with the CRC field zeroed
        let mut page_img = d[i..end].to_vec();
        page_img[22..26].fill(0);
        if ogg_crc(&page_img) != crc {
            return None;
        }
        out.push(Page {
            flags,
            granule,
            serial,
            seq,
            offset,
            size,
            segments: seg_bytes.to_vec(),
        });
        i = end;
    }
    Some(out)
}

/// Reassemble packets across pages: a packet ends at a lacing value
/// `< 255` (a 255-multiple payload ends with a `0` lacing byte).
/// `None` on a malformed page stream; a page flagged `continued`
/// appends to the pending packet.
pub fn packets(d: &[u8]) -> Option<Vec<Vec<u8>>> {
    let ps = pages(d)?;
    let mut out = Vec::new();
    let mut cur: Vec<u8> = Vec::new();
    let mut pending = false;
    for p in &ps {
        if !p.continued() && pending {
            // dangling partial packet — corrupt stream
            return None;
        }
        let mut at = p.offset;
        for &seg in &p.segments {
            let n = seg as usize;
            cur.extend_from_slice(d.get(at..at.checked_add(n)?)?);
            at += n;
            if seg < 255 {
                out.push(std::mem::take(&mut cur));
                pending = false;
            } else {
                pending = true;
            }
        }
    }
    if pending {
        return None;
    }
    Some(out)
}

/// Emit one page: `segments` = the lacing table (⌊n/255⌋ 255s then
/// n%255 per packet), `payload` = the laced bytes. CRC is computed
/// and inserted.
pub fn emit_page(
    flags: u8,
    granule: u64,
    serial: u32,
    seq: u32,
    segments: &[u8],
    payload: &[u8],
) -> Vec<u8> {
    let mut out = b"OggS".to_vec();
    out.push(0);
    out.push(flags);
    for i in 0..8 {
        out.push((granule >> (i * 8)) as u8);
    }
    for i in 0..4 {
        out.push((serial >> (i * 8)) as u8);
    }
    for i in 0..4 {
        out.push((seq >> (i * 8)) as u8);
    }
    out.extend_from_slice(&[0; 4]); // CRC placeholder
    out.push(segments.len() as u8);
    out.extend_from_slice(segments);
    out.extend_from_slice(payload);
    let crc = ogg_crc(&out);
    for i in 0..4 {
        out[22 + i] = (crc >> (i * 8)) as u8;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_page_roundtrip() {
        let p = emit_page(2, 0x1122, 0x77, 0, &[2], b"hi");
        let ps = pages(&p).unwrap();
        assert_eq!(ps.len(), 1);
        assert!(ps[0].bos());
        assert!(!ps[0].continued());
        assert!(!ps[0].eos());
        assert_eq!(ps[0].granule, 0x1122);
        assert_eq!(ps[0].serial, 0x77);
        assert_eq!(packets(&p).unwrap(), vec![b"hi".to_vec()]);
        // the Ogg CRC is *not* the IEEE crc32: pinned independent value
        assert_eq!(ogg_crc(b"OggS"), 0x5FB0_A94F);
        let e = emit_page(1 | 4, 0, 0x77, 1, &[2], b"hi");
        let es = pages(&e).unwrap();
        assert!(es[0].continued() && es[0].eos());
    }

    #[test]
    fn crc_is_verified() {
        let mut p = emit_page(0, 0, 1, 0, &[2], b"hi");
        let n = p.len();
        p[n - 1] ^= 0xFF; // corrupt a payload byte
        assert!(pages(&p).is_none());
        // fix corruption, corrupt the CRC field itself
        p[n - 1] ^= 0xFF;
        p[22] ^= 0x01;
        assert!(pages(&p).is_none());
    }

    #[test]
    fn spanning_packet_reassembles() {
        // packet: 300 bytes = page1's one 255 segment + page2's 45-byte tail
        let body: Vec<u8> = (0..300).map(|i| (i % 251) as u8).collect();
        let mut d = emit_page(2, 0, 7, 0, &[255], &body[..255]);
        d.extend_from_slice(&emit_page(1, 300, 7, 1, &[45], &body[255..])); // continued
        assert_eq!(packets(&d).unwrap(), vec![body.clone()]);
        // without the continued flag the stream is corrupt
        let mut d2 = emit_page(2, 0, 7, 0, &[255], &body[..255]);
        d2.extend_from_slice(&emit_page(0, 300, 7, 1, &[45], &body[255..]));
        assert!(packets(&d2).is_none());
    }

    #[test]
    fn malformed_rejected() {
        assert_eq!(pages(&[]), Some(vec![])); // empty stream is valid
        assert!(pages(b"OggX").is_none());
        let mut p = emit_page(0, 0, 1, 0, &[2], b"hi");
        p[4] = 1; // version must be 0
        assert!(pages(&p).is_none());
        let p2 = emit_page(0, 0, 1, 0, &[2], b"hi");
        assert!(pages(&p2[..p2.len() - 1]).is_none()); // truncated
    }
}
