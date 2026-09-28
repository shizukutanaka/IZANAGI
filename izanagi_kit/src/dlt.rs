//! AUTOSAR DLT message (Diagnostic Log and Trace): standard header
//! `htyp` (UEH 0x01, MSBF 0x02, WEID 0x04, WSID 0x08, WTMS 0x10,
//! version 0x20) + `mcnt` + `len` u16 BE covering the whole message
//! (header + payload), then optional `ecu` 4B, `seid` 4B, `tms` u32,
//! then — when UEH — a 10-byte extended header `msin`/`noar`/`apid`/
//! `ctid`. A stream is `parse`d record by record.
//!
//! ```
//! // verbose log message, ECU "ECU1", APID "APP" CTID "CTX"
//! let mut d = vec![0x25, 0x00, 0x00, 22]; // htyp(UEH|WEID|v1) mcnt len
//! d.extend_from_slice(b"ECU1");
//! d.extend_from_slice(&[0x01, 1, b'A', b'P', b'P', 0, b'C', b'T', b'X', 0]);
//! d.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]); // payload
//! let m = izanagi_kit::dlt::parse(&d).unwrap();
//! assert_eq!(m.len(), 1);
//! assert_eq!(m[0].ecu, Some(*b"ECU1"));
//! assert_eq!(m[0].apid, Some(*b"APP\0"));
//! ```

use std::vec::Vec;

/// htyp bits.
pub const HTYP_UEH: u8 = 0x01;
/// MSBF — payload is big-endian.
pub const HTYP_MSBF: u8 = 0x02;
/// WEID — ECU id field present.
pub const HTYP_WEID: u8 = 0x04;
/// WSID — session id field present.
pub const HTYP_WSID: u8 = 0x08;
/// WTMS — timestamp field present.
pub const HTYP_WTMS: u8 = 0x10;

/// One DLT message.
#[derive(Clone, Debug)]
pub struct Dlt {
    /// Message counter.
    pub mcnt: u8,
    /// ECU ID (when WEID).
    pub ecu: Option<[u8; 4]>,
    /// Session ID (when WSID).
    pub session: Option<u32>,
    /// Timestamp in 0.1 ms (when WTMS).
    pub timestamp: Option<u32>,
    /// Payload is big-endian (MSBF set).
    pub big_endian: bool,
    /// Verbose-mode message (extended header msin bit 0).
    pub verbose: bool,
    /// Application ID (when UEH).
    pub apid: Option<[u8; 4]>,
    /// Context ID (when UEH).
    pub ctid: Option<[u8; 4]>,
    /// Byte length of the message payload after all headers.
    pub payload_len: usize,
    /// Offset of the payload inside the input.
    pub payload_offset: usize,
}

fn u16be(d: &[u8], o: usize) -> Option<u16> {
    Some((u16::from(*d.get(o)?) << 8) | u16::from(*d.get(o + 1)?))
}
fn u32be(d: &[u8], o: usize) -> Option<u32> {
    let d = d.get(o..o + 4)?;
    Some(
        (u32::from(d[0]) << 24)
            | (u32::from(d[1]) << 16)
            | (u32::from(d[2]) << 8)
            | u32::from(d[3]),
    )
}

/// Parse a stream of DLT messages; `None` when any message has a bad
/// version nibble, `len < 4`, a missing optional field, or overruns
/// the buffer.
pub fn parse(d: &[u8]) -> Option<Vec<Dlt>> {
    let mut msgs = Vec::new();
    let mut off = 0usize;
    if d.is_empty() {
        return None;
    }
    while off < d.len() {
        if d.len() - off < 4 {
            return None;
        }
        let htyp = d[off];
        // version bits 5-7 must be 1 (DLTv1)
        if htyp >> 5 != 1 {
            return None;
        }
        let mcnt = d[off + 1];
        let len = u16be(d, off + 2)? as usize;
        if len < 4 || len > d.len() - off {
            return None;
        }
        let mut p = off + 4;
        let end = off + len;
        let ecu = if htyp & HTYP_WEID != 0 {
            if p + 4 > end {
                return None;
            }
            let e: [u8; 4] = d[p..p + 4].try_into().ok()?;
            p += 4;
            Some(e)
        } else {
            None
        };
        let session = if htyp & HTYP_WSID != 0 {
            if p + 4 > end {
                return None;
            }
            let s = u32be(d, p)?;
            p += 4;
            Some(s)
        } else {
            None
        };
        let timestamp = if htyp & HTYP_WTMS != 0 {
            if p + 4 > end {
                return None;
            }
            let t = u32be(d, p)?;
            p += 4;
            Some(t)
        } else {
            None
        };
        let (verbose, apid, ctid) = if htyp & HTYP_UEH != 0 {
            if p + 10 > end {
                return None;
            }
            let msin = d[p];
            (
                msin & 1 != 0,
                Some(d[p + 2..p + 6].try_into().ok()?),
                Some(d[p + 6..p + 10].try_into().ok()?),
            )
        } else {
            (false, None, None)
        };
        if htyp & HTYP_UEH != 0 {
            p += 10;
        }
        if p > end {
            return None;
        }
        msgs.push(Dlt {
            mcnt,
            ecu,
            session,
            timestamp,
            big_endian: htyp & HTYP_MSBF != 0,
            verbose,
            apid,
            ctid,
            payload_len: end - p,
            payload_offset: p,
        });
        off = end;
    }
    Some(msgs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(htyp: u8, extra: &[u8], payload: &[u8]) -> Vec<u8> {
        let len = 4 + extra.len() + payload.len();
        let mut v = vec![htyp, 0xab, (len >> 8) as u8, len as u8];
        v.extend_from_slice(extra);
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn full_header() {
        let mut extra = b"ECU9".to_vec();
        extra.extend_from_slice(&[0, 0, 0, 5]); // seid
        extra.extend_from_slice(&[0, 0, 0, 9]); // tms
        extra.extend_from_slice(&[0x20, 2, b'A', 0, 0, 0, b'C', 0, 0, 0]);
        let d = msg(
            0x20 | HTYP_UEH | HTYP_WEID | HTYP_WSID | HTYP_WTMS | HTYP_MSBF,
            &extra,
            &[1, 2],
        );
        let msgs = parse(&d).unwrap();
        assert_eq!(msgs.len(), 1);
        let m = &msgs[0];
        assert_eq!(m.mcnt, 0xab);
        assert_eq!(m.ecu, Some(*b"ECU9"));
        assert_eq!(m.session, Some(5));
        assert_eq!(m.timestamp, Some(9));
        assert!(m.big_endian);
        assert!(!m.verbose);
    }

    #[test]
    fn chained() {
        let mut d = msg(0x20 | HTYP_WEID, b"ABCD", &[0x55]);
        d.extend_from_slice(&msg(
            0x20 | HTYP_UEH,
            &[0x01, 0, b'A', b'P', b'P', 0, b'C', 0, 0, 0],
            &[7],
        ));
        let msgs = parse(&d).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].ecu, Some(*b"ABCD"));
        assert_eq!(msgs[0].apid, None);
        assert_eq!(msgs[1].apid, Some(*b"APP\0"));
        assert!(msgs[1].verbose);
        assert_eq!(msgs[1].payload_len, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x3d, 0, 0, 3]).is_none()); // len < 4
        let d = msg(0x20 | HTYP_WEID, b"EC", &[1]); // len covers only 2 ecu bytes
        assert!(parse(&d).is_none());
        let d = vec![0x00, 0, 0, 4]; // version 0
        assert!(parse(&d).is_none());
    }
}
