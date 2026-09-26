//! BACnet/IP BVLL frame + NPDU/APDU header parsing.
//!
//! ```
//! use izanagi_kit::bacnet::{parse, PduType};
//!
//! // BVLL original-unicast (0x0A), len 10, NPDU ver 1 ctrl 0, APDU confirmed-req
//! let f = [0x81u8, 0x0A, 0, 10, 1, 0, 0x00, 0x0C, 0, 0];
//! let b = parse(&f).unwrap();
//! assert_eq!(b.pdu_type(), Some(PduType::ConfirmedRequest));
//! ```

/// BACnet/IP frame: BVLC header, NPDU offsets, and the first APDU byte.
pub struct Bacnet {
    /// BVLC function/type byte (0x0A original-unicast, 0x0B broadcast, ...).
    pub bvlc_type: u8,
    /// Total BVLL-declared length (header included).
    pub total_len: usize,
    /// Offset of the NPDU (right after the 4-byte BVLC header), if present.
    pub npdu_at: Option<usize>,
    /// Offset of the APDU (past the 2-byte NPDU header), if present.
    pub apdu_at: Option<usize>,
    /// First APDU byte (`0` when absent).
    pub apdu_first: u8,
}

impl Bacnet {
    /// APDU PDU type from the upper nibble of the first APDU byte.
    pub fn pdu_type(&self) -> Option<PduType> {
        self.apdu_at?;
        PduType::from_nibble(self.apdu_first >> 4)
    }
}

/// APDU PDU types (upper nibble of the first APDU octet).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PduType {
    /// 0x0 — confirmed request.
    ConfirmedRequest,
    /// 0x1 — unconfirmed request.
    UnconfirmedRequest,
    /// 0x2 — simple ACK.
    SimpleAck,
    /// 0x3 — complex ACK.
    ComplexAck,
    /// 0x4 — segment ACK.
    SegmentAck,
    /// 0x5 — error PDU.
    Error,
    /// 0x6 — reject PDU.
    Reject,
    /// 0x7 — abort PDU.
    Abort,
    /// Other / unassigned nibble.
    Other(u8),
}

impl PduType {
    /// Maps the upper nibble to a PDU type.
    pub fn from_nibble(n: u8) -> Option<PduType> {
        Some(match n {
            0 => PduType::ConfirmedRequest,
            1 => PduType::UnconfirmedRequest,
            2 => PduType::SimpleAck,
            3 => PduType::ComplexAck,
            4 => PduType::SegmentAck,
            5 => PduType::Error,
            6 => PduType::Reject,
            7 => PduType::Abort,
            o => PduType::Other(o),
        })
    }
}

/// Parses a BACnet/IP frame: `0x81 type len:u16` then optional
/// NPDU (`version(1)=0x01 control(1)`) and APDU.
pub fn parse(d: &[u8]) -> Option<Bacnet> {
    if d.len() < 4 || d[0] != 0x81 {
        return None;
    }
    let total = (((d[2] as u16) << 8) | d[3] as u16) as usize;
    if total < 4 || total != d.len() {
        return None;
    }
    let mut npdu_at = None;
    let mut apdu_at = None;
    let mut apdu_first = 0;
    // BVLC result / write-broadcast / forwarded frames carry no NPDU.
    if matches!(d[1], 0x04 | 0x09 | 0x0A | 0x0B) && total > 4 {
        npdu_at = Some(4);
        // NPDU: version must be 0x01; control byte follows; hop count may trail.
        if d[4] != 0x01 {
            return None;
        }
        if total > 6 {
            apdu_at = Some(6);
            apdu_first = d[6];
        }
    }
    Some(Bacnet {
        bvlc_type: d[1],
        total_len: total,
        npdu_at,
        apdu_at,
        apdu_first,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_unicast() {
        let f = [0x81, 0x0A, 0, 10, 1, 0, 0x30, 2, 1, 0];
        let b = parse(&f).unwrap();
        assert_eq!(b.bvlc_type, 0x0A);
        assert_eq!(b.total_len, 10);
        assert_eq!(b.npdu_at, Some(4));
        assert_eq!(b.apdu_at, Some(6));
        assert_eq!(b.pdu_type(), Some(PduType::ComplexAck));
    }

    #[test]
    fn nibble_map() {
        assert_eq!(PduType::from_nibble(0), Some(PduType::ConfirmedRequest));
        assert_eq!(PduType::from_nibble(7), Some(PduType::Abort));
        assert_eq!(PduType::from_nibble(9), Some(PduType::Other(9)));
    }

    #[test]
    fn rejects() {
        // not BVLL
        assert!(parse(&[0x80, 0x0A, 0, 4]).is_none());
        // length mismatch
        assert!(parse(&[0x81, 0x0A, 0, 8, 1, 0]).is_none());
        // bad NPDU version
        assert!(parse(&[0x81, 0x0A, 0, 6, 2, 0]).is_none());
        // result frame with no NPDU is fine
        let b = parse(&[0x81, 0x00, 0, 6, 0, 0]).unwrap();
        assert!(b.npdu_at.is_none());
        assert!(b.pdu_type().is_none());
    }
}
