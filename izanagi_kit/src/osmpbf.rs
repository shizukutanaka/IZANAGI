//! OSM PBF — OpenStreetMap's protobuf stream: `u32 be header_len |
//! BlobHeader (type="OSMHeader"|"OSMData", datasize) | u32? data`.
//!
//! BlobHeader is itself protobuf: field 1 `type` string, field 3
//! `datasize` varint.
//!
//! ```
//! // header_len=11, BlobHeader{ type:"OSMHeader", datasize:0 }
//! let d = [
//!     0, 0, 0, 13,                    // blobheader len = 13
//!     0x0A, 9, b'O',b'S',b'M',b'H',b'e',b'a',b'd',b'e',b'r',
//!     0x18, 0,                        // datasize 0
//!     0, 0, 0, 0,                     // blob len 0
//! ];
//! let p = izanagi_kit::osmpbf::parse(&d).unwrap();
//! assert_eq!(p.blob_type, "OSMHeader");
//! ```

/// First BlobHeader of an OSM PBF stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsmPbf {
    /// `type` field — `OSMHeader` for the first block, `OSMData` for data blocks.
    pub blob_type: String,
    /// Declared BlobHeader byte length.
    pub header_len: u32,
    /// `datasize` — byte length of the following Blob.
    pub data_size: u32,
    /// `indexdata` field present (rarely used).
    pub has_indexdata: bool,
}

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        ((*d.get(o)? as u32) << 24)
            | ((*d.get(o + 1)? as u32) << 16)
            | ((*d.get(o + 2)? as u32) << 8)
            | *d.get(o + 3)? as u32,
    )
}

/// Minimal protobuf field walk over the BlobHeader.
fn blob_header(d: &[u8]) -> Option<OsmPbf> {
    let mut blob_type = String::new();
    let mut data_size = 0u32;
    let mut has_indexdata = false;
    let mut i = 0usize;
    while i < d.len() {
        let key = *d.get(i)?;
        let (field, wire) = (key >> 3, key & 7);
        i += 1;
        match (field, wire) {
            (1, 2) => {
                let n = *d.get(i)? as usize;
                i += 1;
                blob_type = std::str::from_utf8(d.get(i..i + n)?).ok()?.to_string();
                i += n;
            }
            (2, 2) => {
                // indexdata blob — skip
                let n = *d.get(i)? as usize;
                i += 1 + n;
                has_indexdata = true;
            }
            (3, 0) => {
                // varint datasize
                let mut v = 0u64;
                let mut sh = 0u32;
                loop {
                    let b = *d.get(i)? as u64;
                    i += 1;
                    v |= (b & 0x7F) << sh;
                    if b & 0x80 == 0 {
                        break;
                    }
                    sh += 7;
                    if sh > 63 {
                        return None;
                    }
                }
                data_size = v as u32;
            }
            _ => return None,
        }
    }
    Some(OsmPbf {
        blob_type,
        header_len: d.len() as u32,
        data_size,
        has_indexdata,
    })
}

/// Parse the stream head; `None` unless the first blob is `OSMHeader`.
pub fn parse(d: &[u8]) -> Option<OsmPbf> {
    let hlen = be32(d, 0)? as usize;
    if hlen == 0 || hlen > 64 * 1024 {
        return None;
    }
    let h = blob_header(d.get(4..4 + hlen)?)?;
    if h.blob_type != "OSMHeader" {
        return None;
    }
    // the Blob length word must fit
    be32(d, 4 + hlen)?;
    Some(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(body: &[u8]) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&[
            (body.len() >> 24) as u8,
            (body.len() >> 16) as u8,
            (body.len() >> 8) as u8,
            body.len() as u8,
        ]);
        d.extend_from_slice(body);
        d.extend_from_slice(&[0, 0, 0, 0]); // blob len
        d
    }

    #[test]
    fn header_block() {
        let mut bh = vec![0x0A, 9];
        bh.extend_from_slice(b"OSMHeader");
        bh.extend_from_slice(&[0x18, 3]); // datasize varint = 3
        let p = parse(&stream(&bh)).unwrap();
        assert_eq!(p.blob_type, "OSMHeader");
        assert_eq!(p.data_size, 3);
        assert_eq!(p.header_len, 13);
        assert!(!p.has_indexdata);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut bh = vec![0x0A, 7];
        bh.extend_from_slice(b"OSMData");
        assert!(parse(&stream(&bh)).is_none()); // first block must be OSMHeader
        assert!(parse(&[0, 0, 0, 0]).is_none()); // zero header len
        assert!(parse(&[0xFF, 0xFF, 0xFF, 0xFF]).is_none()); // absurd len
    }
}
