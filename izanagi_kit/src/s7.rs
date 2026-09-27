//! Siemens S7comm header parsing over TPKT + COTP.
//!
//! Layout: TPKT `03 00 len:u16`, then COTP DT `02 F0 80`
//! (or a connection-confirm `ED ..`), then S7 header starting `0x32`.
//!
//! ```
//! use izanagi_kit::s7::{parse, Rosctr};
//!
//! // TPKT(19) + COTP DT + S7 ack-data header (10B param 0, data 2)
//! let f = [
//!     0x03u8, 0x00, 0x00, 19, // TPKT len 19
//!     0x02, 0xF0, 0x80,       // COTP data
//!     0x32, 3, 0, 0, 0, 1, 0, 0, 0, 2, // S7: proto, rosctr=3, pdu_ref, param_len, data_len
//!     0xFF, 0x04,             // return code + transport size (data block)
//! ];
//! let s = parse(&f).unwrap();
//! assert_eq!(s.rosctr, Rosctr::AckData);
//! assert_eq!(s.pdu_ref, 1);
//! ```

/// S7 ROSCTR (message type byte after `0x32`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rosctr {
    /// 0x01 — job request.
    Job,
    /// 0x02 — simple acknowledgement.
    Ack,
    /// 0x03 — acknowledgement with data.
    AckData,
    /// 0x07 — user data.
    UserData,
    /// Other value.
    Other(u8),
}

/// Parsed S7comm header.
pub struct S7 {
    /// Message type.
    pub rosctr: Rosctr,
    /// Raw rosctr byte.
    pub rosctr_raw: u8,
    /// PDU reference.
    pub pdu_ref: u16,
    /// Declared parameter byte length.
    pub param_len: usize,
    /// Declared data byte length.
    pub data_len: usize,
    /// Offset of the S7 parameter block.
    pub param_at: usize,
    /// For Ack/AckData: error class + code at param offset (when param_len >= 2).
    pub error: Option<(u8, u8)>,
}

/// Parses a TPKT+COTP+S7 packet. Requires the TPKT length to equal `d.len()`,
/// a COTP DT (or CC) TPDU, and a complete 10-byte S7 header.
pub fn parse(d: &[u8]) -> Option<S7> {
    if d.len() < 7 || d[0] != 0x03 || d[1] != 0x00 {
        return None;
    }
    let tpkt_len = (((d[2] as u16) << 8) | d[3] as u16) as usize;
    if tpkt_len != d.len() {
        return None;
    }
    // COTP: length byte, then TPDU code; DT = 0xF0, CC = 0xE0
    let cotp_len = d[4] as usize;
    if d.len() < 5 + cotp_len {
        return None;
    }
    let tpdu = d[5];
    let s7_at = 5 + cotp_len;
    // Only a DT TPDU carries S7; CC/CR/DR terminate here.
    if tpdu != 0xF0 {
        return None;
    }
    if d.len() < s7_at + 10 || d[s7_at] != 0x32 {
        return None;
    }
    let h = &d[s7_at..];
    let param_len = (((h[6] as u16) << 8) | h[7] as u16) as usize;
    let data_len = (((h[8] as u16) << 8) | h[9] as u16) as usize;
    // header(10) + param + data must fit in the packet
    if s7_at + 10 + param_len + data_len > d.len() {
        return None;
    }
    let rosctr_raw = h[1];
    let rosctr = match rosctr_raw {
        0x01 => Rosctr::Job,
        0x02 => Rosctr::Ack,
        0x03 => Rosctr::AckData,
        0x07 => Rosctr::UserData,
        o => Rosctr::Other(o),
    };
    let error = if matches!(rosctr, Rosctr::Ack | Rosctr::AckData) && h.len() >= 12 {
        Some((h[10], h[11]))
    } else {
        None
    };
    Some(S7 {
        rosctr,
        rosctr_raw,
        pdu_ref: ((h[4] as u16) << 8) | h[5] as u16,
        param_len,
        data_len,
        param_at: s7_at + 10,
        error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(rosctr: u8, param: &[u8], data: &[u8]) -> Vec<u8> {
        let mut v = vec![0x03, 0x00, 0, 0, 0x02, 0xF0, 0x80];
        let pl = param.len() as u16;
        let dl = data.len() as u16;
        v.extend_from_slice(&[0x32, rosctr, 0, 0, 0, 7, (pl >> 8) as u8, pl as u8]);
        v.extend_from_slice(&[(dl >> 8) as u8, dl as u8]);
        v.extend_from_slice(param);
        v.extend_from_slice(data);
        let n = v.len() as u16;
        v[2] = (n >> 8) as u8;
        v[3] = n as u8;
        v
    }

    #[test]
    fn ack_data() {
        let f = frame(3, &[0xFF, 0x04], &[0xDE, 0xAD]);
        let s = parse(&f).unwrap();
        assert_eq!(s.rosctr, Rosctr::AckData);
        assert_eq!(s.pdu_ref, 7);
        assert_eq!(s.param_len, 2);
        assert_eq!(s.data_len, 2);
        assert_eq!(s.error, Some((0xFF, 0x04)));
        assert_eq!(f[s.param_at], 0xFF);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0x03, 0x00, 0, 7, 0x02, 0xF0, 0x80]).is_none()); // no S7
        let mut f = frame(1, &[], &[]);
        f[3] = 0; // tpkt len wrong
        assert!(parse(&f).is_none());
        let j = frame(1, &[], &[]);
        let s = parse(&j).unwrap();
        assert_eq!(s.rosctr, Rosctr::Job);
        assert!(s.error.is_none());
    }
}
