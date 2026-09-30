//! NAMD / CHARMM `.dcd` trajectory — Fortran-record binary frames of
//! coordinates or velocities. The first record is an 84-byte header:
//! `u32 len=84`, magic `CORD` (coordinates) or `VELD` (velocities),
//! `NSET` frames, `ISTART`/`NSAVC`, … then the closing `84`. A title
//! record (`NTITLE` + title lines) and an atom-count record follow.
//!
//! `parse` validates the header record shape and reports frame /
//! atom counts plus the magic.
//!
//! ```
//! let mut f = Vec::new();
//! f.extend_from_slice(&84u32.to_le_bytes());
//! f.extend_from_slice(b"CORD");              // magic
//! f.extend_from_slice(&5u32.to_le_bytes());  // NSET = 5 frames
//! f.extend_from_slice(&[0u8; 76]);           // rest of 84-byte header
//! f.extend_from_slice(&84u32.to_le_bytes()); // closing len
//! f.extend_from_slice(&4u32.to_le_bytes());  // title rec len (0 title)
//! f.extend_from_slice(&[0u8; 4]);            // NTITLE=0
//! f.extend_from_slice(&4u32.to_le_bytes());
//! f.extend_from_slice(&4u32.to_le_bytes());  // atom rec len
//! f.extend_from_slice(&100u32.to_le_bytes());// N atoms
//! f.extend_from_slice(&4u32.to_le_bytes());
//! let d = izanagi_kit::dcd::parse(&f).unwrap();
//! assert_eq!(d.nset, 5);
//! assert_eq!(d.natoms, 100);
//! assert_eq!(d.magic, *b"CORD");
//! ```

/// Parsed DCD summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Dcd {
    /// `CORD` or `VELD`.
    pub magic: [u8; 4],
    /// `NSET` — number of frames stored.
    pub nset: u32,
    /// Atom count from the record after the title block, `0` when the
    /// file is too short to carry it.
    pub natoms: u32,
    /// Number of `NTITLE` title lines, `0` when unreadable.
    pub titles: u32,
}

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

/// Parse a `.dcd`; `None` unless the 84-byte `CORD`/`VELD` record is
/// well-formed.
pub fn parse(d: &[u8]) -> Option<Dcd> {
    if d.len() < 92 || le32(d, 0)? != 84 {
        return None;
    }
    let magic: [u8; 4] = d[4..8].try_into().ok()?;
    if magic != *b"CORD" && magic != *b"VELD" {
        return None;
    }
    if le32(d, 88)? != 84 {
        return None;
    }
    let nset = le32(d, 8)?;
    // Title record: u32 len, body (`NTITLE` + titles), u32 len.
    let mut titles = 0;
    let mut natoms = 0;
    if let Some(tlen) = le32(d, 92) {
        let body_end = 96 + tlen as usize;
        if body_end + 4 <= d.len() && tlen >= 4 {
            titles = le32(d, 96).unwrap_or(0); // 'NTITLE' is a 4-byte id + count in CHARMM; keep raw
            let atom = body_end + 4;
            if let Some(alen) = le32(d, atom) {
                if alen == 4 && atom + 12 <= d.len() {
                    natoms = le32(d, atom + 4).unwrap_or(0);
                }
            }
        }
    }
    Some(Dcd {
        magic,
        nset,
        natoms,
        titles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&84u32.to_le_bytes());
        f.extend_from_slice(b"CORD");
        f.extend_from_slice(&7u32.to_le_bytes());
        f.extend_from_slice(&[0u8; 76]);
        f.extend_from_slice(&84u32.to_le_bytes());
        f
    }

    #[test]
    fn basic() {
        let d = parse(&fixture()).unwrap();
        assert_eq!(d.magic, *b"CORD");
        assert_eq!(d.nset, 7);
        assert_eq!(d.natoms, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x54\x00\x00\x00XXXX").is_none()); // bad magic
        let mut f = fixture();
        f[8] = 9; // nset fine, but break closing len
        f[88] = 0;
        assert!(parse(&f).is_none());
    }
}
