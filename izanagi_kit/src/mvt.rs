//! Mapbox Vector Tile (MVT spec 2.x): a tile is a protobuf
//! message of `layer`s, usually gzip-wrapped — [`proto`] and
//! [`inflate_gzip`](crate::inflate::inflate_gzip) do the heavy lifting. Geometry is the
//! `MoveTo=1/LineTo=2/ClosePath=7` command stream with zigzag
//! deltas.
//!
//! ```
//! use izanagi_kit::{mvt, proto};
//! let mut layer = proto::varint_field(15, 4096); // extent
//! layer.extend_from_slice(&proto::len_field(2, b"water"));
//! layer.extend_from_slice(&proto::varint_field(1, 2)); // version
//! let mut tile = proto::len_field(3, &layer);
//! let layers = mvt::parse(&tile).unwrap();
//! assert_eq!(layers[0].name, "water");
//! assert_eq!(layers[0].extent, 4096);
//! ```

use crate::proto;
use std::vec::Vec;

/// A decoded `value` message.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// field 1: string.
    Str(std::string::String),
    /// field 2: raw f32 bits.
    FloatBits(u32),
    /// field 3: raw f64 bits.
    DoubleBits(u64),
    /// field 4: int64.
    Int(i64),
    /// field 5: uint64.
    UInt(u64),
    /// field 6: sint64 (zigzag-decoded).
    SInt(i64),
    /// field 7: bool.
    Bool(bool),
}

/// zigzag decode.
pub fn zigzag(v: u64) -> i64 {
    ((v >> 1) as i64) ^ -((v & 1) as i64)
}

/// A decoded `feature` message.
#[derive(Clone, Debug)]
pub struct Feature {
    /// field 1.
    pub id: u64,
    /// field 2: key/value index pairs (flat).
    pub tags: Vec<u32>,
    /// field 3: 1 point, 2 linestring, 3 polygon.
    pub geom_type: u32,
    /// field 4: raw command stream.
    pub geometry: Vec<u32>,
}

/// One decoded geometry command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// MoveTo — pen up, new part.
    Move(i32, i32),
    /// LineTo — pen down.
    Line(i32, i32),
    /// ClosePath.
    Close,
}

/// Decodes a feature's geometry into commands, applying the
/// zigzag deltas. `None` on a malformed stream.
pub fn geom(f: &Feature) -> Option<Vec<Cmd>> {
    let mut out = Vec::new();
    let (mut x, mut y) = (0i64, 0i64);
    let mut i = 0usize;
    while i < f.geometry.len() {
        let hdr = *f.geometry.get(i)?;
        i += 1;
        let cmd = hdr & 7;
        let count = hdr >> 3;
        match cmd {
            1 | 2 => {
                for _ in 0..count {
                    let dx = zigzag(*f.geometry.get(i)? as u64);
                    let dy = zigzag(*f.geometry.get(i + 1)? as u64);
                    i += 2;
                    x += dx;
                    y += dy;
                    if !(i32::MIN as i64..=i32::MAX as i64).contains(&x)
                        || !(i32::MIN as i64..=i32::MAX as i64).contains(&y)
                    {
                        return None;
                    }
                    out.push(if cmd == 1 {
                        Cmd::Move(x as i32, y as i32)
                    } else {
                        Cmd::Line(x as i32, y as i32)
                    });
                }
            }
            7 => {
                for _ in 0..count {
                    out.push(Cmd::Close);
                }
            }
            _ => return None,
        }
    }
    Some(out)
}

/// One `layer` message.
#[derive(Clone, Debug)]
pub struct Layer {
    /// field 2.
    pub name: std::string::String,
    /// field 3: features.
    pub features: Vec<Feature>,
    /// field 4: tag keys.
    pub keys: Vec<std::string::String>,
    /// field 5: tag values.
    pub values: Vec<Value>,
    /// field 15 (default 4096 when absent).
    pub extent: u32,
    /// field 1 (default 1 when absent).
    pub version: u32,
}

fn packed_u32(d: &[u8], f: &proto::Field) -> Option<Vec<u32>> {
    let b = proto::bytes_at(d, f)?;
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        let (v, n) = crate::varint::decode_u64(b.get(i..)?)?;
        if v > u32::MAX as u64 {
            return None;
        }
        out.push(v as u32);
        i += n;
    }
    Some(out)
}

fn feature(d: &[u8]) -> Option<Feature> {
    let fs = proto::fields(d)?;
    let mut id = 0;
    let mut tags = Vec::new();
    let mut ty = 0;
    let mut geo = Vec::new();
    for f in &fs {
        match f.num {
            1 => id = proto::varint_at(d, f)?,
            2 => tags = packed_u32(d, f)?,
            3 => ty = proto::varint_at(d, f)? as u32,
            4 => geo = packed_u32(d, f)?,
            _ => {}
        }
    }
    Some(Feature {
        id,
        tags,
        geom_type: ty,
        geometry: geo,
    })
}

fn value(d: &[u8]) -> Option<Value> {
    let fs = proto::fields(d)?;
    for f in &fs {
        match f.num {
            1 => return Some(Value::Str(proto::string_at(d, f)?.to_string())),
            2 => return Some(Value::FloatBits(proto::fixed32_at(d, f)?)),
            3 => return Some(Value::DoubleBits(proto::fixed64_at(d, f)?)),
            4 => return Some(Value::Int(proto::varint_at(d, f)? as i64)),
            5 => return Some(Value::UInt(proto::varint_at(d, f)?)),
            6 => {
                let v = proto::varint_at(d, f)?;
                return Some(Value::SInt(zigzag(v)));
            }
            7 => return Some(Value::Bool(proto::varint_at(d, f)? != 0)),
            _ => {}
        }
    }
    Some(Value::Bool(false)) // empty message → false-ish default
}

