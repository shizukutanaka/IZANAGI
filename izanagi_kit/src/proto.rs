//! Protocol Buffers wire format: the `(field << 3) | wire` tag
//! stream — varint, fixed64, length-delimited, fixed32 fields —
//! walked into spans plus typed accessors. Groups (wire 3/4) are
//! rejected: deprecated since proto2 and nesting makes the scan
//! non-streaming. `proto` decodes *structure*, not `.proto` schemas.
//!
//! `varint` provides the LEB128 reader.
//!
//! ```
//! use izanagi_kit::proto;
//! // field 1 varint=150, field 2 len="hi"
//! let mut m = proto::tag(1, 0);
//! m.extend_from_slice(&proto::varint(150));
//! m.extend_from_slice(&proto::tag(2, 2));
//! m.extend_from_slice(&proto::varint(2));
//! m.extend_from_slice(b"hi");
//! let f = proto::fields(&m).unwrap();
//! assert_eq!(f.len(), 2);
//! assert_eq!(f[0].num, 1);
//! assert_eq!(proto::varint_at(&m, &f[0]), Some(150));
//! assert_eq!(proto::bytes_at(&m, &f[1]), Some(&b"hi"[..]));
//! ```

/// Wire types (tag low 3 bits).
pub const WIRE_VARINT: u8 = 0;
/// Fixed 64-bit.
pub const WIRE_I64: u8 = 1;
/// Length-delimited.
pub const WIRE_LEN: u8 = 2;
/// Fixed 32-bit.
pub const WIRE_I32: u8 = 5;

/// One decoded field span — payload coordinates, not the value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// Field number (`1..=0x1FFF_FFFF` in valid messages).
    pub num: u32,
    /// Wire type (0, 1, 2, or 5).
    pub wire: u8,
    /// Offset of the field payload in the input.
    pub offset: usize,
    /// Payload length in bytes (varint: byte count; LEN: contents only).
    pub len: usize,
}

/// Emit a tag byte-sequence for `(num, wire)`.
pub fn tag(num: u32, wire: u8) -> Vec<u8> {
    crate::varint::encode_u64(((num as u64) << 3) | wire as u64)
}

/// Emit a bare varint.
pub fn varint(v: u64) -> Vec<u8> {
    crate::varint::encode_u64(v)
}

/// Emit `num` as a varint field.
pub fn varint_field(num: u32, v: u64) -> Vec<u8> {
    let mut out = tag(num, WIRE_VARINT);
    out.extend_from_slice(&varint(v));
    out
}

/// Emit `num` as a length-delimited field (bytes/string/sub-message).
pub fn len_field(num: u32, body: &[u8]) -> Vec<u8> {
    let mut out = tag(num, WIRE_LEN);
    out.extend_from_slice(&varint(body.len() as u64));
    out.extend_from_slice(body);
    out
}

/// Emit `num` as a fixed64 field (LE on the wire).
pub fn fixed64_field(num: u32, v: u64) -> Vec<u8> {
    let mut out = tag(num, WIRE_I64);
    for i in 0..8 {
        out.push((v >> (i * 8)) as u8);
    }
    out
}

/// Emit `num` as a fixed32 field (LE on the wire).
pub fn fixed32_field(num: u32, v: u32) -> Vec<u8> {
    let mut out = tag(num, WIRE_I32);
    for i in 0..4 {
        out.push((v >> (i * 8)) as u8);
    }
    out
}

/// Walk a message's fields. `None` on a bad tag, wire type 3/4
/// (groups), field number 0, or a payload running past the input.
pub fn fields(d: &[u8]) -> Option<Vec<Field>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        let (t, n) = crate::varint::decode_u64(d.get(i..)?)?;
        i += n;
        let num = (t >> 3) as u32;
        let wire = (t & 7) as u8;
        if num == 0 {
            return None;
        }
        let (offset, len) = match wire {
            WIRE_VARINT => {
                let (_, n) = crate::varint::decode_u64(d.get(i..)?)?;
                (i, n)
            }
            WIRE_I64 => (i, 8),
            WIRE_LEN => {
                let (l, n) = crate::varint::decode_u64(d.get(i..)?)?;
                (i + n, l as usize)
            }
            WIRE_I32 => (i, 4),
            _ => return None, // groups deprecated, 6/7 impossible
        };
        if offset.checked_add(len)? > d.len() {
            return None;
        }
        out.push(Field {
            num,
            wire,
            offset,
            len,
        });
        i = offset + len;
    }
    Some(out)
}

