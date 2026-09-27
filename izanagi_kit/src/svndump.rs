//! Subversion `svnadmin dump` stream parsing.
//!
//! Text format: `SVN-fs-dump-format-version: N` header, `UUID:`
//! line, then blocks separated by blank lines — each block is
//! `Header: value` lines followed by `Content-length`-declared
//! payload bytes. `Revision-number` blocks contain `Node-path:`
//! node blocks with `Node-kind`/`Node-action`.
//!
//! ```
//! use izanagi_kit::svndump;
//! let d = b"SVN-fs-dump-format-version: 3\nUUID: abc\n\nRevision-number: 0\nProp-content-length: 10\nContent-length: 10\n\n0123456789\nNode-path: trunk\nNode-kind: dir\nNode-action: add\nContent-length: 0\n\n";
//! let s = svndump::parse(d).unwrap();
//! assert_eq!(s.format_version, 3);
//! assert_eq!(s.revisions, vec![0]);
//! ```

use std::string::String;
use std::vec::Vec;

/// A `Node-path` record inside a revision block.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// `Node-path` value.
    pub path: String,
    /// `Node-kind` (`file`/`dir`).
    pub kind: String,
    /// `Node-action` (`add`/`change`/`replace`/`delete`).
    pub action: String,
}

/// A parsed dump stream.
#[derive(Clone, Debug, PartialEq)]
pub struct SvnDump {
    /// `SVN-fs-dump-format-version` (2 or 3).
    pub format_version: u32,
    /// `UUID` header value.
    pub uuid: String,
    /// Revision numbers in order.
    pub revisions: Vec<u32>,
    /// Every node record, in file order.
    pub nodes: Vec<Node>,
}

fn headers<'a>(lines: &[&'a str]) -> Vec<(&'a str, &'a str)> {
    let mut out = Vec::new();
    for l in lines {
        match l.split_once(':') {
            Some((k, v)) => out.push((k.trim(), v.trim())),
            None => break,
        }
    }
    out
}

fn find<'a>(h: &[(&'a str, &'a str)], k: &str) -> Option<&'a str> {
    h.iter().find(|(key, _)| *key == k).map(|(_, v)| *v)
}

/// Parses a dump stream: format header + `Revision-number` blocks
/// each consuming their `Content-length` payload, node blocks listed
/// under `Node-path`/`Node-kind`/`Node-action`.
pub fn parse(d: &[u8]) -> Option<SvnDump> {
    let text = std::str::from_utf8(d).ok()?;
    let mut it = text.lines().peekable();
    let first = it.next()?;
    let version = first
        .strip_prefix("SVN-fs-dump-format-version:")?
        .trim()
        .parse::<u32>()
        .ok()?;
    if !(2..=3).contains(&version) {
        return None;
    }
    let mut uuid = String::new();
    let mut revisions = Vec::new();
    let mut nodes = Vec::new();
    // scan the rest block-wise: blank lines separate blocks; content
    // payloads consume raw bytes — approximate with line skip counting
    // via Content-length on header lines (payloads are binary so we
    // bound them by Content-length bytes on the raw stream).
    let mut at = first.len() + 1; // position in raw text
    let bytes = text.as_bytes();
    while at < bytes.len() {
        // read header lines until blank line
        let mut block: Vec<&str> = Vec::new();
        loop {
            if at >= bytes.len() {
                break;
            }
            let nl = bytes[at..].iter().position(|&b| b == b'\n');
            let (line, next) = match nl {
                Some(n) => {
                    let l = &bytes[at..at + n];
                    let l = if l.ends_with(b"\r") {
                        &l[..l.len() - 1]
                    } else {
                        l
                    };
                    (core::str::from_utf8(l).ok()?, at + n + 1)
                }
                None => (core::str::from_utf8(&bytes[at..]).ok()?, bytes.len()),
            };
            at = next;
            if line.is_empty() {
                break;
            }
            block.push(line);
        }
        if block.is_empty() {
            continue;
        }
        let h = headers(&block);
        if let Some(u) = find(&h, "UUID") {
            uuid = u.to_string();
        }
        if let Some(r) = find(&h, "Revision-number") {
            revisions.push(r.parse::<u32>().ok()?);
        }
        if let Some(path) = find(&h, "Node-path") {
            nodes.push(Node {
                path: path.to_string(),
                kind: find(&h, "Node-kind").unwrap_or("").to_string(),
                action: find(&h, "Node-action").unwrap_or("").to_string(),
            });
        }
        // skip the declared content payload
        if let Some(cl) = find(&h, "Content-length") {
            let n: usize = cl.parse().ok()?;
            at = at.checked_add(n)?;
            // a single newline follows the content
            if bytes.get(at) == Some(&b'\n') {
                at += 1;
            }
        }
    }
    Some(SvnDump {
        format_version: version,
        uuid,
        revisions,
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn fixture() -> Vec<u8> {
        b"SVN-fs-dump-format-version: 3\nUUID: repo-uuid\n\nRevision-number: 1\nProp-content-length: 5\nContent-length: 5\n\nK abc\nNode-path: trunk\nNode-kind: dir\nNode-action: add\nContent-length: 0\n\n".to_vec()
    }

    #[test]
    fn parses_dump() {
        let s = parse(&fixture()).unwrap();
        assert_eq!(s.format_version, 3);
        assert_eq!(s.uuid, "repo-uuid");
        assert_eq!(s.revisions, vec![1]);
        assert_eq!(s.nodes.len(), 1);
        assert_eq!(s.nodes[0].path, "trunk");
        assert_eq!(s.nodes[0].kind, "dir");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"random text").is_none());
        assert!(parse(b"SVN-fs-dump-format-version: 9\n").is_none());
    }
}
