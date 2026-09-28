//! MPEG-TS transport stream (ISO/IEC 13818-1): fixed packets with a
//! `0x47` sync byte every 188 bytes (or every 192/204 with a 4/16-byte
//! prefix in M2TS/FEC variants). Each packet is `47 |tei pusi pri| pid
//! (13b) |scram afc cc|`.
//!
//! ```
//! use izanagi_kit::mpegts::{detect, parse};
//!
//! // Three 188B packets: PAT on pid 0, then two on pid 0x100.
//! let mut d = Vec::new();
//! let mut pat = vec![0x47, 0x40, 0x00, 0x10]; // pusi=1 pid=0 afc=1
//! pat.extend_from_slice(&[0u8; 184]);
//! d.extend_from_slice(&pat);
//! for _ in 0..2 {
//!     let mut p = vec![0x47, 0x01, 0x00, 0x10]; // pid=0x100
//!     p.extend_from_slice(&[0xAB; 184]);
//!     d.extend_from_slice(&p);
//! }
//! assert!(detect(&d));
//! let t = parse(&d).unwrap();
//! assert_eq!(t.packets, 3);
//! assert_eq!(t.packet_size, 188);
//! assert_eq!(t.pat_sections, 1);
//! ```

use std::collections::BTreeSet;

/// Parsed MPEG-TS packet census.
#[derive(Debug, Clone, PartialEq)]
pub struct MpegTs {
    /// Stride between sync bytes: 188 (TS), 192 (M2TS) or 204 (FEC).
    pub packet_size: u32,
    /// Leading per-packet prefix bytes (`packet_size - 188`).
    pub prefix_bytes: u32,
    /// Complete packets parsed.
    pub packets: u32,
    /// Trailing bytes left over after the last full packet.
    pub remainder: u32,
    /// Distinct PIDs seen.
    pub pids: u32,
    /// Packets with the payload-unit-start indicator set.
    pub pusi: u32,
    /// Packets whose PID is 0 (PAT) *and* `pusi` — one per section.
    pub pat_sections: u32,
    /// Packets flagged `transport_error_indicator`.
    pub error_packets: u32,
    /// Packets with non-zero scrambling control.
    pub scrambled: u32,
    /// Packets carrying an adaptation field (`afc` 2 or 3).
    pub with_adaptation: u32,
    /// Bytes that broke sync inside the packet region.
    pub desyncs: u32,
}

/// `true` when `0x47` repeats at a 188/192/204 stride.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    stride_of(b).is_some()
}

fn stride_of(b: &[u8]) -> Option<usize> {
    for &(stride, off) in &[(188usize, 0usize), (192, 4), (204, 0)] {
        let sync_at = |k: usize| b.get(off + k * stride) == Some(&0x47);
        if b.len() > off + 2 * stride && sync_at(0) && sync_at(1) && sync_at(2) {
            return Some(stride);
        }
    }
    None
}

/// Walks the packet region; `None` when no stride is found.
#[must_use]
pub fn parse(b: &[u8]) -> Option<MpegTs> {
    let stride = stride_of(b)?;
    let off = if stride == 192 { 4 } else { 0 };
    let mut t = MpegTs {
        packet_size: stride as u32,
        prefix_bytes: (stride - 188) as u32,
        packets: 0,
        remainder: 0,
        pids: 0,
        pusi: 0,
        pat_sections: 0,
        error_packets: 0,
        scrambled: 0,
        with_adaptation: 0,
        desyncs: 0,
    };
    let mut pids = BTreeSet::new();
    let mut pos = off;
    // the packet is always 188 bytes at the sync point; the stride
    // carries any record prefix (192) or FEC trailer (204)
    while pos + 188 <= b.len() {
        let p = &b[pos..pos + 188];
        if p[0] != 0x47 {
            t.desyncs += 1;
            pos += 1;
            continue;
        }
        t.packets += 1;
        if p[1] & 0x80 != 0 {
            t.error_packets += 1;
        }
        let pusi = p[1] & 0x40 != 0;
        if pusi {
            t.pusi += 1;
        }
        let pid = (u16::from(p[1] & 0x1F) << 8) | u16::from(p[2]);
        pids.insert(pid);
        if pid == 0 && pusi {
            t.pat_sections += 1;
        }
        let afc = (p[3] >> 4) & 0x3;
        if afc >= 2 {
            t.with_adaptation += 1;
        }
        if p[3] & 0xC0 != 0 {
            t.scrambled += 1;
        }
        pos += stride;
    }
    t.pids = u32::try_from(pids.len()).unwrap_or(u32::MAX);
    t.remainder = u32::try_from(b.len().saturating_sub(pos)).unwrap_or(u32::MAX);
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        let mut pat = vec![0x47, 0x40, 0x00, 0x10];
        pat.extend_from_slice(&[0u8; 184]);
        d.extend_from_slice(&pat);
        for i in 0..2 {
            // packet 2 carries an adaptation field (afc=3)
            let mut p = vec![0x47, 0x01, 0x00, if i == 0 { 0x30 } else { 0x10 }];
            p.extend_from_slice(&[0xAB; 184]);
            d.extend_from_slice(&p);
        }
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        // 192-byte stride with 4-byte prefix.
        let mut m = Vec::new();
        for _ in 0..3 {
            m.extend_from_slice(&[0, 0, 0, 0]);
            let mut p = vec![0x47, 0x40, 0x00, 0x10];
            p.extend_from_slice(&[0u8; 184]);
            m.extend_from_slice(&p);
        }
        assert!(detect(&m));
        assert!(!detect(&[0x47; 300]));
        assert!(!detect(b"no sync here"));
    }

    #[test]
    fn parses() {
        let t = parse(&fixture()).unwrap();
        assert_eq!(t.packet_size, 188);
        assert_eq!(t.prefix_bytes, 0);
        assert_eq!(t.packets, 3);
        assert_eq!(t.pids, 2);
        assert_eq!(t.pusi, 1);
        assert_eq!(t.pat_sections, 1);
        assert_eq!(t.error_packets, 0);
        assert_eq!(t.scrambled, 0);
        assert_eq!(t.with_adaptation, 1);
        assert_eq!(t.remainder, 0);
        assert_eq!(t.desyncs, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"xx").is_none());
        assert!(parse(&[0x47; 188]).is_none());
    }
}
