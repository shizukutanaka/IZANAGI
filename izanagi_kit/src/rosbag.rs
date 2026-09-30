//! ROS bag v1.2 / v2.0 — sequential record stream with length-prefixed fields.
//!
//! Layout: `#ROSBAG V` text line, then records of
//! `u32 header_len | header fields | u32 data_len | data`, where each header
//! field is `u32 len | key=value` and the first field is always `op=\xNN`.
//!
//! ```
//! let mut d = b"#ROSBAG V1\x2e2\n".to_vec();
//! let mut rec = Vec::new();
//! rec.extend_from_slice(&15u32.to_le_bytes()); // header_len = 8 + 7
//! rec.extend_from_slice(&4u32.to_le_bytes());
//! rec.extend_from_slice(b"op=\x02"); // MSG_DATA
//! rec.extend_from_slice(&3u32.to_le_bytes());
//! rec.extend_from_slice(b"a=b");
//! rec.extend_from_slice(&0u32.to_le_bytes()); // data_len
//! d.extend_from_slice(&rec);
//! let b = izanagi_kit::rosbag::parse(&d).unwrap();
//! assert_eq!((b.major, b.minor), (1, 2));
//! assert_eq!(b.records, 1);
//! ```

/// Parsed ROS bag header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rosbag {
    /// Major version digit (`1` or `2`).
    pub major: u8,
    /// Minor version digit.
    pub minor: u8,
    /// Records successfully walked before EOF/truncation.
    pub records: usize,
    /// `conn_count` field of the v2.0 bag-header record, if present.
    pub conn_count: Option<u32>,
    /// `chunk_count` field of the v2.0 bag-header record, if present.
    pub chunk_count: Option<u32>,
}

fn le32(d: &[u8], o: usize) -> Option<u32> {
    let b: [u8; 4] = d.get(o..o + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(b))
}

/// Parse the `#ROSBAG Vx.y` stream; `None` on bad magic or a corrupt first record.
pub fn parse(d: &[u8]) -> Option<Rosbag> {
    if !d.starts_with(b"#ROSBAG V") || d.len() < 13 {
        return None;
    }
    let nl = d[..64.min(d.len())].iter().position(|&c| c == b'\n')?;
    let line = &d[9..nl];
    if line.len() != 3 || line[1] != b'.' || !line[0].is_ascii_digit() || !line[2].is_ascii_digit()
    {
        return None;
    }
    let (major, minor) = (line[0] - b'0', line[2] - b'0');
    if !(major == 1 || major == 2) {
        return None;
    }

    let mut pos = nl + 1;
    let mut records = 0usize;
    let mut conn_count = None;
    let mut chunk_count = None;
    while pos + 4 <= d.len() {
        let hlen = le32(d, pos)? as usize;
        pos += 4;
        let hend = pos.checked_add(hlen)?;
        if hend + 4 > d.len() {
            break;
        }
        let mut op = None;
        let mut p = pos;
        while p + 4 <= hend {
            let flen = le32(d, p)? as usize;
            p += 4;
            let f = d.get(p..p + flen)?;
            if let Some(eq) = f.iter().position(|&c| c == b'=') {
                let (k, v) = (&f[..eq], &f[eq + 1..]);
                if k == b"op" && v.len() == 1 {
                    op = Some(v[0]);
                } else if k == b"conn_count" && v.len() == 4 {
                    conn_count = Some(u32::from_le_bytes(v.try_into().ok()?));
                } else if k == b"chunk_count" && v.len() == 4 {
                    chunk_count = Some(u32::from_le_bytes(v.try_into().ok()?));
                }
            }
            p += flen;
        }
        op?;
        let dlen = le32(d, hend)? as usize;
        pos = hend + 4 + dlen;
        if pos > d.len() {
            break;
        }
        records += 1;
    }
    if records == 0 {
        return None;
    }
    Some(Rosbag {
        major,
        minor,
        records,
        conn_count,
        chunk_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(fields: &[&[u8]], data: &[u8]) -> Vec<u8> {
        let mut f = Vec::new();
        for kv in fields {
            f.extend_from_slice(&(kv.len() as u32).to_le_bytes());
            f.extend_from_slice(kv);
        }
        let mut r = Vec::new();
        r.extend_from_slice(&(f.len() as u32).to_le_bytes());
        r.extend_from_slice(&f);
        r.extend_from_slice(&(data.len() as u32).to_le_bytes());
        r.extend_from_slice(data);
        r
    }

    #[test]
    fn v2_bag() {
        let mut d = b"#ROSBAG V2\x2e0\n".to_vec();
        let mut hdr = vec![b"op=\x03".as_slice()];
        hdr.push(b"index_pos=12345678");
        let mut cc = b"conn_count=".to_vec();
        cc.extend_from_slice(&7u32.to_le_bytes());
        hdr.push(&cc);
        let mut ck = b"chunk_count=".to_vec();
        ck.extend_from_slice(&2u32.to_le_bytes());
        hdr.push(&ck);
        d.extend_from_slice(&rec(&hdr, &[]));
        d.extend_from_slice(&rec(&[b"op=\x05"], &[1, 2, 3]));
        let b = parse(&d).unwrap();
        assert_eq!((b.major, b.minor), (2, 0));
        assert_eq!(b.records, 2);
        assert_eq!(b.conn_count, Some(7));
        assert_eq!(b.chunk_count, Some(2));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"#ROSBAG V9\x2e9\n").is_none()); // bad version
        assert!(parse(b"#ROSBAG V2\x2e0\n").is_none()); // no records
                                                        // corrupt first record (header_len past EOF)
        let mut d = b"#ROSBAG V1\x2e2\n".to_vec();
        d.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&d).is_none());
    }
}
