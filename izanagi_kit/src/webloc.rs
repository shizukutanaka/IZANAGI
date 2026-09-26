//! macOS `.webloc` Internet-location files.
//!
//! Modern Finder `.webloc` is a one-key plist: `{URL = "https://..."}`
//! in XML or binary plist form (decoded via [`crate::plist`]). Classic
//! Macintosh `.webloc`/`.url` files instead use Windows-INI shape
//! `[InternetShortcut]\nURL=...`; both are accepted.
//!
//! ```
//! use izanagi_kit::webloc;
//! let x = b"<plist><dict><key>URL</key><string>https://a/b</string></dict></plist>";
//! assert_eq!(webloc::parse(x).unwrap().url, "https://a/b");
//! ```

use crate::plist::{self, Val};
use std::string::String;
use std::vec::Vec;

/// A parsed `.webloc` file.
#[derive(Clone, Debug, PartialEq)]
pub struct Webloc {
    /// The `URL` value.
    pub url: String,
}

fn url_of(v: &Val) -> Option<String> {
    match plist::get(v, "URL")? {
        Val::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn ini_url(d: &[u8]) -> Option<String> {
    // [InternetShortcut]\nURL=... (case-insensitive key, INI comments).
    let mut in_section = false;
    for raw in d.split(|&b| b == b'\n') {
        let mut line = raw;
        while let Some(&c) = line.last() {
            if c == b'\r' || c == b' ' || c == b'\t' {
                line = &line[..line.len() - 1];
            } else {
                break;
            }
        }
        if line.is_empty() || line[0] == b';' || line[0] == b'#' {
            continue;
        }
        if line[0] == b'[' && line[line.len() - 1] == b']' {
            let s = &line[1..line.len() - 1];
            in_section = s.eq_ignore_ascii_case(b"internetshortcut");
            continue;
        }
        if !in_section {
            continue;
        }
        let eq = line.iter().position(|&b| b == b'=')?;
        let key = &line[..eq];
        if key.eq_ignore_ascii_case(b"url") {
            return String::from_utf8(line[eq + 1..].to_vec()).ok();
        }
        return None;
    }
    None
}

/// Parses XML plist, binary plist, or INI `.webloc` forms.
pub fn parse(d: &[u8]) -> Option<Webloc> {
    if let Some(v) = plist::parse_xml(d) {
        if let Some(url) = url_of(&v) {
            return Some(Webloc { url });
        }
    }
    if let Some(b) = plist::parse_bin(d) {
        if let Some(v) = plist::resolve(&b) {
            if let Some(url) = url_of(&v) {
                return Some(Webloc { url });
            }
        }
    }
    let url: Vec<u8> = ini_url(d)?.into_bytes();
    Some(Webloc {
        url: String::from_utf8(url).ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_plist() {
        let d = b"<?xml version=\"1.0\"?><plist version=\"1.0\"><dict>\
                  <key>URL</key><string>https://x/y?z=1</string></dict></plist>";
        assert_eq!(parse(d).unwrap().url, "https://x/y?z=1");
    }

    #[test]
    fn ini_shortcut() {
        let d = b"[InternetShortcut]\r\nURL=https://old.mac/\r\n";
        assert_eq!(parse(d).unwrap().url, "https://old.mac/");
    }

    #[test]
    fn rejects_other() {
        assert!(parse(b"").is_none());
        assert!(parse(b"[InternetShortcut]\nName=x\n").is_none());
        assert!(parse(b"<plist><dict><key>Name</key><string>x</string></dict></plist>").is_none());
    }
}
