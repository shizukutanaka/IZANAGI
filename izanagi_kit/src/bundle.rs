//! Git bundle file header parsing.
//!
//! `# v2 git bundle` or `# v3 git bundle` signature line, then
//! prerequisite lines `-<hex-sha> [comment]` (v2+), then reference
//! lines `<hex-sha> <refname>`, then a blank line followed by `PACK`
//! data. v3 also allows `@object-format=sha256` capability lines.
//!
//! ```
//! use izanagi_kit::bundle;
//! let d = b"# v2 git bundle\n-0123456789abcdef0123456789abcdef01234567 base\n0123456789abcdef0123456789abcdef01234567 refs/heads/main\n\nPACK";
//! let b = bundle::parse(d).unwrap();
//! assert_eq!(b.version, 2);
//! assert_eq!(b.refs[0].1, "refs/heads/main");
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed bundle header.
#[derive(Clone, Debug, PartialEq)]
pub struct Bundle {
    /// Bundle format `version` (2 or 3).
    pub version: u32,
    /// Prerequisite SHAs (`-<sha> [comment]`).
    pub prerequisites: Vec<String>,
    /// `(sha, refname)` reference list.
    pub refs: Vec<(String, String)>,
    /// Byte offset of the `PACK` payload.
    pub pack_offset: usize,
}

fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Parses a bundle header: version line, `-sha` prerequisites,
/// `<sha> <ref>` references, blank line, `PACK`.
pub fn parse(d: &[u8]) -> Option<Bundle> {
    let text = std::str::from_utf8(d).ok()?;
    let first = text.lines().next()?;
    let version: u32 = first
        .strip_prefix("# v")
        .and_then(|r| r.strip_suffix(" git bundle"))?
        .trim()
        .parse()
        .ok()?;
    if !(2..=3).contains(&version) {
        return None;
    }
    let mut prerequisites = Vec::new();
    let mut refs = Vec::new();
    let mut pack_offset = text.len();
    let mut at = first.len() + 1;
    let bytes = text.as_bytes();
    let mut in_refs = false;
    while at < bytes.len() {
        let nl = bytes[at..].iter().position(|&b| b == b'\n');
        let (end, next) = match nl {
            Some(n) => (at + n, at + n + 1),
            None => (bytes.len(), bytes.len()),
        };
        let line = core::str::from_utf8(&bytes[at..end]).ok()?;
        at = next;
        if line.is_empty() {
            if bytes.get(at..at + 4) == Some(b"PACK") {
                pack_offset = at;
            }
            break;
        }
        if let Some(rest) = line.strip_prefix('-') {
            // prerequisite: -<sha> [comment]
            if in_refs {
                return None; // prerequisites must precede refs
            }
            let sha: String = rest.split_whitespace().next().unwrap_or("").to_string();
            if !is_hex(&sha, 40) && !is_hex(&sha, 64) {
                return None;
            }
            prerequisites.push(sha);
        } else if line.starts_with('@') {
            // v3 capability line (@object-format=sha256)
            if version < 3 {
                return None;
            }
        } else {
            // ref line: <sha> <ref>
            let mut parts = line.splitn(2, ' ');
            let sha = parts.next().unwrap_or("");
            let name = parts.next().unwrap_or("");
            if name.is_empty() || !is_hex(sha, 40) && !is_hex(sha, 64) {
                return None;
            }
            refs.push((sha.to_string(), name.to_string()));
            in_refs = true;
        }
    }
    Some(Bundle {
        version,
        prerequisites,
        refs,
        pack_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    fn fixture() -> Vec<u8> {
        format!("# v2 git bundle\n-{SHA} base commit\n{SHA} refs/heads/main\n\nPACKjunk")
            .into_bytes()
    }

    #[test]
    fn parses_header() {
        let b = parse(&fixture()).unwrap();
        assert_eq!(b.version, 2);
        assert_eq!(b.prerequisites, vec![SHA.to_string()]);
        assert_eq!(b.refs.len(), 1);
        // PACK sits right after the blank line
        assert_eq!(&fixture()[b.pack_offset..b.pack_offset + 4], b"PACK");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"# v9 git bundle\n").is_none());
        assert!(parse(b"not a bundle").is_none());
        // non-hex sha
        assert!(parse(b"# v2 git bundle\nzzzz refs/heads/x\n\nPACK").is_none());
    }
}