fn layer(d: &[u8]) -> Option<Layer> {
    let fs = proto::fields(d)?;
    let mut name = std::string::String::new();
    let mut version = 1u32;
    let mut extent = 4096u32;
    let mut features = Vec::new();
    let mut keys = Vec::new();
    let mut values = Vec::new();
    for f in &fs {
        match f.num {
            1 => version = proto::varint_at(d, f)? as u32,
            2 => name = proto::string_at(d, f)?.to_string(),
            3 => features.push(feature(proto::bytes_at(d, f)?)?),
            4 => keys.push(proto::string_at(d, f)?.to_string()),
            5 => values.push(value(proto::bytes_at(d, f)?)?),
            15 => extent = proto::varint_at(d, f)? as u32,
            _ => {}
        }
    }
    Some(Layer {
        name,
        features,
        keys,
        values,
        extent,
        version,
    })
}

/// Parses a tile — raw protobuf, or gzip when it starts with
/// `1f 8b` (the usual `.mvt` framing).
pub fn parse(d: &[u8]) -> Option<Vec<Layer>> {
    let body: Vec<u8> = if d.get(0..2) == Some(&[0x1F, 0x8B]) {
        crate::inflate::inflate_gzip(d)?
    } else {
        d.to_vec()
    };
    let mut out = Vec::new();
    for f in &proto::fields(&body)? {
        if f.num == 3 {
            out.push(layer(proto::bytes_at(&body, f)?)?);
        }
    }
    Some(out)
}

/// `feature.tags` as resolved `(key, value)` pairs.
pub fn attrs<'a>(l: &'a Layer, f: &'a Feature) -> Vec<(&'a str, &'a Value)> {
    let mut out = Vec::new();
    for p in f.tags.chunks_exact(2) {
        if let (Some(k), Some(v)) = (l.keys.get(p[0] as usize), l.values.get(p[1] as usize)) {
            out.push((k.as_str(), v));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer_with_feature() -> Vec<u8> {
        // feature{ id=7, tags=[0,0], geom_type=1, geom=[Move(1,1) ×1] }
        let mut feat = proto::varint_field(1, 7);
        feat.extend_from_slice(&proto::len_field(2, &[0, 0]));
        feat.extend_from_slice(&proto::varint_field(3, 1));
        // geom: hdr=(1<<3)|1 = 9 → MoveTo ×1; dx=2 (zigzag 1), dy=4 (zigzag 2)
        feat.extend_from_slice(&proto::len_field(4, &[9, 2, 4]));
        let mut layer = proto::varint_field(1, 2);
        layer.extend_from_slice(&proto::len_field(2, b"land"));
        layer.extend_from_slice(&proto::len_field(3, &feat));
        layer.extend_from_slice(&proto::len_field(4, b"kind"));
        let mut val = proto::len_field(1, b"plain");
        layer.extend_from_slice(&proto::len_field(5, &val));
        val.clear();
        proto::len_field(3, &layer)
    }

    #[test]
    fn decodes_layer_feature_attrs() {
        let tile = layer_with_feature();
        let layers = parse(&tile).unwrap();
        assert_eq!(layers.len(), 1);
        let l = &layers[0];
        assert_eq!(l.name, "land");
        assert_eq!(l.version, 2);
        assert_eq!(l.extent, 4096); // absent → default
        let f = &l.features[0];
        assert_eq!(f.id, 7);
        assert_eq!(f.geom_type, 1);
        assert_eq!(geom(f).unwrap(), vec![Cmd::Move(1, 2)]);
        let a = attrs(l, f);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].0, "kind");
        assert_eq!(a[0].1, &Value::Str("plain".into()));
    }

    #[test]
    fn gzipped_tile_decodes() {
        let tile = layer_with_feature();
        let gz = crate::deflate::deflate_gzip(&tile);
        let layers = parse(&gz).unwrap();
        assert_eq!(layers[0].name, "land");
    }

    #[test]
    fn geom_close_and_lines() {
        // MoveTo×1 (1,1), LineTo×2 (0,+1),(+1,0), ClosePath×1
        let mut feat = proto::varint_field(3, 3);
        // 9:Move×1 dx2 dy2 →(1,1); 0x12:Line×2 dz→(0,1) dzig0? dy zigzag 2=+1, dx zigzag 2=+1
        feat.extend_from_slice(&proto::len_field(4, &[9, 2, 2, 0x12, 0, 2, 2, 0, 0x0F]));
        let f = feature(&feat).unwrap();
        let g = geom(&f).unwrap();
        assert_eq!(
            g,
            vec![
                Cmd::Move(1, 1),
                Cmd::Line(1, 2),
                Cmd::Line(2, 2),
                Cmd::Close
            ]
        );
    }

    #[test]
    fn zigzag_roundtrip() {
        assert_eq!(zigzag(0), 0);
        assert_eq!(zigzag(1), -1);
        assert_eq!(zigzag(2), 1);
        assert_eq!(zigzag(3), -2);
        assert_eq!(zigzag(u64::MAX), i64::MIN);
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).unwrap().is_empty()); // zero layers is valid
        let g = geom(&Feature {
            id: 0,
            tags: vec![],
            geom_type: 1,
            geometry: vec![1], // MoveTo×0 with bad count encoding
        });
        assert!(g.is_some()); // count 0 → empty ok
        let g2 = geom(&Feature {
            id: 0,
            tags: vec![],
            geom_type: 1,
            geometry: vec![9, 2], // MoveTo×1 needs 2 coords
        });
        assert!(g2.is_none());
    }
}
