//! cpio — the SVR4 "newc" archive format (`070701`): a fixed 110-byte
//! ASCII-hex header (ino/mode/uid/gid/nlink/mtime/filesize/devs/
//! namesize/check), a NUL-terminated name, then file data — both
//! padded to 4-byte alignment — terminated by a `TRAILER!!!` record.
//!
//! `tar`/`zip` cover the mainstream archivers; `cpio` covers the one
//! initramfs and RPM payloads actually use. Readers degrade: a bad hex
//! field or truncated record ends the listing rather than panicking.
//!
//! ```
//! use izanagi_kit::cpio;
//! let arc = cpio::emit(&[cpio::Entry::file("hello.txt", b"hi\n")]);
//! let es = cpio::parse(&arc);
//! assert_eq!(es[0].name, "hello.txt");
//! assert_eq!(es[0].data, b"hi\n");
//! ```

/// One archive member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// File name (`TRAILER!!!` is consumed, never produced).
    pub name: String,
    /// `st_mode` bits.
    pub mode: u32,
    /// Modification time.
    pub mtime: u64,
    /// Inode number.
    pub ino: u64,
    /// File body.
    pub data: Vec<u8>,
}

impl Entry {
    /// Convenience regular-file entry (`mode 0o100644`, mtime 0).
    pub fn file(name: &str, data: &[u8]) -> Self {
        Entry {
            name: name.to_string(),
            mode: 0o100644,
            mtime: 0,
            ino: 0,
            data: data.to_vec(),
        }
    }
}

fn hex8(s: &[u8]) -> Option<u64> {
    if s.len() != 8 {
        return None;
    }
    let mut v = 0u64;
    for &c in s {
        v = (v << 4) | (c as char).to_digit(16)? as u64;
    }
    Some(v)
}

/// Parse all members until `TRAILER!!!` (or EOF); malformed records
/// stop the scan and return what was collected — never panic.
pub fn parse(d: &[u8]) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut at = 0usize;
    loop {
        if at + 110 > d.len() {
            return out;
        }
        if &d[at..at + 6] != b"070701" && &d[at..at + 6] != b"070702" {
            return out;
        }
        let mut f = [0u64; 13];
        let mut ok = true;
        for i in 0..13 {
            match hex8(&d[at + 6 + i * 8..at + 14 + i * 8]) {
                Some(v) => f[i] = v,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            return out;
        }
        let (ino, mode, _uid, _gid, _nlink, mtime, filesize, namesize) =
            (f[0], f[1] as u32, f[2], f[3], f[4], f[5], f[6], f[11]);
        let name_at = at + 110;
        let name_end = name_at.checked_add(namesize as usize).unwrap_or(d.len());
        if name_end > d.len() {
            return out;
        }
        let name_raw = &d[name_at..name_end];
        let name = name_raw
            .split(|&c| c == 0)
            .next()
            .map(|v| String::from_utf8_lossy(v).into_owned())
            .unwrap_or_default();
        let data_at = (name_end + 3) & !3;
        if name == "TRAILER!!!" {
            return out;
        }
        let data_end = match data_at.checked_add(filesize as usize) {
            Some(v) if v <= d.len() => v,
            _ => return out,
        };
        out.push(Entry {
            name,
            mode,
            mtime,
            ino,
            data: d[data_at..data_end].to_vec(),
        });
        at = (data_end + 3) & !3;
    }
}

/// Find a member by exact name.
pub fn find<'a>(es: &'a [Entry], name: &str) -> Option<&'a Entry> {
    es.iter().find(|e| e.name == name)
}

/// Canonical emit: newc records for `es` plus the `TRAILER!!!` record,
/// dev fields and check zeroed.
pub fn emit(es: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut write_one = |name: &str, ino: u64, mode: u32, mtime: u64, data: &[u8]| {
        let fields = [
            ino,
            mode as u64,
            0,
            0,
            1,
            mtime,
            data.len() as u64,
            0,
            0,
            0,
            0,
            (name.len() + 1) as u64,
            0,
        ];
        let mut h = String::from("070701");
        for v in fields {
            h.push_str(&format!("{:08x}", v));
        }
        out.extend_from_slice(h.as_bytes());
        out.extend_from_slice(name.as_bytes());
        out.push(0);
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out.extend_from_slice(data);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    };
    for e in es {
        write_one(&e.name, e.ino, e.mode, e.mtime, &e.data);
    }
    write_one("TRAILER!!!", 0, 0, 0, &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_parse_roundtrip() {
        let es = vec![
            Entry::file("a.txt", b"alpha"),
            Entry::file("dir/b", b"b"),
            Entry {
                name: "x".into(),
                mode: 0o40755,
                mtime: 1700000000,
                ino: 42,
                data: Vec::new(),
            },
        ];
        let arc = emit(&es);
        assert_eq!(&arc[..6], b"070701");
        let back = parse(&arc);
        assert_eq!(back, es);
        assert_eq!(find(&back, "dir/b").unwrap().data, b"b");
        assert!(find(&back, "nope").is_none());
    }

    #[test]
    fn alignment() {
        // name lengths 1..8 exercise every pad width
        for n in 1..8usize {
            let name = "x".repeat(n);
            let arc = emit(&[Entry::file(&name, b"d")]);
            let back = parse(&arc);
            assert_eq!(back.len(), 1);
            assert_eq!(back[0].name, name);
            assert_eq!(back[0].data, b"d");
        }
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(b"").is_empty());
        assert!(parse(b"garbage").is_empty());
        let mut arc = emit(&[Entry::file("a", b"1")]);
        arc.truncate(arc.len() - 40); // clip trailer + tail
        let got = parse(&arc);
        assert_eq!(got.len(), 1); // first record still recovered
        let mut bad = emit(&[Entry::file("a", b"1")]);
        bad[20] = b'z'; // invalid hex digit in a field
        assert!(parse(&bad).is_empty());
        // truncated data → drop, not panic
        let mut t = emit(&[Entry::file("a", b"12345678")]);
        t.truncate(110 + 8);
        assert!(parse(&t).is_empty());
    }
}
