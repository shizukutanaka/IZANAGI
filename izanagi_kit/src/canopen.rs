//! CANopen frame classification over a raw CAN 2.0A frame (COB-ID + data).
//!
//! ```
//! use izanagi_kit::canopen::{parse, Func};
//!
//! // COB-ID 0x600 + node 5 = SDO tx (client→server), data [0x40 ...] = upload req
//! let f = parse(0x605, &[0x40, 0x00, 0x10, 0x00]).unwrap();
//! assert_eq!(f.func, Func::SdoRx);
//! assert_eq!(f.node, 5);
//! ```

/// CANopen function code (COB-ID >> 7) classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    /// 0 — NMT network management.
    Nmt,
    /// 1 — SYNC / EMCY producer.
    SyncOrEmcy,
    /// 2 — TIME stamp.
    Time,
    /// 3,5,7,9 — TPDO 1-4.
    Tpdo(u8),
    /// 4,6,8,10 — RPDO 1-4.
    Rpdo(u8),
    /// 11 — SDO server transmit (response, server→client).
    SdoTx,
    /// 12 — SDO client transmit (request, client→server).
    SdoRx,
    /// 14 — heartbeat / node guarding / boot-up.
    Heartbeat,
    /// Other function code.
    Other(u8),
}

/// A classified CANopen frame.
pub struct CanOpen<'a> {
    /// Full 11-bit COB-ID.
    pub cob_id: u16,
    /// Function code group.
    pub func: Func,
    /// Raw function code bits (cob_id >> 7).
    pub func_code: u8,
    /// Node id (cob_id & 0x7F).
    pub node: u8,
    /// CAN payload (0-8 bytes).
    pub data: &'a [u8],
}

impl CanOpen<'_> {
    /// For NMT frames (func 0, cob-id 0): command specifier + target node.
    /// Returns `(cs, target)`; cs 1=start, 2=stop, 0x80=pre-op, 0x81=reset, 0x82=reset-comm.
    pub fn nmt_command(&self) -> Option<(u8, u8)> {
        if self.cob_id != 0 || self.data.len() < 2 {
            return None;
        }
        Some((self.data[0], self.data[1]))
    }
}

/// Classifies a CAN frame's COB-ID into a CANopen function group.
/// `cob_id` must be ≤ 0x7FF (standard frame); `data` is the ≤8-byte payload.
pub fn parse(cob_id: u16, data: &[u8]) -> Option<CanOpen<'_>> {
    if cob_id > 0x7FF || data.len() > 8 {
        return None;
    }
    let func_code = (cob_id >> 7) as u8;
    let func = match func_code {
        0 => Func::Nmt,
        1 => Func::SyncOrEmcy,
        2 => Func::Time,
        3 => Func::Tpdo(1),
        5 => Func::Tpdo(2),
        7 => Func::Tpdo(3),
        9 => Func::Tpdo(4),
        4 => Func::Rpdo(1),
        6 => Func::Rpdo(2),
        8 => Func::Rpdo(3),
        10 => Func::Rpdo(4),
        11 => Func::SdoTx,
        12 => Func::SdoRx,
        14 => Func::Heartbeat,
        o => Func::Other(o),
    };
    Some(CanOpen {
        cob_id,
        func,
        func_code,
        node: (cob_id & 0x7F) as u8,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify() {
        let f = parse(0x000, &[1, 0]).unwrap();
        assert_eq!(f.func, Func::Nmt);
        assert_eq!(f.nmt_command(), Some((1, 0))); // start all

        let f = parse(0x080, &[0]).unwrap();
        assert_eq!(f.func, Func::SyncOrEmcy);

        let f = parse(0x183, &[0; 8]).unwrap();
        assert_eq!(f.func, Func::Tpdo(1));
        assert_eq!(f.node, 3);

        let f = parse(0x583, &[0x43]).unwrap();
        assert_eq!(f.func, Func::SdoTx);
        assert_eq!(f.node, 3);

        let f = parse(0x703, &[0x7F]).unwrap();
        assert_eq!(f.func, Func::Heartbeat);
        assert_eq!(f.data[0], 0x7F); // pre-operational
    }

    #[test]
    fn rejects() {
        assert!(parse(0x800, &[]).is_none()); // extended id not handled
        assert!(parse(0x600, &[0; 9]).is_none()); // >8 payload
    }
}
