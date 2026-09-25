//! Bencode — the serialization of BitTorrent (BEP 3).
//!
//! Four value kinds: integers `i42e`, byte strings `4:spam`, lists
//! `l...e`, dictionaries `d...e`. Parsing is total ([`decode`] returns
//! `None` on malformed input); [`encode`] is canonical — dictionary keys
//! emit in lexicographic byte order regardless of insertion order, so
//! `encode∘parse` normalizes any encoding the spec forbids (unsorted
//! keys, `-0`, leading zeros). Parsing itself is lenient about key
//! order and rejects only structurally broken input.
//!
//! ```
//! use izanagi_kit::bencode::{Ben, decode, encode};
//! let b = decode(b"i42e").unwrap();
//! assert_eq!(b, Ben::Int(42));
//! assert_eq!(encode(&b), b"i42e");
//! ```

use std::collections::BTreeMap;

/// A bencode value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ben {
    /// `i<integer>e`
    Int(i64),
    /// `<len>:<bytes>` — arbitrary bytes, not necessarily UTF-8.
    Bytes(Vec<u8>),
    /// `l<items>e`
    List(Vec<Ben>),
    /// `d<key><value>...e` — keys are byte strings.
    Dict(BTreeMap<Vec<u8>, Ben>),
}

const MAX_DEPTH: usize = 512;

/// Decode a whole buffer; trailing bytes reject (`Some` iff total).
pub fn decode(d: &[u8]) -> Option<Ben> {
    let (v, used) = decode_prefix(d)?;
    if used == d.len() {
        Some(v)
    } else {
        None
    }
}

/// Decode one value from the front, returning `(value, bytes_used)`.
pub fn decode_prefix(d: &[u8]) -> Option<(Ben, usize)> {
    let v = value(d, 0, 0)?;
    Some((v.0, v.1))
}

fn value(d: &[u8], i: usize, depth: usize) -> Option<(Ben, usize)> {
    if depth > MAX_DEPTH {
        return None;
    }
    match *d.get(i)? {
        b'i' => {
            let e = find(d, i + 1, b'e')?;
            let s = std::str::from_utf8(&d[i + 1..e]).ok()?;
            // No "-0", no leading zeros, at least one digit.
            if s.is_empty()
                || (s.len() > 1 && s.starts_with('0'))
                || s == "-0"
                || (s.starts_with('-') && s.len() == 1)
            {
                return None;
            }
            let n: i64 = s.parse().ok()?;
            Some((Ben::Int(n), e + 1))
        }
        b'l' => {
            let mut items = Vec::new();
            let mut j = i + 1;
            while *d.get(j)? != b'e' {
                let (v, n) = value(d, j, depth + 1)?;
                items.push(v);
                j = n;
            }
            Some((Ben::List(items), j + 1))
        }
        b'd' => {
            let mut m = BTreeMap::new();
            let mut j = i + 1;
            while *d.get(j)? != b'e' {
                let (k, n) = byte_string(d, j)?;
                let (v, n2) = value(d, n, depth + 1)?;
                m.insert(k, v);
                j = n2;
            }
            Some((Ben::Dict(m), j + 1))
        }
        b'0'..=b'9' => {
            let (b, n) = byte_string(d, i)?;
            Some((Ben::Bytes(b), n))
        }
        _ => None,
    }
}

fn byte_string(d: &[u8], i: usize) -> Option<(Vec<u8>, usize)> {
    let e = find(d, i, b':')?;
    let s = std::str::from_utf8(&d[i..e]).ok()?;
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) {
        return None;
    }
    let len: usize = s.parse().ok()?;
    let end = (e + 1).checked_add(len)?;
    if end > d.len() {
        return None;
    }
    Some((d[e + 1..end].to_vec(), end))
}

fn find(d: &[u8], from: usize, c: u8) -> Option<usize> {
    d[from..].iter().position(|&b| b == c).map(|p| from + p)
}

/// Canonical encoding: dictionary keys sorted by raw bytes.
pub fn encode(v: &Ben) -> Vec<u8> {
    let mut out = Vec::new();
    emit(v, &mut out);
    out
}

fn emit(v: &Ben, out: &mut Vec<u8>) {
    match v {
        Ben::Int(n) => {
            out.push(b'i');
            out.extend_from_slice(n.to_string().as_bytes());
            out.push(b'e');
        }
        Ben::Bytes(b) => {
            out.extend_from_slice(b.len().to_string().as_bytes());
            out.push(b':');
            out.extend_from_slice(b);
        }
        Ben::List(items) => {
            out.push(b'l');
            for it in items {
                emit(it, out);
            }
            out.push(b'e');
        }
        Ben::Dict(m) => {
            out.push(b'd');
            for (k, val) in m {
                emit(&Ben::Bytes(k.clone()), out);
                emit(val, out);
            }
            out.push(b'e');
        }
    }
}

impl Ben {
    /// Borrow a `Bytes` payload as UTF-8, if it is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Ben::Bytes(b) => std::str::from_utf8(b).ok(),
            _ => None,
        }
    }

    /// Look up `key` in a `Dict`; `None` for other kinds.
    pub fn get(&self, key: &str) -> Option<&Ben> {
        match self {
            Ben::Dict(m) => m.get(key.as_bytes()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_examples() {
        assert_eq!(decode(b"i42e"), Some(Ben::Int(42)));
        assert_eq!(decode(b"i-3e"), Some(Ben::Int(-3)));
        assert_eq!(decode(b"4:spam"), Some(Ben::Bytes(b"spam".to_vec())));
        assert_eq!(decode(b"0:"), Some(Ben::Bytes(Vec::new())));
        assert_eq!(
            decode(b"l4:spami42ee"),
            Some(Ben::List(vec![Ben::Bytes(b"spam".to_vec()), Ben::Int(42)]))
        );
        let d = decode(b"d3:bar4:spam3:fooi42ee").unwrap();
        assert_eq!(d.get("foo"), Some(&Ben::Int(42)));
        assert_eq!(d.get("bar").and_then(Ben::as_str), Some("spam"));
    }

    #[test]
    fn malformed_inputs_rejected() {
        for bad in [
            &b""[..],
            b"i03e",
            b"i-0e",
            b"ie",
            b"i-e",
            b"i4x2e",
            b"i42",
            b"4spa",
            b"04:a",
            b"-1:a",
            b"4:sp",
            b"3:spam",
            b"l4:spam",
            b"d1:a",
            b"xi42e",
            b"i42ejunk",
        ] {
            assert!(decode(bad).is_none(), "{bad:?}");
        }
        // Unsorted dict keys parse (lenient) but emit canonical.
        let d = decode(b"d1:bi1e1:ai2ee").unwrap();
        assert_eq!(encode(&d), b"d1:ai2e1:bi1ee");
    }

    #[test]
    fn torrent_shape_roundtrip() {
        let doc = decode(
            b"d8:announce20:http://tracker.test/4:infod6:lengthi1024e4:name8:file.bin12:piece lengthi16384e6:pieces20:aaaaaaaaaaaaaaaaaaaaee",
        )
        .unwrap();
        assert_eq!(
            doc.get("info").unwrap().get("length"),
            Some(&Ben::Int(1024))
        );
        assert!(!encode(&doc).is_empty());
        assert_eq!(decode(&encode(&doc)), Some(doc));
    }
}
