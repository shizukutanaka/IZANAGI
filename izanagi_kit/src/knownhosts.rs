//! SSH `known_hosts` lines: `[marker] hosts key-type base64-key
//! [comment]` — comma-separated hosts, `|1|…` hashed names,
//! `@cert-authority`/`@revoked` markers, `#` comments.
//!
//! ```
//! use izanagi_kit::knownhosts::lines;
//!
//! let d = b"# comment\n@cert-authority *.example.com ssh-ed25519 AAAAC3 comment\n[host]:2222 ssh-rsa AAAA\n";
//! let v: Vec<_> = lines(d).collect();
//! assert_eq!(v.len(), 2);
//! assert_eq!(v[0].marker, Some(izanagi_kit::knownhosts::Marker::CertAuthority));
//! assert_eq!(v[0].key_type, "ssh-ed25519");
//! assert_eq!(v[1].hosts, "[host]:2222");
//! ```

/// Marker kinds that may prefix a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// `@cert-authority` — CA key for host certs.
    CertAuthority,
    /// `@revoked` — revoked key.
    Revoked,
    /// Any other `@…` marker (kept generically).
    Other,
}

/// One `known_hosts` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    /// Line marker when present.
    pub marker: Option<Marker>,
    /// Hosts field verbatim (comma list; `|1|…` hashed form kept raw).
    pub hosts: String,
    /// Key algorithm (`ssh-rsa`, `ssh-ed25519`, `ecdsa-sha2-nistp256`, …).
    pub key_type: String,
    /// Base64 public-key blob.
    pub key: String,
    /// Trailing comment (empty when absent).
    pub comment: String,
}

/// Iterate entries; blank lines, `#` comments, and malformed lines are skipped.
pub fn lines(d: &[u8]) -> impl Iterator<Item = Host> + '_ {
    std::str::from_utf8(d)
        .unwrap_or("")
        .split('\n')
        .filter_map(entry)
}

fn entry(line: &str) -> Option<Host> {
    let l = line.trim();
    if l.is_empty() || l.starts_with('#') {
        return None;
    }
    let mut parts = l.split_whitespace();
    let mut first = parts.next()?;
    let marker = if first.starts_with('@') {
        let m = match first {
            "@cert-authority" => Marker::CertAuthority,
            "@revoked" => Marker::Revoked,
            _ => Marker::Other,
        };
        first = parts.next()?;
        Some(m)
    } else {
        None
    };
    let key_type = parts.next()?;
    let key = parts.next()?;
    let comment = parts.collect::<Vec<_>>().join(" ");
    Some(Host {
        marker,
        hosts: first.into(),
        key_type: key_type.into(),
        key: key.into(),
        comment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"# header\n@cert-authority *.ex.com ssh-ed25519 AAA ok\n|1|salt|hash ssh-rsa BBBB # note\n@revoked host ssh-rsa CCCC\nbad line\n\n";

    #[test]
    fn fields() {
        let v: Vec<_> = lines(DOC).collect();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].marker, Some(Marker::CertAuthority));
        assert_eq!(v[0].hosts, "*.ex.com");
        assert_eq!(v[0].comment, "ok");
        assert!(v[1].hosts.starts_with('|'));
        assert_eq!(v[1].comment, "# note");
        assert_eq!(v[2].marker, Some(Marker::Revoked));
        assert!(v[2].comment.is_empty());
        assert!(lines(b"").next().is_none());
    }
}
