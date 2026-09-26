//! OBD-II — Mode/PID request & response framing on top of CAN.
//!
//! Request (ISO-TP single frame): `[len, 0x01|mode, pid, ...]` on ID
//! `0x7DF`; positive response on `0x7E8` with mode+0x40; negative is
//! `7F <mode> <nrc>`. `decode_response` exposes `(pid, data)` slices.
//!
//! ```
//! use izanagi_kit::obd::{decode_response, Response};
//!
//! // mode 01 pid 0C (RPM) with data bytes A B
//! match decode_response(&[4, 0x41, 0x0C, 0x1A, 0xF8, 0, 0, 0]) {
//!     Some(Response::Positive { mode, pid, data }) => {
//!         assert_eq!((mode, pid), (1, 0x0C));
//!         assert_eq!(data, &[0x1A, 0xF8]);
//!     }
//!     _ => panic!(),
//! }
//! ```

/// OBD-II response kinds.
#[derive(Clone, Debug)]
pub enum Response {
    /// Positive: `len, mode+0x40, pid, data..`.
    Positive {
        /// Echoed mode (high nibble 4, e.g. 1 for "show current").
        mode: u8,
        /// PID byte.
        pid: u8,
        /// Remaining data bytes of this frame.
        data: Vec<u8>,
    },
    /// Negative: `03 7F mode nrc`.
    Negative {
        /// Original mode.
        mode: u8,
        /// Negative response code.
        nrc: u8,
    },
}

/// Decode one ISO-TP single OBD-II frame payload (up to 8 bytes).
/// `None` when empty, `len` exceeds the buffer, or the shape is neither
/// positive nor negative.
pub fn decode_response(d: &[u8]) -> Option<Response> {
    let len = *d.first()? as usize;
    if len == 0 || d.len() < 1 + len {
        return None;
    }
    let body = &d[1..1 + len];
    if body[0] == 0x7F {
        if len < 3 {
            return None;
        }
        return Some(Response::Negative {
            mode: body[1],
            nrc: body[2],
        });
    }
    let mode = body[0] & 0x0F;
    if body[0] >> 4 != 4 || len < 2 {
        return None;
    }
    Some(Response::Positive {
        mode,
        pid: body[1],
        data: body[2..].to_vec(),
    })
}

/// `true` when `id` is the functional OBD-II broadcast request ID.
pub fn is_request_id(id: u32) -> bool {
    id == 0x7DF
}

/// `true` when `id` is a physical response ID (`0x7E8`..`0x7EF`).
pub fn is_response_id(id: u32) -> bool {
    (0x7E8..=0x7EF).contains(&id)
}

/// Mode-01 PID formulas, `(pid)` → `Some((bytes_needed, "formula"))`
/// for a few common PIDs; `None` when unknown.
pub fn mode01_formula(pid: u8) -> Option<&'static str> {
    Some(match pid {
        0x00 => "supported PIDs bitmask",
        0x04 => "engine load A*100/255",
        0x05 => "coolant temp A-40 °C",
        0x0C => "rpm (A*256+B)/4",
        0x0D => "vehicle speed A km/h",
        0x0F => "intake air temp A-40 °C",
        0x10 => "MAF (A*256+B)/100 g/s",
        0x11 => "throttle A*100/255 %",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive() {
        match decode_response(&[4, 0x41, 0x0C, 0x1A, 0xF8]) {
            Some(Response::Positive { mode, pid, data }) => {
                assert_eq!(mode, 1);
                assert_eq!(pid, 0x0C);
                assert_eq!(data, vec![0x1A, 0xF8]);
            }
            _ => panic!(),
        }
        match decode_response(&[3, 0x7F, 0x01, 0x12]) {
            Some(Response::Negative { mode, nrc }) => {
                assert_eq!((mode, nrc), (1, 0x12));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn rejects() {
        assert!(decode_response(&[]).is_none());
        assert!(decode_response(&[0]).is_none());
        assert!(decode_response(&[5, 0x41]).is_none());
        // len<2 without pid stays malformed
        assert!(decode_response(&[1, 0x41]).is_none());
        // len=2 → positive with empty data
        assert!(matches!(
            decode_response(&[2, 0x41, 0x0C]),
            Some(Response::Positive { .. })
        ));
    }

    #[test]
    fn ids() {
        assert!(is_request_id(0x7DF));
        assert!(!is_request_id(0x7E8));
        assert!(is_response_id(0x7E8));
        assert!(!is_response_id(0x7F0));
        assert!(mode01_formula(0x0C).is_some());
        assert!(mode01_formula(0xFE).is_none());
    }
}
