//! Unix `ar` archive — the `!<arch>\n` container behind `.a` static
//! libraries and `.deb` packages. Parses the 60-byte member headers,
//! GNU long-name tables (`//` string table, `/name#`, `/n` offsets)
//! and BSD `#1/n` inline names.
//!
//! All fields are ASCII-decimal/octal text in the header; malformed
//! input degrades to `None`.
//!
//! ```
//! use izanagi_kit::ar;
//! let mut a = b"!<arch>\n".to_vec();
//! a.extend_from_slice(b"foo.o/          "); // name, 16B, trailing '/'
//! a.extend_from_slice(b"0           ");     // mtime, 12B
//! a.extend_from_slice(b"0     ");           // uid, 6B
//! a.extend_from_slice(b"0     ");           // gid, 6B
//! a.extend_from_slice(b"100644  ");         // mode, 8B (octal)
//! a.extend_from_slice(b"3         ");       // size, 10B
//! a.extend_from_slice(b"`\n");              // terminator
//! a.extend_from_slice(b"obj");              // member data (pad if odd)
//! let arc = ar::parse(&a).unwrap();
//! assert_eq!(arc.entries.len(), 1);
//! assert_eq!(arc.entries[0].name(b""), "foo.o");
//! assert_eq!(ar::data(&a, &arc.entries[0]).unwrap(), b"obj");
//! ```

use std::string::String;
use std::vec::Vec;

fn trim_num(d: &[u8]) -> Option<u64> {
    let s = std::str::from_utf8(d).ok()?.trim();
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut v = 0u64;
    for b in s.bytes() {
        v = v.checked_mul(10)?.checked_add((b - b'0') as u64)?;
    }
    Some(v)
}

fn trim_octal(d: &[u8]) -> Option<u64> {
    let s = std::str::from_utf8(d).ok()?.trim();
    if s.is_empty() || !s.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
        return None;
    }
    let mut v = 0u64;
    for b in s.bytes() {
        v = v.checked_mul(8)?.checked_add((b - b'0') as u64)?;
    }
    Some(v)
}

/// One `ar` member header.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Raw 16-byte name field (as stored).
    pub raw_name: [u8; 16],
    /// File offset of the member data (after any BSD `#1/n` name
    /// bytes have been accounted for).
    pub offset: usize,
    /// Member data length (excluding the `#1` name prefix).
    pub size: usize,
    /// Modification time, seconds.
    pub mtime: u64,
    /// Owner id.
    pub uid: u64,
    /// Group id.
    pub gid: u64,
    /// Permission bits (octal field).
    pub mode: u64,
    /// For BSD `#1/n` names: the in-line name length (0 otherwise).
    pub inline_name_len: usize,
    /// For GNU `/n` names: offset into the `//` string table
    /// (`None` when not such a name).
    pub long_name_off: Option<usize>,
    /// Data also includes the name bytes (BSD `#1`).
    pub raw_size: usize,
}

impl Entry {
    /// Resolved member name: GNU `/n` string-table lookup
    /// (`strings` = the `//` member's data), or the raw field with a
    /// trailing `/`+spaces stripped. BSD `#1/n` names resolve via
    /// [`Entry::name_in`], which needs the archive bytes.
    pub fn name(&self, strings: &[u8]) -> String {
        if let Some(off) = self.long_name_off {
            if let Some(rest) = strings.get(off..) {
                let end = rest
                    .iter()
                    .position(|&b| b == b'\n' || b == b'/')
                    .unwrap_or(rest.len());
                return String::from_utf8_lossy(&rest[..end]).into_owned();
            }
        }
        let mut s = String::from_utf8_lossy(&self.raw_name).into_owned();
        while s.ends_with('/') || s.ends_with(' ') {
            s.pop();
        }
        s
    }

    /// Name resolved against the archive's own bytes: handles BSD
    /// `#1/n` names too (they live at the start of the data region).
    pub fn name_in(&self, whole: &[u8], strings: &[u8]) -> String {
        if self.inline_name_len > 0 {
            if let Some(at) = self.offset.checked_sub(self.inline_name_len) {
                if let Some(bytes) = whole.get(at..at + self.inline_name_len) {
                    return String::from_utf8_lossy(bytes).into_owned();
                }
            }
        }
        self.name(strings)
    }
}

/// A parsed `ar` archive: magic + member list.
#[derive(Clone, Debug)]
pub struct Ar {
    /// Members in file order.
    pub entries: Vec<Entry>,
    /// Contents of the GNU `//` long-name string table member,
    /// if present (empty otherwise).
    pub string_table: Vec<u8>,
}

