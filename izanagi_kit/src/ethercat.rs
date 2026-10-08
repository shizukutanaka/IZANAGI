//! EtherCAT datagram chain inside an Ethernet II frame (EtherType 0x88A4).
//!
//! Frame: ethernet(14) `len:11 res:1 type:4` then `len` bytes of
//! concatenated datagrams, each `cmd idx addr:u32(LE) len:11 r:3 c:1 m:1
//! irq:u16 data[len] wkc:u16`.
//!
//! ```
//! use izanagi_kit::ethercat::parse;
//!
//! let mut f = vec![0u8; 14];
//! f[12] = 0x88; f[13] = 0xA4;
//! // one datagram: cmd=1(APRD) idx=0 addr=0 len=0 irq=0 wkc=0 -> 12 bytes
//! let dg = [1u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
//! let elen = dg.len() as u16;
//! f.extend_from_slice(&[(elen & 0xFF) as u8, ((elen >> 8) as u8) | (1 << 4)]);
//! f.extend_from_slice(&dg);
//! let e = parse(&f).unwrap();
//! assert_eq!(e.datagrams.len(), 1);
//! assert_eq!(e.datagrams[0].cmd, 1);
//! ```

/// One EtherCAT datagram header.
#[derive(Debug)]
pub struct Datagram {
    /// Command byte (APR D/WR, LRW, ...).
    pub cmd: u8,
    /// Index byte.
    pub idx: u8,
    /// 32-bit address (LE).
    pub addr: u32,
    /// Payload byte length (11-bit field).
    pub len: usize,
    /// Circulated flag (datagram traversed all slaves).
    pub circulated: bool,
    /// More datagrams follow flag.
    pub more: bool,
    /// Offset of the datagram payload.
    pub data_at: usize,
    /// Working counter (u16, LE) trailing the payload.
    pub wkc: u16,
}

/// Parsed EtherCAT header of an Ethernet frame.
#[derive(Debug)]
pub struct EtherCat {
    /// Byte offset of the first datagram.
    pub datagrams_at: usize,
    /// Parsed datagram headers.
    pub datagrams: Vec<Datagram>,
}

/// Parses an Ethernet II frame carrying EtherCAT (ethertype 0x88A4).
pub fn parse(d: &[u8]) -> Option<EtherCat> {
    if d.len() < 16 || d[12] != 0x88 || d[13] != 0xA4 {
        return None;
    }
    // header u16: len[10:0], res[11], type[15:12]; type 1 = datagrams
    let hdr = (d[14] as u16) | ((d[15] as u16) << 8);
    let elen = (hdr & 0x07FF) as usize;
    let etype = hdr >> 12;
    if etype != 1 || 16 + elen > d.len() {
        return None;
    }
    let end = 16 + elen;
    let mut datagrams = Vec::new();
    let mut at = 16;
    while at < end {
        if at + 10 > end {
            return None;
        }
        let cmd = d[at];
        let idx = d[at + 1];
        let addr = (d[at + 2] as u32)
            | ((d[at + 3] as u32) << 8)
            | ((d[at + 4] as u32) << 16)
            | ((d[at + 5] as u32) << 24);
        let lfield = (d[at + 6] as u16) | ((d[at + 7] as u16) << 8);
        let len = (lfield & 0x07FF) as usize;
        let circulated = lfield & 0x4000 != 0;
        let more = lfield & 0x8000 != 0;
        let data_at = at + 10;
        if data_at + len + 2 > end {
            return None;
        }
        let wkc_at = data_at + len;
        let wkc = (d[wkc_at] as u16) | ((d[wkc_at + 1] as u16) << 8);
        datagrams.push(Datagram {
            cmd,
            idx,
            addr,
            len,
            circulated,
            more,
            data_at,
            wkc,
        });
        at = wkc_at + 2;
    }
    Some(EtherCat {
        datagrams_at: 16,
        datagrams,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(dgs: &[&[u8]]) -> Vec<u8> {
        let mut f = vec![0u8; 14];
        f[12] = 0x88;
        f[13] = 0xA4;
        let elen: usize = dgs.iter().map(|d| d.len()).sum();
        f.push((elen & 0xFF) as u8);
        f.push(((elen >> 8) as u8) | (1 << 4));
        for d in dgs {
            f.extend_from_slice(d);
        }
        f
    }

    fn dg(cmd: u8, data: &[u8], more: bool, wkc: u16) -> Vec<u8> {
        let mut v = vec![cmd, 0x42, 0x11, 0x22, 0x33, 0x44];
        let mut lf = data.len() as u16;
        if more {
            lf |= 0x8000;
        }
        v.extend_from_slice(&[(lf & 0xFF) as u8, (lf >> 8) as u8]);
        v.extend_from_slice(&[0, 0]); // irq
        v.extend_from_slice(data);
        v.extend_from_slice(&[(wkc & 0xFF) as u8, (wkc >> 8) as u8]);
        v
    }

    #[test]
    fn chain() {
        let d1 = dg(1, &[0xAA, 0xBB], true, 3);
        let d2 = dg(4, &[], false, 1);
        let f = frame(&[&d1, &d2]);
        let e = parse(&f).unwrap();
        assert_eq!(e.datagrams.len(), 2);
        assert_eq!(e.datagrams[0].cmd, 1);
        assert_eq!(e.datagrams[0].idx, 0x42);
        assert_eq!(e.datagrams[0].addr, 0x44332211);
        assert_eq!(e.datagrams[0].len, 2);
        assert!(e.datagrams[0].more);
        assert_eq!(e.datagrams[0].wkc, 3);
        assert!(!e.datagrams[1].more);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0; 14]).is_none()); // wrong ethertype
                                            // etype != 1
        let mut f = vec![0u8; 14];
        f[12] = 0x88;
        f[13] = 0xA4;
        f.extend_from_slice(&[0, 0x20]); // type=2
        assert!(parse(&f).is_none());
        // truncated datagram
        let d = dg(1, &[0xAA], false, 0);
        let mut f = frame(&[&d]);
        f.truncate(f.len() - 1);
        assert!(parse(&f).is_none());
    }
}
