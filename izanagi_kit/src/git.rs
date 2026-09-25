//! Git object model and wire framing — loose-object naming and parsing
//! (`"type N\0body"` content plus its `sha1` object name, optionally
//! zlib-wrapped as it sits in `.git/objects/`) and pkt-line framing
//! from the smart protocol (RFC-style `pkt-line`: 4 lowercase hex
//! digits of total length including themselves, `0000` flush).
//!
//! Determinism notes: object names are `sha1`, so `git hash-object`
//! output and this module agree bit-for-bit; `store`/`open_loose`
//! round-trip through [`crate::deflate`]/[`crate::inflate`].
//!
//! ```
//! use izanagi_kit::git;
//! assert_eq!(
//!     git::object_name("blob", b"hello\n"),
//!     "ce013625030ba8dba906f756967f9e9ca394464a"
//! );
//! ```

use crate::inflate;
use crate::sha1::sha1_hex;

/// `"<kind> <len>\0" + body` — the hashed, stored content.
pub fn store(kind: &str, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(kind.len() + body.len() + 24);
    out.extend_from_slice(kind.as_bytes());
    out.push(b' ');
    out.extend_from_slice(body.len().to_string().as_bytes());
    out.push(0);
    out.extend_from_slice(body);
    out
}

/// Lowercase hex object name of `"kind len\0body"`.
pub fn object_name(kind: &str, body: &[u8]) -> String {
    sha1_hex(&store(kind, body))
}

/// Split `"type N\0body"`; `None` on a missing NUL, a non-numeric size,
/// or a size that disagrees with the trailing body length.
pub fn parse_obj(data: &[u8]) -> Option<(String, Vec<u8>)> {
    let nul = data.iter().position(|&c| c == 0)?;
    let head = std::str::from_utf8(&data[..nul]).ok()?;
    let sp = head.find(' ')?;
    let kind = &head[..sp];
    if kind.is_empty() || !kind.bytes().all(|c| c.is_ascii_lowercase()) {
        return None;
    }
    let n: usize = head[sp + 1..].parse().ok()?;
    let body = &data[nul + 1..];
    if body.len() != n {
        return None;
    }
    Some((kind.to_string(), body.to_vec()))
}

/// Loose object file body (zlib over `"type N\0body"`) → (kind, body).
pub fn open_loose(z: &[u8]) -> Option<(String, Vec<u8>)> {
    parse_obj(&inflate::inflate_zlib(z)?)
}

/// zlib-compress `"type N\0body"` as written under `.git/objects/xx/`.
pub fn write_loose(kind: &str, body: &[u8]) -> Vec<u8> {
    crate::deflate::deflate_zlib(&store(kind, body))
}

/// One tree record: `"<mode> <name>\0" + 20-byte sha`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeEnt {
    /// Octal mode as a number (`0o100644`, `0o40000`, `0o160000`, …).
    pub mode: u32,
    /// Entry name (never contains '/' or NUL).
    pub name: String,
    /// Referenced object id.
    pub oid: [u8; 20],
}

/// Parse a tree object body; `None` on truncation or a non-octal mode.
pub fn parse_tree(body: &[u8]) -> Option<Vec<TreeEnt>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < body.len() {
        let sp = body[at..].iter().position(|&c| c == b' ')? + at;
        let m = std::str::from_utf8(&body[at..sp]).ok()?;
        if m.is_empty() || !m.bytes().all(|c| (b'0'..=b'7').contains(&c)) {
            return None;
        }
        let mode = u32::from_str_radix(m, 8).ok()?;
        let nul = body[sp + 1..].iter().position(|&c| c == 0)? + sp + 1;
        let name = std::str::from_utf8(&body[sp + 1..nul]).ok()?.to_string();
        let oid: [u8; 20] = body.get(nul + 1..nul + 21)?.try_into().ok()?;
        out.push(TreeEnt { mode, name, oid });
        at = nul + 21;
    }
    Some(out)
}

