//! Windows `hiberfil.sys` header parsing.
//!
//! The file opens with a `PO_MEMORY_IMAGE`-style header whose first
//! u32 is the signature: `hibr`/`HIBR` (valid hibernation image),
//! `wake`/`WAKE` (being woken), `rstr`/`RSTR` (resume in progress),
//! or `0` (cleared). Field offsets differ across Windows versions,
//! so only the signature plus `first_table_page`/`last_checked_page`
//! u32 fields (present since XP's `HIBR` variant) are read.
//!
//! ```
//! use izanagi_kit::hiberfil;
//! let mut d = vec![0u8; 4096];
//! d[0..4].copy_from_slice(b"HIBR");
//! let h = hiberfil::parse(&d).unwrap();
//! assert_eq!(h.state, hiberfil::State::Hibernation);
//! ```

/// Signature-to-state map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// `hibr`/`HIBR` — hibernated image present.
    Hibernation,
    /// `wake`/`WAKE` — image being woken.
    Wake,
    /// `rstr`/`RSTR` — resume in progress.
    Restore,
    /// `0x00000000` — cleared header (system boots fresh).
    Cleared,
    /// Unrecognised signature value.
    Unknown(u32),
}

/// A parsed header.
#[derive(Clone, Debug, PartialEq)]
pub struct Hiberfil {
    /// Signature state.
    pub state: State,
    /// Raw signature u32.
    pub signature: u32,
    /// `system_time` FILETIME when the image was written (LE).
    pub system_time: u64,
    /// `first_table_page` (when the header is long enough).
    pub first_table_page: u32,
    /// `last_checked_page`.
    pub last_checked_page: u32,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

fn u64le(d: &[u8], at: usize) -> Option<u64> {
    let s = d.get(at..at.checked_add(8)?)?;
    let mut v = 0u64;
    for (i, &b) in s.iter().enumerate() {
        v |= (b as u64) << (8 * i);
    }
    Some(v)
}

/// Parses a `hiberfil.sys` header: one sector minimum, recognised
/// signature, plus the XP-era `system_time`/table-page fields.
pub fn parse(d: &[u8]) -> Option<Hiberfil> {
    if d.len() < 0x1000 {
        return None;
    }
    let sig = u32le(d, 0)?;
    let state = match sig {
        0x52424948 | 0x72626968 => State::Hibernation, // HIBR | hibr
        0x454B4157 | 0x656B6177 => State::Wake,        // WAKE | wake
        0x52545352 | 0x72747372 => State::Restore,     // RSTR | rstr
        0 => State::Cleared,
        o => State::Unknown(o),
    };
    if matches!(state, State::Unknown(_)) {
        return None;
    }
    Some(Hiberfil {
        state,
        signature: sig,
        system_time: u64le(d, 0x98)?,
        first_table_page: u32le(d, 0xA0).unwrap_or(0),
        last_checked_page: u32le(d, 0xA4).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn parses_states() {
        for (m, st) in [
            (b"HIBR" as &[u8], State::Hibernation),
            (b"hibr", State::Hibernation),
            (b"WAKE", State::Wake),
            (b"RSTR", State::Restore),
        ] {
            let mut d = vec![0u8; 4096];
            d[0..4].copy_from_slice(m);
            assert_eq!(parse(&d).unwrap().state, st);
        }
        assert_eq!(parse(&vec![0u8; 4096]).unwrap().state, State::Cleared);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 2048]).is_none());
        let mut d = vec![0u8; 4096];
        d[0..4].copy_from_slice(b"XXXX");
        assert!(parse(&d).is_none());
    }
}
