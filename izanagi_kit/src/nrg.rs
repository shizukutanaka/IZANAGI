//! Nero image (`.nrg`) — footer + chunk directory.
//!
//! NRG v1 files end in `NERO` + u32 BE offset of the first header;
//! NRG v2 ends in `NER5` + u64 BE. From that offset a chain of
//! `id4 + u32be size + data` chunks runs until `END!`.
//!
//! ```
//! use izanagi_kit::nrg::parse;
//!
//! // "CUES" chunk (4 bytes data) then "END!" — header at offset 8.
//! let mut d = vec![0u8; 8];
//! d.extend_from_slice(b"CUES\x00\x00\x00\x04ab");
//! d.extend_from_slice(b"cdEND!\x00\x00\x00\x00");
//! d.extend_from_slice(b"NERO\x00\x00\x00\x08");
//! let n = parse(&d).unwrap();
//! assert_eq!(n.version, 1);
//! assert_eq!(n.chunks[0].id, "CUES");
//! assert_eq!(n.chunks[1].id, "END!");
//! ```

/// One chunk record in the header chain.
#[derive(Clone, Debug)]
pub struct Chunk {
    /// 4-byte ASCII chunk id (e.g. `CUES`, `DAOX`, `ETN2`, `END!`).
    pub id: String,
    /// Offset of this chunk's data (right after its 8-byte header).
    pub offset: u64,
    /// Declared data length in bytes.
    pub len: u32,
}

/// A parsed `.nrg` file.
#[derive(Clone, Debug)]
pub struct Nrg {
    /// 1 (`NERO` footer) or 2 (`NER5` footer).
    pub version: u8,
    /// Offset where the chunk chain starts.
    pub first_header: u64,
    /// Chunks in file order, including the `END!` terminator.
    pub chunks: Vec<Chunk>,
}

fn u32be(d: &[u8], o: usize) -> Option<u32> {
    let b = d.get(o..o + 4)?;
    Some(((b[0] as u32) << 24) | ((b[1] as u32) << 16) | ((b[2] as u32) << 8) | b[3] as u32)
}

fn u64be(d: &[u8], o: usize) -> Option<u64> {
    let b = d.get(o..o + 8)?;
    let mut v = 0u64;
    for &x in b {
        v = (v << 8) | x as u64;
    }
    Some(v)
}

/// Parse an NRG image trailer + chunk chain. `None` on missing footer,
/// out-of-range header offset, or truncated chunk.
pub fn parse(d: &[u8]) -> Option<Nrg> {
    let n = d.len();
    let (version, first_header) = if n >= 12 && &d[n - 12..n - 8] == b"NER5" {
        (2u8, u64be(d, n - 8)?)
    } else if n >= 8 && &d[n - 8..n - 4] == b"NERO" {
        (1u8, u32be(d, n - 4)? as u64)
    } else {
        return None;
    };
    let mut i = usize::try_from(first_header).ok()?;
    if i + 8 > n {
        return None;
    }
    let mut chunks = Vec::new();
    loop {
        if i + 8 > n {
            return None;
        }
        let id = std::str::from_utf8(d.get(i..i + 4)?).ok()?.to_string();
        let len = u32be(d, i + 4)?;
        let data_off = i + 8;
        let end = data_off.checked_add(len as usize)?;
        if end > n {
            return None;
        }
        let terminal = id == "END!";
        chunks.push(Chunk {
            id,
            offset: data_off as u64,
            len,
        });
        if terminal {
            break;
        }
        i = end;
    }
    Some(Nrg {
        version,
        first_header,
        chunks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img_v2() -> Vec<u8> {
        let mut d = vec![0u8; 16];
        d.extend_from_slice(b"ABCD\x00\x00\x00\x02xy");
        d.extend_from_slice(b"END!\x00\x00\x00\x00");
        let off = 16u64.to_be_bytes();
        d.extend_from_slice(b"NER5");
        d.extend_from_slice(&off);
        d
    }

    #[test]
    fn v1_and_v2() {
        let n2 = parse(&img_v2()).unwrap();
        assert_eq!(n2.version, 2);
        assert_eq!(n2.first_header, 16);
        assert_eq!(n2.chunks.len(), 2);
        assert_eq!(n2.chunks[0].len, 2);
        assert_eq!(n2.chunks[0].offset, 24);
        let mut d = vec![0u8; 8];
        d.extend_from_slice(b"END!\x00\x00\x00\x00");
        d.extend_from_slice(b"NERO\x00\x00\x00\x08");
        let n1 = parse(&d).unwrap();
        assert_eq!(n1.version, 1);
        assert_eq!(n1.chunks[0].id, "END!");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&img_v2()[..8]).is_none());
        // header offset beyond data
        let mut d = vec![0u8; 8];
        d.extend_from_slice(b"NERO\xff\xff\xff\xff");
        assert!(parse(&d).is_none());
        // truncated chunk
        let mut d2 = vec![0u8; 4];
        d2.extend_from_slice(b"ABCD\x00\x00\x00\xff");
        d2.extend_from_slice(b"NERO\x00\x00\x00\x04");
        assert!(parse(&d2).is_none());
    }
}