/// Parse an `ar` archive.
pub fn parse(d: &[u8]) -> Option<Ar> {
    if d.get(0..8)? != b"!<arch>\n" {
        return None;
    }
    let mut i = 8usize;
    let mut entries = Vec::new();
    let mut string_table = Vec::new();
    while i < d.len() {
        let h = d.get(i..i + 60)?;
        if &h[58..60] != b"`\n" {
            return None;
        }
        let mtime = trim_num(&h[16..28])?;
        let uid = trim_num(&h[28..34])?;
        let gid = trim_num(&h[34..40])?;
        let mode = trim_octal(&h[40..48])?;
        let raw_size = trim_num(&h[48..58])? as usize;
        let mut raw_name = [0u8; 16];
        raw_name.copy_from_slice(&h[0..16]);
        let data_at = i + 60;
        let data_end = data_at.checked_add(raw_size)?;
        if data_end > d.len() {
            return None;
        }
        let mut offset = data_at;
        let mut size = raw_size;
        let mut inline_name_len = 0usize;
        let mut long_name_off = None;
        if raw_name.starts_with(b"#1/") {
            // BSD: name occupies the first n bytes of the data.
            let n = trim_num(&raw_name[3..])? as usize;
            if n > raw_size {
                return None;
            }
            inline_name_len = n;
            offset += n;
            size -= n;
        } else if raw_name.starts_with(b"/") && raw_name[1].is_ascii_digit() {
            // GNU: /decimal — offset into the `//` string table.
            let n = trim_num(&raw_name[1..])? as usize;
            long_name_off = Some(n);
        }
        if raw_name.starts_with(b"//") {
            string_table = d[data_at..data_end].to_vec();
            i = data_end + (raw_size & 1);
            continue; // the table itself is not a file member
        }
        entries.push(Entry {
            raw_name,
            offset,
            size,
            mtime,
            uid,
            gid,
            mode,
            inline_name_len,
            long_name_off,
            raw_size,
        });
        i = data_end + (raw_size & 1); // members are 2-byte aligned
    }
    Some(Ar {
        entries,
        string_table,
    })
}

/// Member data bytes.
pub fn data<'a>(d: &'a [u8], e: &Entry) -> Option<&'a [u8]> {
    d.get(e.offset..e.offset.checked_add(e.size)?)
}

/// First member with the resolved name `name`.
pub fn find<'a>(ar: &'a Ar, d: &[u8], name: &str) -> Option<&'a Entry> {
    ar.entries
        .iter()
        .find(|e| e.name_in(d, &ar.string_table) == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(name: &[u8; 16], size: usize) -> Vec<u8> {
        let mut h = Vec::with_capacity(60);
        h.extend_from_slice(name);
        h.extend_from_slice(b"0           ");
        h.extend_from_slice(b"0     ");
        h.extend_from_slice(b"0     ");
        h.extend_from_slice(b"100644  ");
        let mut s = Vec::with_capacity(10);
        let digits = size.to_string();
        s.extend_from_slice(digits.as_bytes());
        s.resize(10, b' ');
        h.extend_from_slice(&s);
        h.extend_from_slice(b"`\n");
        h
    }

    #[test]
    fn roundtrip_basic() {
        let mut a = b"!<arch>\n".to_vec();
        a.extend_from_slice(&hdr(b"foo.o/          ", 3));
        a.extend_from_slice(b"obj");
        a.push(b'\n'); // odd size → pad byte
        let arc = parse(&a).unwrap();
        assert_eq!(arc.entries.len(), 1);
        assert_eq!(arc.entries[0].name(&arc.string_table), "foo.o");
        assert_eq!(data(&a, &arc.entries[0]).unwrap(), b"obj");
        // second member follows the pad byte
        let mut b = a.clone();
        b.extend_from_slice(&hdr(b"bar.c/          ", 4));
        b.extend_from_slice(b"data");
        let arc2 = parse(&b).unwrap();
        assert_eq!(arc2.entries.len(), 2);
        assert_eq!(find(&arc2, &b, "bar.c").unwrap().size, 4);
    }

    #[test]
    fn gnu_long_names() {
        // `//` string table + `/0` member referring to it
        let table = b"averylongfilename.o/\n";
        let mut a = b"!<arch>\n".to_vec();
        a.extend_from_slice(&hdr(b"//              ", table.len()));
        a.extend_from_slice(table);
        if table.len() & 1 == 1 {
            a.push(b'\n');
        }
        a.extend_from_slice(&hdr(b"/0              ", 2));
        a.extend_from_slice(b"hi");
        let arc = parse(&a).unwrap();
        assert_eq!(arc.entries.len(), 1);
        assert_eq!(
            arc.entries[0].name_in(&a, &arc.string_table),
            "averylongfilename.o"
        );
        assert!(find(&arc, &a, "averylongfilename.o").is_some());
    }

    #[test]
    fn bsd_inline_names() {
        let name = b"longfile.o";
        let mut nm = Vec::new();
        nm.extend_from_slice(b"#1/10");
        nm.resize(16, b' ');
        let mut a = b"!<arch>\n".to_vec();
        a.extend_from_slice(&hdr(&nm[..16].try_into().unwrap(), 10 + 2));
        a.extend_from_slice(name);
        a.extend_from_slice(b"zz");
        let arc = parse(&a).unwrap();
        assert_eq!(arc.entries[0].inline_name_len, 10);
        assert_eq!(arc.entries[0].name_in(&a, &arc.string_table), "longfile.o");
        assert_eq!(data(&a, &arc.entries[0]).unwrap(), b"zz");
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"!<arch>").is_none()); // truncated magic
        assert!(parse(b"!<arch>!").is_none());
        let mut a = b"!<arch>\n".to_vec();
        a.extend_from_slice(&hdr(b"x/              ", 5));
        a.extend_from_slice(b"ab"); // declared 5, only 2 present
        assert!(parse(&a).is_none());
        let mut bad = b"!<arch>\n".to_vec();
        let mut h = hdr(b"x/              ", 0);
        h[58] = b'X';
        bad.extend_from_slice(&h);
        assert!(parse(&bad).is_none()); // bad terminator
    }

    #[test]
    fn determinism_is_structural() {
        let mut a = b"!<arch>\n".to_vec();
        a.extend_from_slice(&hdr(b"a.o/            ", 1));
        a.push(b'x');
        a.push(b'\n');
        let p1 = parse(&a).unwrap();
        let p2 = parse(&a).unwrap();
        assert_eq!(p1.entries.len(), p2.entries.len());
    }
}
