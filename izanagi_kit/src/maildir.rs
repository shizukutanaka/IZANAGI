//! Maildir message filename decoding (Dan Bernstein's maildir format,
//! `https://cr.yp.to/proto/maildir.html`).
//!
//! A delivered message lives in `new/` or `cur/`. In `cur/` the name is
//! `unique:2,<flags>` where flags are a canonical, alphabetically-sorted
//! subset of `D`raft, `F`lagged, `P`assed, `R`eplied, `S`een, `T`rashed.
//! Messages in `new/` and `tmp/` carry no info suffix.
//!
//! ```
//! use izanagi_kit::maildir::{self, Flag};
//! let n = maildir::parse_name(b"12345.M1P2Q3.host:2,FS").unwrap();
//! assert!(n.flags.contains(&Flag::Flagged));
//! assert!(n.flags.contains(&Flag::Seen));
//! ```

use std::vec::Vec;

/// A maildir message flag (`:2,` info section).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flag {
    /// `D` — draft.
    Draft,
    /// `F` — flagged.
    Flagged,
    /// `P` — passed (forwarded/bounced).
    Passed,
    /// `R` — replied.
    Replied,
    /// `S` — seen.
    Seen,
    /// `T` — trashed.
    Trashed,
}

impl Flag {
    fn from_byte(b: u8) -> Option<Flag> {
        match b {
            b'D' => Some(Flag::Draft),
            b'F' => Some(Flag::Flagged),
            b'P' => Some(Flag::Passed),
            b'R' => Some(Flag::Replied),
            b'S' => Some(Flag::Seen),
            b'T' => Some(Flag::Trashed),
            _ => None,
        }
    }
}

/// A decoded maildir file name.
#[derive(Clone, Debug, PartialEq)]
pub struct Name {
    /// The unique-name portion (everything before `:2,` or the whole name).
    pub unique: Vec<u8>,
    /// Info flags from `cur/`-style names; empty for `new/`/`tmp/` names.
    pub flags: Vec<Flag>,
}

/// Parses a message's base file name (the part inside `new/`/`cur/`).
///
/// Accepts both `unique` (new/tmp) and `unique:2,<flags>` (cur). Other
/// experimental info versions (`:1,`) are tolerated verbatim; an empty
/// unique part or a non-flag byte in the flag section rejects.
pub fn parse_name(name: &[u8]) -> Option<Name> {
    if name.is_empty() {
        return None;
    }
    let mut unique_end = name.len();
    let mut flags = Vec::new();
    for (i, &b) in name.iter().enumerate() {
        if b == b':' {
            unique_end = i;
            let rest = &name[i + 1..];
            let comma = rest.iter().position(|&c| c == b',')?;
            let version = &rest[..comma];
            if version.is_empty() || !version.iter().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let fs = &rest[comma + 1..];
            let mut prev = 0u8;
            for &f in fs {
                let fl = Flag::from_byte(f)?;
                // Canonical order is alphabetical.
                if f < prev {
                    return None;
                }
                prev = f;
                flags.push(fl);
            }
            break;
        }
    }
    if unique_end == 0 {
        return None;
    }
    Some(Name {
        unique: name[..unique_end].to_vec(),
        flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cur_name_with_flags() {
        let n = parse_name(b"1700000000.1234.host,U=42:2,DFPRST").unwrap();
        assert_eq!(n.flags.len(), 6);
        assert_eq!(n.flags[0], Flag::Draft);
        assert_eq!(n.unique, b"1700000000.1234.host,U=42".to_vec());
    }

    #[test]
    fn new_name_without_info() {
        let n = parse_name(b"1700000000.5678.host").unwrap();
        assert!(n.flags.is_empty());
    }

    #[test]
    fn noncanonical_flag_order_rejected() {
        assert!(parse_name(b"u:2,SF").is_none());
        assert!(parse_name(b"u:2,X").is_none());
        assert!(parse_name(b":2,S").is_none());
        assert!(parse_name(b"u:").is_none());
        assert!(parse_name(b"").is_none());
    }

    #[test]
    fn every_flag_maps() {
        assert_eq!(Flag::from_byte(b'P'), Some(Flag::Passed));
        assert_eq!(Flag::from_byte(b'z'), None);
    }
}
