//! `.htpasswd` / `htdigest` lines: `user:hash` (extra `:` fields kept
//! as `realm` for htdigest's `user:realm:hash`). Hash scheme is
//! sniffed: `$apr1$` MD5-crypt, `$2a/2b/2y$` bcrypt, `{SHA}` SHA-1,
//! `{CRYPT}`/`$1$`/`$5$`/`$6$` crypt, else plaintext/unknown.
//!
//! ```
//! use izanagi_kit::htpasswd::{entries, Scheme};
//!
//! let d = b"alice:$apr1$abc$xyz\nbob:$2y$10$deadbeef\ncarol:{SHA}q=\n";
//! let v: Vec<_> = entries(d).collect();
//! assert_eq!(v[0].scheme, Scheme::Apr1Md5);
//! assert_eq!(v[1].scheme, Scheme::Bcrypt);
//! assert_eq!(v[2].scheme, Scheme::Sha1);
//! ```

/// Password hash scheme, sniffed from the hash prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// `$apr1$` — Apache MD5 crypt.
    Apr1Md5,
    /// `$1$` — classic MD5 crypt.
    Md5Crypt,
    /// `$5$`/`$6$` — SHA-256/SHA-512 crypt.
    ShaCrypt,
    /// `$2a$`/`$2b$`/`$2x$`/`$2y$` — bcrypt.
    Bcrypt,
    /// `{SHA}` — base64 SHA-1.
    Sha1,
    /// `{CRYPT}` — DES-crypt output after the tag.
    Crypt,
    /// No scheme marker — plaintext or foreign.
    Plain,
}

/// One `user:hash[:realm]` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Username (never empty).
    pub user: String,
    /// Hash field verbatim (scheme prefix included).
    pub hash: String,
    /// Sniffed scheme.
    pub scheme: Scheme,
    /// Second `:`-separated field (htdigest realm) when present.
    pub realm: Option<String>,
}

use std::string::String;

/// Sniff the scheme from a hash string.
pub fn scheme_of(hash: &str) -> Scheme {
    if hash.starts_with("$apr1$") {
        Scheme::Apr1Md5
    } else if hash.starts_with("$2a$")
        || hash.starts_with("$2b$")
        || hash.starts_with("$2x$")
        || hash.starts_with("$2y$")
    {
        Scheme::Bcrypt
    } else if hash.starts_with("$5$") || hash.starts_with("$6$") {
        Scheme::ShaCrypt
    } else if hash.starts_with("$1$") {
        Scheme::Md5Crypt
    } else if hash.starts_with("{SHA}") {
        Scheme::Sha1
    } else if hash.starts_with("{CRYPT}") {
        Scheme::Crypt
    } else {
        Scheme::Plain
    }
}

/// Iterate entries; blank lines, `#` comments, and lines without `:` are skipped.
pub fn entries(d: &[u8]) -> impl Iterator<Item = Entry> + '_ {
    std::str::from_utf8(d)
        .unwrap_or("")
        .split('\n')
        .filter_map(one)
}

fn one(line: &str) -> Option<Entry> {
    let l = line.trim_end_matches(['\n', '\r']);
    if l.is_empty() || l.starts_with('#') {
        return None;
    }
    let (user, rest) = l.split_once(':')?;
    if user.is_empty() {
        return None;
    }
    let (hash, realm) = match rest.split_once(':') {
        Some((h, r)) => (h, Some(String::from(r))),
        None => (rest, None),
    };
    Some(Entry {
        user: user.into(),
        hash: hash.into(),
        scheme: scheme_of(hash),
        realm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] =
        b"# c\nalice:$apr1$s$e\nbob:$2y$10$bb\nold:{CRYPT}xx\nplain:secret\ndigest:realm:md5sum\n";

    #[test]
    fn fields() {
        let v: Vec<_> = entries(DOC).collect();
        assert_eq!(v.len(), 5);
        assert_eq!(v[0].user, "alice");
        assert_eq!(v[0].scheme, Scheme::Apr1Md5);
        assert_eq!(v[1].scheme, Scheme::Bcrypt);
        assert_eq!(v[2].scheme, Scheme::Crypt);
        assert_eq!(v[3].scheme, Scheme::Plain);
        assert_eq!(v[4].realm.as_deref(), Some("md5sum"));
        assert_eq!(v[4].hash, "realm"); // first ':' splits user, second splits realm
        assert_eq!(scheme_of("$6$salt$hash"), Scheme::ShaCrypt);
        assert_eq!(scheme_of("$1$x"), Scheme::Md5Crypt);
        assert!(entries(b"").next().is_none());
        assert!(entries(b"nopair\n").next().is_none());
    }
}
