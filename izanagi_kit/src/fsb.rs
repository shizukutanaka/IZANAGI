//! FMOD Sound Bank FSB5 header parsing.
//!
//! `FSB5` magic, then i32LE: `version`, `num_samples`,
//! `sample_header_size`, `name_table_size`, `data_size`, `mode`.
//! Fixed header is 60 bytes; the sample-header/name/data regions
//! must fit the input.
//!
//! ```
//! use izanagi_kit::fsb;
//! let mut d = b"FSB5".to_vec();
//! d.extend_from_slice(&1i32.to_le_bytes()); // version
//! d.extend_from_slice(&2i32.to_le_bytes()); // num_samples
//! d.extend_from_slice(&16i32.to_le_bytes()); // sample_header_size
//! d.extend_from_slice(&8i32.to_le_bytes()); // name_table_size
//! d.extend_from_slice(&40i32.to_le_bytes()); // data_size
//! d.extend_from_slice(&0i32.to_le_bytes()); // mode
//! d.extend_from_slice(&[0i32; 8].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>());
//! d.extend_from_slice(&[0u8; 64]); // headers+names+data
//! let f = fsb::parse(&d).unwrap();
//! assert_eq!(f.num_samples, 2);
//! ```

/// FSB5 magic.
pub const MAGIC: &[u8; 4] = b"FSB5";

/// Fixed FSB5 header length.
pub const HEADER_LEN: usize = 60;

/// A parsed FSB5 header.
#[derive(Clone, Debug, PartialEq)]
pub struct Fsb {
    /// Container `version` (typically 1).
    pub version: i32,
    /// Number of sample entries.
    pub num_samples: i32,
    /// Byte size of the sample-header region.
    pub sample_header_size: i32,
    /// Byte size of the name table.
    pub name_table_size: i32,
    /// Byte size of the audio-data region.
    pub data_size: i32,
    /// Mode flags (FSOUND_FSB5_...).
    pub mode: i32,
    /// Offset of the sample headers.
    pub headers_offset: usize,
    /// Offset of the name table.
    pub names_offset: usize,
    /// Offset of the audio data.
    pub data_offset: usize,
}

fn i32le(d: &[u8], at: usize) -> Option<i32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24) as i32)
}

/// Parses an FSB5 header; the three regions must fit the input.
pub fn parse(d: &[u8]) -> Option<Fsb> {
    if d.len() < HEADER_LEN {
        return None;
    }
    if d.get(..4)? != MAGIC {
        return None;
    }
    let version = i32le(d, 4)?;
    let num_samples = i32le(d, 8)?;
    let sample_header_size = i32le(d, 12)?;
    let name_table_size = i32le(d, 16)?;
    let data_size = i32le(d, 20)?;
    let mode = i32le(d, 24)?;
    for v in [num_samples, sample_header_size, name_table_size, data_size] {
        if v < 0 {
            return None;
        }
    }
    let headers_offset = HEADER_LEN;
    let names_offset = headers_offset.checked_add(sample_header_size as usize)?;
    let data_offset = names_offset.checked_add(name_table_size as usize)?;
    if data_offset.checked_add(data_size as usize)? > d.len() {
        return None;
    }
    Some(Fsb {
        version,
        num_samples,
        sample_header_size,
        name_table_size,
        data_size,
        mode,
        headers_offset,
        names_offset,
        data_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        for v in [1i32, 3, 16, 8, 40, 0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(
            &[0i32; 8]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<u8>>(),
        );
        d.extend_from_slice(&[0u8; 16 + 8 + 40]);
        d
    }

    #[test]
    fn parses_header() {
        let f = parse(&fixture()).unwrap();
        assert_eq!(f.num_samples, 3);
        assert_eq!(f.data_offset, 60 + 16 + 8);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 20]).is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[20..24].copy_from_slice(&9999i32.to_le_bytes()); // data_size too big
        assert!(parse(&d).is_none());
    }
}
