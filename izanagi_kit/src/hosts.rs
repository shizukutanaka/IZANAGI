//! `/etc/hosts` static host table: `addr canon [alias ...]` per line,
//! `#` comments (whole line or trailing) (hosts(5)).
//!
//! ```
//! use izanagi_kit::hosts::parse;
//!
//! let d = b"127.0.0.1 localhost\n::1 ip6-localhost ip6-loopback # v6\n";
//! let h = parse(d).unwrap();
//! assert_eq!(h.entries.len(), 2);
//! assert_eq!(h.entries[1].names, vec!["ip6-localhost", "ip6-loopback"]);
//! ```

/// One hosts entry: address plus all names on the line.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Address text (IPv4 dotted or IPv6) kept verbatim.
    pub address: String,
    /// Canonical name followed by aliases.
    pub names: Vec<String>,
}

/// Whole hosts file.
#[derive(Debug, Clone)]
pub struct Hosts {
    /// Entries in file order.
    pub entries: Vec<Entry>,
}

fn looks_like_addr(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_hexdigit() || b == b'.' || b == b':')
        && (s.contains('.') || s.contains(':'))
}

/// Parse a `hosts` file. Blank/comment lines are skipped; each entry needs at
/// least an address and one name.
pub fn parse(data: &[u8]) -> Option<Hosts> {
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
        if f.len() < 2 || !looks_like_addr(f[0]) {
            return None;
        }
        entries.push(Entry {
            address: f[0].to_string(),
            names: f[1..].iter().map(|s| s.to_string()).collect(),
        });
    }
    Some(Hosts { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"# header\n127.0.0.1 localhost local\n192.168.0.1 gw.lan gw\n";
        let h = parse(d).unwrap();
        assert_eq!(h.entries[0].names.len(), 2);
        assert_eq!(h.entries[1].address, "192.168.0.1");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"127.0.0.1\n").is_none()); // address without name
        assert!(parse(b"notanaddr host\n").is_none());
    }
}