/// Canonical tree emit (entries in the order given — git sorts
/// directories with an implicit trailing '/', left to the caller).
pub fn emit_tree(ents: &[TreeEnt]) -> Vec<u8> {
    let mut out = Vec::new();
    for e in ents {
        out.extend_from_slice(format!("{:o}", e.mode).as_bytes());
        out.push(b' ');
        out.extend_from_slice(e.name.as_bytes());
        out.push(0);
        out.extend_from_slice(&e.oid);
    }
    out
}

fn hex_oid(s: &str) -> Option<[u8; 20]> {
    let b = s.as_bytes();
    if b.len() != 40 {
        return None;
    }
    let mut out = [0u8; 20];
    for i in 0..20 {
        let hi = (b[i * 2] as char).to_digit(16)? as u8;
        let lo = (b[i * 2 + 1] as char).to_digit(16)? as u8;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

/// Parsed commit object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    /// Root tree object id.
    pub tree: [u8; 20],
    /// Parent commit ids (empty for a root commit).
    pub parents: Vec<[u8; 20]>,
    /// `author` header value (`Name <mail> ts tz`), verbatim.
    pub author: String,
    /// `committer` header value, verbatim.
    pub committer: String,
    /// Message after the blank line.
    pub message: String,
}

/// Parse a commit object body; `None` without a `tree` header or the
/// header/message separator.
pub fn parse_commit(body: &[u8]) -> Option<Commit> {
    let text = std::str::from_utf8(body).ok()?;
    let sep = text.find("\n\n")?;
    let mut c = Commit {
        tree: [0; 20],
        parents: Vec::new(),
        author: String::new(),
        committer: String::new(),
        message: text[sep + 2..].to_string(),
    };
    let mut saw_tree = false;
    for line in text[..sep].lines() {
        if let Some(v) = line.strip_prefix("tree ") {
            c.tree = hex_oid(v)?;
            saw_tree = true;
        } else if let Some(v) = line.strip_prefix("parent ") {
            c.parents.push(hex_oid(v)?);
        } else if let Some(v) = line.strip_prefix("author ") {
            c.author = v.to_string();
        } else if let Some(v) = line.strip_prefix("committer ") {
            c.committer = v.to_string();
        }
    }
    if saw_tree {
        Some(c)
    } else {
        None
    }
}

/// pkt-line frame: 4 hex digits of total length + payload.
/// `None` (empty output) when `body` exceeds the 65520-byte cap.
pub fn pkt_line(body: &[u8]) -> Vec<u8> {
    let n = body.len() + 4;
    if n > 0xfff0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(n);
    out.extend_from_slice(format!("{:04x}", n).as_bytes());
    out.extend_from_slice(body);
    out
}

/// The `0000` flush packet.
pub fn pkt_flush() -> Vec<u8> {
    b"0000".to_vec()
}

/// Decode a pkt-line stream; `None` on a bad length field or a length
/// smaller than the 4-byte header. A `0000` flush ends the stream and
/// yields no element.
pub fn pkt_lines(d: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 4 <= d.len() {
        let h = std::str::from_utf8(&d[at..at + 4]).ok()?;
        if !h.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let n = usize::from_str_radix(h, 16).ok()?;
        if n == 0 {
            return Some(out);
        }
        if n < 4 || at + n > d.len() {
            return None;
        }
        out.push(d[at + 4..at + n].to_vec());
        at += n;
    }
    if at == d.len() {
        Some(out)
    } else {
        None
    }
}

/// `git ls-remote`-style ref advertisement line (`"<oid> <name>"`).
pub fn ref_line(oid: &[u8; 20], name: &str) -> String {
    let mut s = String::with_capacity(41 + name.len());
    for &v in oid {
        s.push(char::from_digit((v >> 4) as u32, 16).unwrap_or('0'));
        s.push(char::from_digit((v & 15) as u32, 16).unwrap_or('0'));
    }
    s.push(' ');
    s.push_str(name);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(s: &str) -> [u8; 20] {
        hex_oid(s).unwrap()
    }

    #[test]
    fn hash_object_matches_git() {
        // `echo hello | git hash-object --stdin`
        assert_eq!(
            object_name("blob", b"hello\n"),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
        assert_eq!(
            object_name("blob", b""),
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
        // `git commit-tree` of the empty tree
        let empty_tree: Vec<u8> = Vec::new();
        assert_eq!(
            object_name("tree", &empty_tree),
            "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
        );
    }

    #[test]
    fn store_and_parse_roundtrip() {
        let raw = store("blob", b"abc");
        assert_eq!(raw, b"blob 3\x00abc");
        let (k, b) = parse_obj(&raw).unwrap();
        assert_eq!((k.as_str(), b.as_slice()), ("blob", &b"abc"[..]));
        assert!(parse_obj(b"blob 4\x00abc").is_none()); // size mismatch
        assert!(parse_obj(b"blob x\x00abc").is_none());
        assert!(parse_obj(b"Blob 3\x00abc").is_none()); // kinds are lowercase
        assert!(parse_obj(b"blob3\x00abc").is_none());
        assert!(parse_obj(b"").is_none());
    }

    #[test]
    fn loose_zlib_roundtrip() {
        let z = write_loose("blob", b"some file contents\n");
        let (k, b) = open_loose(&z).unwrap();
        assert_eq!(k, "blob");
        assert_eq!(b, b"some file contents\n");
        assert!(open_loose(b"not zlib").is_none());
    }

    #[test]
    fn tree_binary_roundtrip() {
        let ents = vec![
            TreeEnt {
                mode: 0o100644,
                name: "a.txt".into(),
                oid: oid("ce013625030ba8dba906f756967f9e9ca394464a"),
            },
            TreeEnt {
                mode: 0o40000,
                name: "dir".into(),
                oid: oid("4b825dc642cb6eb9a060e54bf8d69288fbee4904"),
            },
        ];
        let body = emit_tree(&ents);
        assert_eq!(body[..7], b"100644 ".to_vec());
        let back = parse_tree(&body).unwrap();
        assert_eq!(back, ents);
        assert!(parse_tree(&body[..body.len() - 3]).is_none());
        assert!(parse_tree(b"888 name\x00123").is_none()); // non-octal mode
    }

    #[test]
    fn commit_headers() {
        let body = b"tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904\n\
                     parent ce013625030ba8dba906f756967f9e9ca394464a\n\
                     author A U Thor <a@x> 1700000000 +0900\n\
                     committer A U Thor <a@x> 1700000000 +0900\n\
                     \nsubject line\n\nbody\n";
        let c = parse_commit(body).unwrap();
        assert_eq!(c.tree, oid("4b825dc642cb6eb9a060e54bf8d69288fbee4904"));
        assert_eq!(c.parents.len(), 1);
        assert!(c.author.starts_with("A U Thor"));
        assert_eq!(c.message, "subject line\n\nbody\n");
        assert!(parse_commit(b"author x\n\nmsg").is_none()); // no tree
        assert!(parse_commit(b"tree zz\n\n").is_none()); // bad hex
    }

    #[test]
    fn pkt_line_roundtrip() {
        assert_eq!(pkt_line(b""), b"0004".to_vec());
        assert_eq!(pkt_line(b"ok"), b"0006ok".to_vec());
        assert_eq!(pkt_flush(), b"0000");
        let stream = [pkt_line(b"hello"), pkt_line(b""), pkt_flush()].concat();
        assert_eq!(
            pkt_lines(&stream).unwrap(),
            vec![b"hello".to_vec(), Vec::new()]
        );
        assert!(pkt_lines(b"0003x").is_none()); // len < 4
        assert!(pkt_lines(b"zzzz").is_none()); // non-hex
        assert!(pkt_lines(b"0009ab").is_none()); // truncated payload
        assert_eq!(pkt_line(&vec![0; 0x10000]), Vec::<u8>::new()); // over cap
    }

    #[test]
    fn ref_line_format() {
        let s = ref_line(
            &oid("ce013625030ba8dba906f756967f9e9ca394464a"),
            "refs/heads/main",
        );
        assert_eq!(
            s,
            "ce013625030ba8dba906f756967f9e9ca394464a refs/heads/main"
        );
    }
}
