//! ABC — an Alembic *Ogawa* stream head. Byte 0 carries the 8-byte
//! magic `89 4F 67 61 77 61 0D 0A`, then a u16 `frozen` flag and a
//! u16LE format version. Each *group* starts with a u64LE child
//! count followed by that many u64LE child offsets; a group with no
//! children stores a u64 data size and a u64 data offset instead.
//!
//! ```
//! use izanagi_kit::abc::{parse, group};
//! let mut d = vec![0x89, b'O', b'g', b'a', b'w', b'a', 0x0D, 0x0A];
//! d.extend_from_slice(&0u16.to_le_bytes()); // frozen
//! d.extend_from_slice(&1u16.to_le_bytes()); // version
//! d.extend_from_slice(&2u64.to_le_bytes()); // 2 children
//! d.extend_from_slice(&0x100u64.to_le_bytes());
//! d.extend_from_slice(&0x200u64.to_le_bytes());
//! let a = parse(&d).unwrap();
//! assert_eq!(a.version, 1);
//! assert_eq!(group(&d, 12).unwrap().children, vec![0x100, 0x200]);
//! ```

/// The 8-byte Ogawa magic.
pub const MAGIC: &[u8; 8] = b"\x89Ogawa\x0D\x0A";

/// A parsed stream head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Abc {
    /// Non-zero when the stream is frozen (all data written).
    pub frozen: u16,
    /// Ogawa format version.
    pub version: u16,
}

/// One group header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// Child offsets when non-empty.
    pub children: Vec<u64>,
    /// For leaf groups: `(data_offset, data_size)`.
    pub data: Option<(u64, u64)>,
}

fn u16l(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u64l(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Parse magic + frozen + version.
pub fn parse(d: &[u8]) -> Option<Abc> {
    if d.get(..8)? != MAGIC {
        return None;
    }
    Some(Abc {
        frozen: u16l(d, 8)?,
        version: u16l(d, 10)?,
    })
}

/// Read the group header at `at`: a u64 child count `n`; when `n` is
/// 0xff... the next two words are `(data_size, data_offset)` for a
/// leaf, otherwise `n` u64 offsets follow.
pub fn group(d: &[u8], at: usize) -> Option<Group> {
    let n = u64l(d, at)?;
    if n == u64::MAX {
        return Some(Group {
            children: Vec::new(),
            data: Some((u64l(d, at + 8)?, u64l(d, at + 16)?)),
        });
    }
    let n = usize::try_from(n).ok()?;
    let mut children = Vec::with_capacity(n.min(1024));
    for i in 0..n {
        children.push(u64l(d, at + 8 + i.checked_mul(8)?)?);
    }
    Some(Group {
        children,
        data: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&0u16.to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&2u64.to_le_bytes());
        d.extend_from_slice(&0x100u64.to_le_bytes());
        d.extend_from_slice(&0x200u64.to_le_bytes());
        d
    }

    #[test]
    fn parses_head_and_group() {
        let d = fixture();
        let a = parse(&d).unwrap();
        assert_eq!(a.version, 1);
        assert_eq!(a.frozen, 0);
        let g = group(&d, 12).unwrap();
        assert_eq!(g.children, vec![0x100, 0x200]);
        assert!(g.data.is_none());
    }

    #[test]
    fn leaf_group() {
        let mut d = fixture();
        let at = d.len();
        d.extend_from_slice(&u64::MAX.to_le_bytes());
        d.extend_from_slice(&64u64.to_le_bytes());
        d.extend_from_slice(&0x400u64.to_le_bytes());
        let g = group(&d, at).unwrap();
        assert_eq!(g.data, Some((64, 0x400)));
        assert!(g.children.is_empty());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[0] = 0;
        assert!(parse(&d).is_none());
        // child offset past EOF
        assert!(group(&d[..20], 12).is_none());
    }
}
