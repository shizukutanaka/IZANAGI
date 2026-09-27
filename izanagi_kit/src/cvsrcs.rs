//! RCS `,v` (CVS/RCS master) file parsing.
//!
//! The admin section is `key   value;` lines (`head`, `branch`,
//! `access`, `symbols`, `locks`, `comment`, ...), followed by
//! revision blocks introduced by a bare `X.Y` revision-number line
//! (`date ...; author ...; state ...; ...` fields), then `desc`,
//! `@...@` log text and per-revision `log`/`text` blobs.
//!
//! ```
//! use izanagi_kit::cvsrcs;
//! let d = b"head\t1.2;\naccess;\nsymbols;\nlocks;\ncomment\t@# @;\n\n\n1.2\ndate\t2020.01.01.00.00.00;\tauthor\tme;\tstate\tExp;\nbranches;\nnext\t1.1;\n\n1.1\ndate\t2019.01.01.00.00.00;\tauthor\tme;\tstate\tExp;\nbranches;\nnext\t;\n\ndesc\n@@\n";
//! let r = cvsrcs::parse(d).unwrap();
//! assert_eq!(r.head, "1.2");
//! assert_eq!(r.revisions, vec!["1.2".to_string(), "1.1".to_string()]);
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// A parsed RCS file's admin section + revision index.
#[derive(Clone, Debug, PartialEq)]
pub struct Rcs {
    /// `head` revision (e.g. `1.2`).
    pub head: String,
    /// `branch` default (empty when absent).
    pub branch: String,
    /// Admin keys → values (`access`, `symbols`, `locks`, `comment`…).
    pub keys: BTreeMap<String, String>,
    /// Revision numbers in file order.
    pub revisions: Vec<String>,
}

fn is_revision_line(l: &str) -> bool {
    let l = l.trim();
    !l.is_empty()
        && l.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && l.bytes().any(|b| b == b'.')
        && !l.starts_with('.')
        && !l.ends_with('.')
}

fn dequote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('@') && v.ends_with('@') {
        v[1..v.len() - 1].replace("@@", "@")
    } else {
        v.to_string()
    }
}

/// Parses an RCS `,v` file: admin `key value;` lines until the first
/// bare revision number, then revision numbers until `desc`.
pub fn parse(d: &[u8]) -> Option<Rcs> {
    let text = std::str::from_utf8(d).ok()?;
    let mut head = String::new();
    let mut branch = String::new();
    let mut keys = BTreeMap::new();
    let mut revisions = Vec::new();
    let mut in_revs = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if !in_revs && is_revision_line(line) {
            in_revs = true;
        }
        if in_revs {
            if line == "desc" {
                break;
            }
            if is_revision_line(line) {
                revisions.push(line.to_string());
            }
            continue;
        }
        // admin line: `key value;` or `key;`
        let end = line.strip_suffix(';').unwrap_or(line);
        let (k, v) = match end.split_once(char::is_whitespace) {
            Some((k, v)) => (k, v),
            None => (end, ""),
        };
        let k = k.trim_end_matches(';').to_string();
        let v = dequote(v);
        if k == "head" {
            head = v.clone();
        } else if k == "branch" {
            branch = v.clone();
        }
        keys.insert(k, v);
    }
    if head.is_empty() || revisions.is_empty() {
        return None;
    }
    Some(Rcs {
        head,
        branch,
        keys,
        revisions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    const FIXTURE: &[u8] = b"head\t1.3;\naccess;\nsymbols\trel1:1.2;\nlocks; strict;\ncomment\t@# @;\n\n\n1.3\ndate\t2021.05.05.00.00.00;\tauthor\tjoe;\tstate\tExp;\nbranches;\nnext\t1.2;\n\n1.2\ndate\t2020.01.01.00.00.00;\tauthor\tjoe;\tstate\tExp;\nbranches;\nnext\t;\n\ndesc\n@file@\n";

    #[test]
    fn parses_rcs() {
        let r = parse(FIXTURE).unwrap();
        assert_eq!(r.head, "1.3");
        assert_eq!(r.revisions, vec!["1.3".to_string(), "1.2".to_string()]);
        assert_eq!(r.keys.get("symbols").map(|s| s.as_str()), Some("rel1:1.2"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"no rcs here").is_none());
        assert!(parse(b"head 1.1;\n").is_none()); // no revisions
    }
}
