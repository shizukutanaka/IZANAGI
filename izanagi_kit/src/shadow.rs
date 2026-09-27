//! `/etc/shadow` aging database: `name:hash:lastchg:min:max:warn:inactive:expire[:reserved]`
//! (shadow(5)). Numeric fields may be empty (meaning "not set").
//!
//! ```
//! use izanagi_kit::shadow::parse;
//!
//! let d = b"root:$6$salt$hash:19700:0:99999:7:::\nuser:!:19000::::::\n";
//! let s = parse(d).unwrap();
//! assert_eq!(s.entries.len(), 2);
//! assert_eq!(s.entries[0].last_change, Some(19700));
//! assert_eq!(s.entries[1].min_days, None);
//! ```

/// One `shadow` line.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Login name.
    pub name: String,
    /// Password hash (`$id$salt$digest`, `!` locked, `*` no login).
    pub hash: String,
    /// Days since epoch of last password change.
    pub last_change: Option<u64>,
    /// Minimum days between changes.
    pub min_days: Option<u64>,
    /// Maximum days a password is valid.
    pub max_days: Option<u64>,
    /// Warning period in days.
    pub warn_days: Option<u64>,
    /// Days after expiry before the account is disabled.
    pub inactive_days: Option<u64>,
    /// Account expiry, days since epoch.
    pub expire: Option<u64>,
}

/// Whole `/etc/shadow` file.
#[derive(Debug, Clone)]
pub struct Shadow {
    /// Entries in file order.
    pub entries: Vec<Entry>,
}

fn num_or_empty(s: &str) -> Option<Option<u64>> {
    if s.is_empty() {
        return Some(None);
    }
    let mut v: u64 = 0;
    for &b in s.as_bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v.checked_mul(10)?.checked_add(u64::from(b - b'0'))?;
    }
    Some(Some(v))
}

/// Parse a `shadow` file: 8 or 9 colon-separated fields per non-blank line.
pub fn parse(data: &[u8]) -> Option<Shadow> {
    let text = std::str::from_utf8(data).ok()?;
    let mut entries = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(':').collect();
        if f.len() != 8 && f.len() != 9 {
            return None;
        }
        entries.push(Entry {
            name: f[0].to_string(),
            hash: f[1].to_string(),
            last_change: num_or_empty(f[2])?,
            min_days: num_or_empty(f[3])?,
            max_days: num_or_empty(f[4])?,
            warn_days: num_or_empty(f[5])?,
            inactive_days: num_or_empty(f[6])?,
            expire: num_or_empty(f[7])?,
        });
    }
    Some(Shadow { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"root:$6$xyz:19700:0:99999:7::0:\nbin:*:18000:0:99999:7:::reserved\n";
        let s = parse(d).unwrap();
        assert_eq!(s.entries[0].max_days, Some(99999));
        assert_eq!(s.entries[0].expire, Some(0));
        assert_eq!(s.entries[1].hash, "*");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"root:x:0\n").is_none());
        assert!(parse(b"root:x:notnum::::::\n").is_none());
    }
}
