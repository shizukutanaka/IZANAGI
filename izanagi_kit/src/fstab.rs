//! `/etc/fstab` static mount table: `spec file vfstype mntops freq passno`
//! (fstab(5)). `#` starts a comment (whole line or trailing).
//!
//! ```
//! use izanagi_kit::fstab::parse;
//!
//! let d = b"# comment\n/dev/sda1 / ext4 defaults 0 1\nproc /proc proc defaults 0 0\n";
//! let f = parse(d).unwrap();
//! assert_eq!(f.entries.len(), 2);
//! assert_eq!(f.entries[0].vfstype, "ext4");
//! assert_eq!(f.entries[0].passno, 1);
//! ```

/// One `fstab` entry.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Device / remote spec (`/dev/...`, `UUID=...`, `host:/path`).
    pub spec: String,
    /// Mount point.
    pub file: String,
    /// Filesystem type (`ext4`, `swap`, `nfs`, ...).
    pub vfstype: String,
    /// Mount options, comma list kept verbatim.
    pub mntops: String,
    /// dump(8) frequency.
    pub freq: u32,
    /// fsck pass number.
    pub passno: u32,
}

/// Whole `/etc/fstab` file.
#[derive(Debug, Clone)]
pub struct Fstab {
    /// Entries in file order.
    pub entries: Vec<Entry>,
}

fn num(s: &str) -> Option<u32> {
    let mut v: u32 = 0;
    let mut any = false;
    for &b in s.as_bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        any = true;
        v = v.checked_mul(10)?.checked_add(u32::from(b - b'0'))?;
    }
    if any {
        Some(v)
    } else {
        None
    }
}

/// Parse an `fstab` file: each entry line has 6 whitespace-separated fields.
pub fn parse(data: &[u8]) -> Option<Fstab> {
    let text = std::str::from_utf8(data).ok()?;
    let mut entries = Vec::new();
    for line in text.lines() {
        let body = match line.find('#') {
            Some(i) => &line[..i],
            None => line,
        };
        let f: Vec<&str> = body.split_whitespace().collect();
        if f.is_empty() {
            continue;
        }
        if f.len() != 6 {
            return None;
        }
        entries.push(Entry {
            spec: f[0].to_string(),
            file: f[1].to_string(),
            vfstype: f[2].to_string(),
            mntops: f[3].to_string(),
            freq: num(f[4])?,
            passno: num(f[5])?,
        });
    }
    Some(Fstab { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"UUID=x /boot vfat defaults 0 2 # trailing\nswap none swap sw 0 0\n";
        let f = parse(d).unwrap();
        assert_eq!(f.entries.len(), 2);
        assert_eq!(f.entries[0].file, "/boot");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"/dev/sda1 / ext4\n").is_none());
        assert!(parse(b"a b c d x 0\n").is_none());
    }
}