/// Read a varint field's value.
pub fn varint_at(d: &[u8], f: &Field) -> Option<u64> {
    if f.wire != WIRE_VARINT {
        return None;
    }
    let (v, n) = crate::varint::decode_u64(d.get(f.offset..f.offset + f.len)?)?;
    (n == f.len).then_some(v)
}

/// Read a length-delimited field's bytes.
pub fn bytes_at<'a>(d: &'a [u8], f: &Field) -> Option<&'a [u8]> {
    if f.wire != WIRE_LEN {
        return None;
    }
    d.get(f.offset..f.offset + f.len)
}

/// Read a length-delimited field as UTF-8.
pub fn string_at<'a>(d: &'a [u8], f: &Field) -> Option<&'a str> {
    std::str::from_utf8(bytes_at(d, f)?).ok()
}

/// Read a fixed64 field (LE).
pub fn fixed64_at(d: &[u8], f: &Field) -> Option<u64> {
    if f.wire != WIRE_I64 || f.len != 8 {
        return None;
    }
    let mut v = 0u64;
    for i in 0..8 {
        v |= (*d.get(f.offset + i)? as u64) << (i * 8);
    }
    Some(v)
}

/// Read a fixed32 field (LE).
pub fn fixed32_at(d: &[u8], f: &Field) -> Option<u32> {
    if f.wire != WIRE_I32 || f.len != 4 {
        return None;
    }
    let mut v = 0u32;
    for i in 0..4 {
        v |= (*d.get(f.offset + i)? as u32) << (i * 8);
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_wires() {
        let mut m = varint_field(1, 150);
        m.extend_from_slice(&len_field(2, b"hi"));
        m.extend_from_slice(&fixed64_field(3, 0x1122_3344_5566_7788));
        m.extend_from_slice(&fixed32_field(4, 0xAABB_CCDD));
        let f = fields(&m).unwrap();
        assert_eq!(f.len(), 4);
        assert_eq!(varint_at(&m, &f[0]), Some(150));
        assert_eq!(bytes_at(&m, &f[1]), Some(&b"hi"[..]));
        assert_eq!(string_at(&m, &f[1]), Some("hi"));
        assert_eq!(fixed64_at(&m, &f[2]), Some(0x1122_3344_5566_7788));
        assert_eq!(fixed32_at(&m, &f[3]), Some(0xAABB_CCDD));
        // wrong-wire accessors return None
        assert_eq!(varint_at(&m, &f[1]), None);
    }

    #[test]
    fn noncanonical_varint_rejected() {
        // protobuf tolerates overlong varints on the wire; this module
        // keeps `varint`'s canonical-only rule — padding is rejected
        let m = [0x08, 0x96, 0x81, 0x80, 0x80, 0x00]; // field1, overlong
        assert!(fields(&m).is_none());
    }

    #[test]
    fn malformed_rejected() {
        assert_eq!(fields(&[]), Some(vec![])); // empty message is valid
        assert!(fields(&[0x00]).is_none()); // field 0
        assert!(fields(&[0x0B]).is_none()); // wire 3 (group)
        assert!(fields(&[0x08]).is_none()); // tag then missing varint
        assert!(fields(&[0x0A, 0x05, b'h', b'i']).is_none()); // len overruns
        let mut m = varint_field(1, 7);
        m.pop(); // truncate mid-varint
        assert!(fields(&m).is_none());
    }

    #[test]
    fn packed_repeated_is_len_field() {
        // field 4 packed varints [1,2,300]
        let m = len_field(4, &[1, 2, 0xAC, 0x02]);
        let f = fields(&m).unwrap();
        assert_eq!(bytes_at(&m, &f[0]).unwrap(), &[1, 2, 0xAC, 0x02]);
    }
}
