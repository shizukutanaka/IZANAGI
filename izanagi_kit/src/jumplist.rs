//! Windows JumpList (`*.automaticDestinations-ms`) container scan.
//!
//! An automatic-destinations file is an OLE compound document whose
//! streams are `DestList` plus one hex-numbered stream per entry.
//! This module layers on `ole::parse` and reports the stream
//! inventory a forensic examiner needs: entry ids and whether
//! `DestList` is present.
//!
//! ```
//! // fabricate via a fake ole is heavy; exercise the classifier:
//! use izanagi_kit::jumplist;
//! assert_eq!(jumplist::hex_stream_id("1a2b3c4d"), Some(0x1a2b3c4d));
//! assert_eq!(jumplist::hex_stream_id("DestList"), None);
//! ```

use std::vec::Vec;

/// A parsed jumplist container.
#[derive(Clone, Debug, PartialEq)]
pub struct JumpList {
    /// Hex-named stream ids (one per jump entry).
    pub entry_ids: Vec<u32>,
    /// Whether a `DestList` stream exists.
    pub has_destlist: bool,
    /// Every stream name found in the container.
    pub stream_names: Vec<std::string::String>,
}

/// Parses an 8-hex-digit stream name to its id.
pub fn hex_stream_id(name: &str) -> Option<u32> {
    if name.len() != 8 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(name, 16).ok()
}

/// Scans a parsed OLE container for JumpList structure: streams are
/// `DestList` plus hex-named entries. Works on raw bytes via
/// `crate::ole::parse`.
pub fn parse(d: &[u8]) -> Option<JumpList> {
    let ole = crate::ole::parse(d)?;
    let mut entry_ids = Vec::new();
    let mut has_destlist = false;
    let mut stream_names = Vec::new();
    for e in &ole.entries {
        if e.kind != crate::ole::ENTRY_STREAM {
            continue;
        }
        stream_names.push(e.name.clone());
        if e.name == "DestList" {
            has_destlist = true;
        }
        if let Some(id) = hex_stream_id(&e.name) {
            entry_ids.push(id);
        }
    }
    Some(JumpList {
        entry_ids,
        has_destlist,
        stream_names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_ids() {
        assert_eq!(hex_stream_id("0000000a"), Some(10));
        assert_eq!(hex_stream_id("ffffffff"), Some(0xFFFFFFFF));
        assert_eq!(hex_stream_id("xyz"), None);
        assert_eq!(hex_stream_id("1234567"), None);
    }

    #[test]
    fn rejects_non_ole() {
        assert!(parse(&[0u8; 512]).is_none());
    }
}
