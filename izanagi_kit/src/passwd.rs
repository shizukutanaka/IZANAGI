//! `/etc/passwd` user database: `name:passwd:uid:gid:gecos:dir:shell` (getpwent(3), passwd(5)).
//!
//! ```
//! use izanagi_kit::passwd::parse;
//!
//! let d = b"root:x:0:0:root:/root:/bin/sh\nnobody:x:65534:65534:nobody:/:/sbin/nologin\n";
//! let p = parse(d).unwrap();
//! assert_eq!(p.entries.len(), 2);
//! assert_eq!(p.entries[0].name, "root");
//! assert_eq!(p.entries[1].shell, "/sbin/nologin");
//! ```

/// One `passwd` line.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Login name.
    pub name: String,
    /// Encrypted password placeholder (usually `x`).
    pub password: String,
    /// Numeric user id.
    pub uid: u32,
    /// Primary group id.
    pub gid: u32,
    /// GECOS / comment field.
    pub gecos: String,
    /// Home directory.
    pub dir: String,
    /// Login shell.
    pub shell: String,
}

/// Whole `/etc/passwd` file.
#[derive(Debug, Clone)]
pub struct Passwd {
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

/// Parse a `passwd` file. Blank lines are skipped; every other line must have
/// exactly 7 `:`-separated fields with numeric uid/gid.
pub fn parse(data: &[u8]) -> Option<Passwd> {
    let text = std::str::from_utf8(data).ok()?;
    let mut entries = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(':').collect();
        if f.len() != 7 {
            return None;
        }
        entries.push(Entry {
            name: f[0].to_string(),
            password: f[1].to_string(),
            uid: num(f[2])?,
            gid: num(f[3])?,
            gecos: f[4].to_string(),
            dir: f[5].to_string(),
            shell: f[6].to_string(),
        });
    }
    Some(Passwd { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"root:x:0:0:root:/root:/bin/sh\ndaemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin\n";
        let p = parse(d).unwrap();
        assert_eq!(p.entries[1].gid, 1);
        assert_eq!(p.entries[0].dir, "/root");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"root:x:0\n").is_none()); // too few fields
        assert!(parse(b"root:x:notnum:0::/:/bin/sh\n").is_none());
        assert!(parse(&[0xff, 0xfe]).is_none()); // non-UTF8
    }

    #[test]
    fn empty_file_is_empty_not_error() {
        let p = parse(b"").unwrap();
        assert!(p.entries.is_empty());
    }
}
