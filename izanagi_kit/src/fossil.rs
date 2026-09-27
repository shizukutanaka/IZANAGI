//! Fossil SCM artifact parsing (manifest/control-artifact cards).
//!
//! An artifact is newline-separated cards: `A <md5> <date> <nonce>`,
//! `B <uuid>`, `C <comment>`, `D <date>` (required), `F <name>
//! <uuid> [perm]`, `N <comment>`, `P <uuid>...`, `Q <+|-|*> <uuid>`,
//! `R <md5>`, `T <+tag> <uuid> <value>`, `U <user>` (required),
//! `W <size>` followed by `<size>` bytes of payload, `Z <md5>`
//! (required trailer checksum).
//!
//! ```
//! use izanagi_kit::fossil;
//! let d = b"D 2020-01-01T00:00:00\nU alice\nW 5\nhello\nZ 0000\n";
//! let f = fossil::parse(d).unwrap();
//! assert_eq!(f.date, "2020-01-01T00:00:00");
//! assert_eq!(f.user, "alice");
//! ```

use std::string::String;
use std::vec::Vec;

/// One artifact card.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    /// Card letter (`A`..=`Z`).
    pub letter: u8,
    /// Space-separated fields after the letter.
    pub fields: Vec<String>,
    /// `W` only: payload bytes (after the `<size>` prefix).
    pub payload: Vec<u8>,
}

/// A parsed artifact.
#[derive(Clone, Debug, PartialEq)]
pub struct Fossil {
    /// Cards in file order.
    pub cards: Vec<Card>,
    /// `D` card value (check-in date).
    pub date: String,
    /// `U` card value (user).
    pub user: String,
    /// `P` card values (parents).
    pub parents: Vec<String>,
    /// `F` card filenames (manifest files).
    pub files: Vec<String>,
}

/// Parses a Fossil artifact: known card letters only, requires `D`,
/// `U` and the `Z` trailer; `W` consumes its declared payload.
pub fn parse(d: &[u8]) -> Option<Fossil> {
    let mut cards = Vec::new();
    let (mut date, mut user) = (String::new(), String::new());
    let (mut parents, mut files) = (Vec::new(), Vec::new());
    let mut saw_z = false;
    let mut at = 0usize;
    while at < d.len() {
        let nl = d[at..].iter().position(|&b| b == b'\n');
        let (raw_end, next) = match nl {
            Some(n) => (at + n, at + n + 1),
            None => (d.len(), d.len()),
        };
        let mut line = &d[at..raw_end];
        if line.ends_with(b"\r") {
            line = &line[..line.len() - 1];
        }
        at = next;
        if line.is_empty() {
            continue;
        }
        let letter = line[0];
        if !letter.is_ascii_uppercase() || line.len() < 2 || line[1] != b' ' {
            return None;
        }
        let rest = core::str::from_utf8(&line[2..]).ok()?;
        let fields: Vec<String> = rest.split(' ').map(|s| s.to_string()).collect();
        let mut payload = Vec::new();
        match letter {
            b'D' => date = fields.first().cloned().unwrap_or_default(),
            b'U' => user = fields.first().cloned().unwrap_or_default(),
            b'P' => parents = fields.clone(),
            b'F' => {
                if let Some(f) = fields.first() {
                    files.push(f.clone());
                }
            }
            b'W' => {
                let n: usize = fields.first()?.parse().ok()?;
                let end = at.checked_add(n)?;
                payload = d.get(at..end)?.to_vec();
                at = end;
                if d.get(at) == Some(&b'\n') {
                    at += 1;
                }
            }
            b'Z' => saw_z = true,
            _ => {}
        }
        let z = letter == b'Z';
        cards.push(Card {
            letter,
            fields,
            payload,
        });
        if z {
            break;
        }
    }
    if !saw_z || date.is_empty() || user.is_empty() {
        return None;
    }
    Some(Fossil {
        cards,
        date,
        user,
        parents,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    const FIXTURE: &[u8] =
        b"D 2021-06-01T12:00:00\nF src/main.rs aabbccdd perms\nN comment\nP deadbeef\nU bob\nW 7\ncontent\nZ ff00\n";

    #[test]
    fn parses_artifact() {
        let f = parse(FIXTURE).unwrap();
        assert_eq!(f.user, "bob");
        assert_eq!(f.parents, vec!["deadbeef".to_string()]);
        assert_eq!(f.files, vec!["src/main.rs".to_string()]);
        assert_eq!(f.cards.len(), 7);
        assert_eq!(f.cards[5].payload, b"content".to_vec());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"D 2020\nU x\n").is_none()); // no Z
        assert!(parse(b"1 bad card\n").is_none());
        assert!(parse(b"D 2020\nZ 00\n").is_none()); // no U
                                                     // W payload longer than file
        assert!(parse(b"D 2020\nU x\nW 99\n\nZ 00\n").is_none());
    }
}
