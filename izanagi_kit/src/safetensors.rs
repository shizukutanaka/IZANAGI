//! SafeTensors — HuggingFace's safe tensor serialization:
//! `u64LE header_len` + a JSON object of `"name" -> {"dtype",
//! "shape", "data_offsets"}` (plus optional `"__metadata__"`),
//! then the raw tensor data blob.
//!
//! ```
//! use izanagi_kit::safetensors::parse;
//!
//! let h = br#"{"w":{"dtype":"F32","shape":[2,2],"data_offsets":[0,16]}}"#;
//! let mut d = Vec::new();
//! d.extend_from_slice(&u64::to_le_bytes(h.len() as u64));
//! d.extend_from_slice(h);
//! d.extend_from_slice(&[0u8; 16]);
//! let s = parse(&d).unwrap();
//! let t = s.tensors();
//! assert_eq!(t.len(), 1);
//! assert_eq!(t[0].name, "w");
//! assert_eq!(t[0].shape, vec![2, 2]);
//! assert_eq!(s.tensor_data(&t[0]), Some(&[0u8; 16][..]));
//! ```

use crate::json::{self, Json};
use std::vec::Vec;

/// Header length prefix is 8 bytes.
pub const PREFIX: usize = 8;

/// One tensor entry from the header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tensor {
    /// Tensor name (object key).
    pub name: String,
    /// dtype string (`"F32"`, `"I64"`, `"BF16"` …).
    pub dtype: String,
    /// Shape dimensions.
    pub shape: Vec<u64>,
    /// `[begin, end)` offsets into the data blob.
    pub begin: u64,
    /// End offset.
    pub end: u64,
}

/// Parsed file.
#[derive(Debug)]
pub struct Safetensors<'a> {
    d: &'a [u8],
    /// Offset where the tensor data blob begins.
    pub data_at: usize,
    header: &'a [u8],
}

fn le64(d: &[u8], i: usize) -> Option<u64> {
    let mut v = 0u64;
    for k in 0..8 {
        v |= u64::from(*d.get(i + k)?) << (8 * k);
    }
    Some(v)
}

/// Parse the header; `None` on truncation or invalid JSON.
pub fn parse(d: &[u8]) -> Option<Safetensors<'_>> {
    let len = usize::try_from(le64(d, 0)?).ok()?;
    let data_at = PREFIX.checked_add(len)?;
    let header = d.get(PREFIX..data_at)?;
    Some(Safetensors { d, data_at, header })
}

fn ints(v: &Json) -> Option<Vec<u64>> {
    match v {
        Json::Arr(items) => items
            .iter()
            .map(|i| match i {
                Json::Int(n) => u64::try_from(*n).ok(),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

impl<'a> Safetensors<'a> {
    /// The parsed header object.
    pub fn header(&self) -> Option<Json> {
        json::parse(self.header).ok()
    }

    /// Iterate tensor descriptors (skips `__metadata__`).
    pub fn tensors(&self) -> Vec<Tensor> {
        let mut out = Vec::new();
        let Ok(Json::Obj(map)) = json::parse(self.header) else {
            return out;
        };
        for (name, v) in &map {
            if name == "__metadata__" {
                continue;
            }
            let Json::Obj(t) = v else { continue };
            let Some(Json::Str(dtype)) = t.get("dtype") else {
                continue;
            };
            let Some(shape) = t.get("shape").and_then(ints) else {
                continue;
            };
            let Some(off) = t.get("data_offsets").and_then(ints) else {
                continue;
            };
            if off.len() != 2 || off[0] > off[1] {
                continue;
            }
            out.push(Tensor {
                name: name.clone(),
                dtype: dtype.clone(),
                shape,
                begin: off[0],
                end: off[1],
            });
        }
        out
    }

    /// Optional `__metadata__` string map.
    pub fn metadata(&self) -> Vec<(String, String)> {
        let Ok(Json::Obj(map)) = json::parse(self.header) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if let Some(Json::Obj(m)) = map.get("__metadata__") {
            for (k, v) in m {
                if let Json::Str(s) = v {
                    out.push((k.clone(), s.clone()));
                }
            }
        }
        out
    }

    /// The tensor's bytes inside the data blob.
    pub fn tensor_data(&self, t: &Tensor) -> Option<&'a [u8]> {
        let lo = self.data_at.checked_add(usize::try_from(t.begin).ok()?)?;
        let hi = self.data_at.checked_add(usize::try_from(t.end).ok()?)?;
        self.d.get(lo..hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let h = br#"{"a":{"dtype":"I64","shape":[3],"data_offsets":[0,24]},"w":{"dtype":"F32","shape":[2,2],"data_offsets":[24,40]},"__metadata__":{"fmt":"pt"}}"#;
        let mut d = Vec::new();
        d.extend_from_slice(&u64::to_le_bytes(h.len() as u64));
        d.extend_from_slice(h);
        d.extend_from_slice(&(0u8..40).collect::<Vec<u8>>());
        d
    }

    #[test]
    fn fields() {
        let d = fixture();
        let s = parse(&d).unwrap();
        assert!(s.data_at > PREFIX);
        let ts = s.tensors();
        assert_eq!(ts.len(), 2);
        assert_eq!(ts[0].name, "a");
        assert_eq!(ts[0].dtype, "I64");
        assert_eq!((ts[0].begin, ts[0].end), (0, 24));
        assert_eq!(s.tensor_data(&ts[0]).unwrap().len(), 24);
        let md = s.metadata();
        assert_eq!(md.len(), 1);
        assert_eq!(md[0].0, "fmt");
        assert!(s.header().is_some());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&u64::MAX.to_le_bytes()).is_none());
        let mut d = fixture();
        d.truncate(4);
        assert!(parse(&d).is_none());
    }
}
