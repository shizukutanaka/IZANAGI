//! `/etc/group` group database: `name:passwd:gid:member1,member2,...` (group(5)).
//!
//! ```
//! use izanagi_kit::group::parse;
//!
//! let d = b"wheel:x:10:root,alice\ndaemon:x:1:\n";
//! let g = parse(d).unwrap();
//! assert_eq!(g.entries[0].members, vec!["root", "alice"]);
//! assert!(g.entries[1].members.is_empty());
//! ```

/// One `group` line.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Group name.
    pub name: String,
    /// Password placeholder.
    pub password: String,
    /// Numeric group id.
    pub gid: u32,
    /// Supplementary members (empty when the field is blank).
    pub members: Vec<String>,
}

/// Whole `/etc/group` file.
#[derive(Debug, Clone)]
pub struct Group {
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

/// Parse a `group` file: exactly 4 `:`-separated fields, numeric gid,
/// comma-separated member list.
pub fn parse(data: &[u8]) -> Option<Group> {
    let text = std::str::from_utf8(data).ok()?;
    let mut entries = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(':').collect();
        if f.len() != 4 {
            return None;
        }
        let members = if f[3].is_empty() {
            Vec::new()
        } else {
            f[3].split(',').map(|m| m.to_string()).collect()
        };
        entries.push(Entry {
            name: f[0].to_string(),
            password: f[1].to_string(),
            gid: num(f[2])?,
            members,
        });
    }
    Some(Group { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"root:x:0:\nsudo:x:27:a,b,c\n";
        let g = parse(d).unwrap();
        assert_eq!(g.entries[1].gid, 27);
        assert_eq!(g.entries[1].members.len(), 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"root:x:notnum:\n").is_none());
        assert!(parse(b"root:x:0\n").is_none());
    }
}
