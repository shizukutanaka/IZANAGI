//! BitTorrent metainfo (`.torrent`) — a bencoded dictionary (BEP 3).
//!
//! The whole file is one dict: `announce` (tracker URL), optional
//! `announce-list` (tiers of URLs), and `info` — `name`, `piece length`,
//! `pieces` (concatenated 20-byte hashes), and either `length` (single
//! file) or `files` (`{length, path}` dicts, one per file).
//!
//! ```
//! use izanagi_kit::torrent::parse;
//!
//! let t = parse(b"d8:announce17:http://tr.example4:infod6:lengthi4e4:name5:a.txt12:piece lengthi16384e6:pieces20:aaaaaaaaaaaaaaaaaaaaee").unwrap();
//! assert_eq!(t.name, "a.txt");
//! assert_eq!(t.total_length(), 4);
//! ```

use crate::bencode::{decode, Ben};

/// One file entry (multi-file torrents get one per `files` item).
#[derive(Clone, Debug)]
pub struct TorrentFile {
    /// Path components relative to `name`.
    pub path: Vec<String>,
    /// File size in bytes.
    pub length: u64,
}

/// A parsed `.torrent` file.
#[derive(Clone, Debug)]
pub struct Torrent {
    /// Primary tracker URL.
    pub announce: Option<String>,
    /// Every URL across all `announce-list` tiers.
    pub announce_list: Vec<String>,
    /// `info.name` — the file or top directory name.
    pub name: String,
    /// Bytes per piece.
    pub piece_length: u64,
    /// Raw concatenated SHA-1 piece hashes (always a multiple of 20).
    pub pieces: Vec<u8>,
    /// Files; single-file torrents produce one entry with `path == [name]`.
    pub files: Vec<TorrentFile>,
}

impl Torrent {
    /// Total bytes across all files.
    pub fn total_length(&self) -> u64 {
        self.files.iter().map(|f| f.length).sum()
    }
}

fn int(b: &Ben) -> Option<u64> {
    match b {
        Ben::Int(n) => u64::try_from(*n).ok(),
        _ => None,
    }
}

fn bytes(b: &Ben) -> Option<&[u8]> {
    match b {
        Ben::Bytes(v) => Some(v),
        _ => None,
    }
}

/// Parse a metainfo file. `None` on non-bencode, missing `info`, wrong
/// `pieces` width, or a multi-file entry without `length`/`path`.
pub fn parse(d: &[u8]) -> Option<Torrent> {
    let doc = decode(d)?;
    let info = doc.get("info")?;
    let name = info.get("name")?.as_str()?.to_string();
    let piece_length = int(info.get("piece length")?)?;
    let pieces = bytes(info.get("pieces")?)?.to_vec();
    if pieces.is_empty() || pieces.len() % 20 != 0 {
        return None;
    }
    let mut files = Vec::new();
    match info.get("files") {
        Some(Ben::List(items)) => {
            for it in items {
                let length = int(it.get("length")?)?;
                let mut path = Vec::new();
                match it.get("path")? {
                    Ben::List(parts) => {
                        for p in parts {
                            path.push(p.as_str()?.to_string());
                        }
                    }
                    _ => return None,
                }
                if path.is_empty() {
                    return None;
                }
                files.push(TorrentFile { path, length });
            }
            if files.is_empty() {
                return None;
            }
        }
        Some(_) => return None,
        None => {
            let length = int(info.get("length")?)?;
            files.push(TorrentFile {
                path: vec![name.clone()],
                length,
            });
        }
    }
    let mut announce_list = Vec::new();
    if let Some(Ben::List(tiers)) = doc.get("announce-list") {
        for tier in tiers {
            if let Ben::List(urls) = tier {
                for u in urls {
                    if let Some(s) = u.as_str() {
                        announce_list.push(s.to_string());
                    }
                }
            }
        }
    }
    Some(Torrent {
        announce: doc
            .get("announce")
            .and_then(Ben::as_str)
            .map(str::to_string),
        announce_list,
        name,
        piece_length,
        pieces,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINGLE: &[u8] = b"d8:announce17:http://tr.example4:infod6:lengthi4e4:name5:a.txt12:piece lengthi16384e6:pieces20:aaaaaaaaaaaaaaaaaaaaee";

    #[test]
    fn single_file() {
        let t = parse(SINGLE).unwrap();
        assert_eq!(t.announce.as_deref(), Some("http://tr.example"));
        assert_eq!(t.files.len(), 1);
        assert_eq!(t.files[0].path, vec!["a.txt"]);
        assert_eq!(t.pieces.len(), 20);
        assert_eq!(t.piece_length, 16384);
        assert_eq!(t.total_length(), 4);
    }

    #[test]
    fn multi_file() {
        let d = b"d8:announce2:tr13:announce-listll2:t1el2:t2ee4:infod5:filesld6:lengthi3e4:pathl1:deed6:lengthi5e4:pathl3:sub1:feee4:name3:dir12:piece lengthi4e6:pieces20:aaaaaaaaaaaaaaaaaaaaee";
        let t = parse(d).unwrap();
        assert_eq!(t.files.len(), 2);
        assert_eq!(t.files[0].path, vec!["d"]);
        assert_eq!(t.files[1].path, vec!["sub", "f"]);
        assert_eq!(t.announce_list, vec!["t1", "t2"]);
        assert_eq!(t.total_length(), 8);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"d4:infod4:name1:aei0e").is_none());
        // pieces not a multiple of 20
        assert!(parse(b"d4:infod4:name1:a6:pieces2:xz12:piece lengthi4e6:lengthi1eee").is_none());
        // bad bencode
        assert!(parse(b"hello").is_none());
    }
}
