//! Modbus TCP (MBAP) and RTU frame parsing.
//!
//! ```
//! use izanagi_kit::modbus::{parse_tcp, parse_rtu, crc16};
//!
//! // TCP: txid=1 proto=0 len=4 unit=1 func=3 data=[0x02]
//! let tcp = [0u8, 1, 0, 0, 0, 4, 1, 3, 2, 0];
//! let m = parse_tcp(&tcp).unwrap();
//! assert_eq!(m.func, 3);
//!
//! // RTU: addr=1 func=3 + CRC over the whole PDU
//! let mut rtu = vec![1u8, 3, 0x02, 0x2B, 0x00];
//! let c = crc16(&rtu);
//! rtu.push((c & 0xFF) as u8);
//! rtu.push((c >> 8) as u8);
//! assert!(parse_rtu(&rtu).is_some());
//! ```

/// Parsed Modbus TCP frame (MBAP header + PDU).
pub struct ModbusTcp {
    /// Transaction identifier.
    pub txid: u16,
    /// Unit identifier (MBAP unit field).
    pub unit: u8,
    /// Function code (first PDU byte).
    pub func: u8,
    /// Byte offset of PDU data after the function code.
    pub data_at: usize,
    /// PDU data length in bytes.
    pub data_len: usize,
}

/// Parses a Modbus TCP frame: `txid(2) proto(2)=0 len(2) unit(1) func(1) data...`.
/// `len` covers unit+PDU and must match the remainder exactly.
pub fn parse_tcp(d: &[u8]) -> Option<ModbusTcp> {
    if d.len() < 8 {
        return None;
    }
    let txid = ((d[0] as u16) << 8) | d[1] as u16;
    let proto = ((d[2] as u16) << 8) | d[3] as u16;
    if proto != 0 {
        return None;
    }
    let len = (((d[4] as u16) << 8) | d[5] as u16) as usize;
    if len < 2 || 6 + len != d.len() {
        return None;
    }
    Some(ModbusTcp {
        txid,
        unit: d[6],
        func: d[7],
        data_at: 8,
        data_len: len - 2,
    })
}

/// Modbus CRC-16 (poly 0xA001 reflected, init 0xFFFF), returned as u16;
/// on the wire it goes low byte first.
pub fn crc16(d: &[u8]) -> u16 {
    let mut c: u16 = 0xFFFF;
    for &b in d {
        c ^= b as u16;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ 0xA001
            } else {
                c >> 1
            };
        }
    }
    c
}

/// Parsed Modbus RTU frame.
pub struct ModbusRtu {
    /// Slave address.
    pub addr: u8,
    /// Function code.
    pub func: u8,
    /// Byte offset of PDU data.
    pub data_at: usize,
    /// PDU data length (between function and CRC).
    pub data_len: usize,
}

/// Parses an RTU frame `addr func data crc_lo crc_hi`; CRC verified.
/// Returns `None` for exception-looking or malformed frames only when
/// the CRC does not match — the function byte itself is not constrained.
pub fn parse_rtu(d: &[u8]) -> Option<ModbusRtu> {
    if d.len() < 4 {
        return None;
    }
    let want = crc16(&d[..d.len() - 2]);
    let got = (d[d.len() - 1] as u16) << 8 | d[d.len() - 2] as u16;
    if want != got {
        return None;
    }
    Some(ModbusRtu {
        addr: d[0],
        func: d[1],
        data_at: 2,
        data_len: d.len() - 4,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp() {
        let m = parse_tcp(&[0, 0xAB, 0, 0, 0, 3, 9, 4, 0x77]).unwrap();
        assert_eq!(m.txid, 0xAB);
        assert_eq!(m.unit, 9);
        assert_eq!(m.func, 4);
        assert_eq!(m.data_len, 1);
        assert_eq!(m.data_at, 8);
        // proto nonzero
        assert!(parse_tcp(&[0, 0, 0, 1, 0, 3, 9, 4, 0x77]).is_none());
        // len mismatch
        assert!(parse_tcp(&[0, 0, 0, 0, 0, 5, 9, 4, 0x77]).is_none());
        // short
        assert!(parse_tcp(&[0; 7]).is_none());
    }

    #[test]
    fn rtu() {
        // known vector: frame 01 04 02 FF FF B8 80 (read input regs resp)
        let f = [1u8, 4, 2, 0xFF, 0xFF, 0xB8, 0x80];
        let r = parse_rtu(&f).unwrap();
        assert_eq!(r.addr, 1);
        assert_eq!(r.func, 4);
        assert_eq!(r.data_len, 3);
        let mut bad = f;
        bad[2] = 3;
        assert!(parse_rtu(&bad).is_none());
        assert!(parse_rtu(&f[..3]).is_none());
    }
}
