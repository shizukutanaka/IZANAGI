//! ONNX — `ModelProto` protobuf: field 1 `ir_version` (varint),
//! 3 `producer_name`, 4 `producer_version`, 7 `graph`
//! (sub-message). Built on `crate::proto`'s wire reader.
//!
//! ```
//! use izanagi_kit::{onnx, proto};
//!
//! let mut d = proto::varint_field(1, 7);
//! d.extend_from_slice(&proto::len_field(3, b"izanagi"));
//! d.extend_from_slice(&proto::len_field(4, b"1.0"));
//! let m = onnx::parse(&d).unwrap();
//! assert_eq!(m.ir_version(), Some(7));
//! assert_eq!(m.producer(), Some("izanagi"));
//! assert_eq!(m.producer_version(), Some("1.0"));
//! ```

use crate::proto::{self, Field};

/// Parsed `ModelProto` field view.
#[derive(Debug, Clone)]
pub struct Onnx<'a> {
    d: &'a [u8],
    /// All top-level fields.
    pub fields: Vec<Field>,
}

/// Field numbers in `onnx.ModelProto` that this reader surfaces.
pub mod field {
    /// `ir_version` (int64 varint).
    pub const IR_VERSION: u32 = 1;
    /// `opset_import` (repeated OperatorSetIdProto message).
    pub const OPSET_IMPORT: u32 = 2;
    /// `producer_name` (string).
    pub const PRODUCER: u32 = 3;
    /// `producer_version` (string).
    pub const PRODUCER_VERSION: u32 = 4;
    /// `domain` (string).
    pub const DOMAIN: u32 = 5;
    /// `model_version` (int64 varint).
    pub const MODEL_VERSION: u32 = 6;
    /// `graph` (GraphProto message).
    pub const GRAPH: u32 = 7;
}

/// Parse the wire stream; `None` when the protobuf is malformed.
pub fn parse(d: &[u8]) -> Option<Onnx<'_>> {
    let fields = proto::fields(d)?;
    Some(Onnx { d, fields })
}

impl<'a> Onnx<'a> {
    fn first(&self, num: u32) -> Option<&Field> {
        self.fields.iter().find(|f| f.num == num)
    }

    /// `ir_version` — the ONNX spec version the model targets.
    pub fn ir_version(&self) -> Option<u64> {
        let f = self.first(field::IR_VERSION)?;
        proto::varint_at(self.d, f)
    }

    /// `producer_name`.
    pub fn producer(&self) -> Option<&str> {
        let f = self.first(field::PRODUCER)?;
        proto::string_at(self.d, f)
    }

    /// `producer_version`.
    pub fn producer_version(&self) -> Option<&str> {
        let f = self.first(field::PRODUCER_VERSION)?;
        proto::string_at(self.d, f)
    }

    /// The `graph` sub-message bytes (its own protobuf stream).
    pub fn graph(&self) -> Option<&'a [u8]> {
        let f = self.first(field::GRAPH)?;
        proto::bytes_at(self.d, f)
    }

    /// `opset_import` sub-messages.
    pub fn opsets(&self) -> Vec<&'a [u8]> {
        self.fields
            .iter()
            .filter(|f| f.num == field::OPSET_IMPORT)
            .filter_map(|f| proto::bytes_at(self.d, f))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = proto::varint_field(1, 10);
        d.extend_from_slice(&proto::len_field(3, b"torch"));
        d.extend_from_slice(&proto::len_field(4, b"2.0"));
        // graph sub-message: name = "g"
        d.extend_from_slice(&proto::len_field(7, &proto::len_field(2, b"g")));
        // opset: domain "" + version 13
        let mut opset = proto::len_field(1, b"");
        opset.extend_from_slice(&proto::varint_field(2, 13));
        d.extend_from_slice(&proto::len_field(2, &opset));
        d
    }

    #[test]
    fn fields() {
        let d = fixture();
        let m = parse(&d).unwrap();
        assert_eq!(m.ir_version(), Some(10));
        assert_eq!(m.producer(), Some("torch"));
        assert_eq!(m.producer_version(), Some("2.0"));
        let g = m.graph().unwrap();
        assert_eq!(proto::fields(g).unwrap().len(), 1);
        assert_eq!(m.opsets().len(), 1);
        let inner = proto::fields(m.opsets()[0]).unwrap();
        assert_eq!(proto::varint_at(m.opsets()[0], &inner[1]), Some(13));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0xFF]).is_none()); // bad varint tag
        assert!(parse(b"").unwrap().fields.is_empty());
    }
}
