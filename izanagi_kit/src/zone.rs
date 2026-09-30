//! DNS zone file format (RFC 1035 master file).
//!
//! Zone files list resource records in `name [ttl] [class] type rdata`
//! form, with `$ORIGIN`/`$TTL`/`$INCLUDE`/`$GENERATE` directives, `@`
//! origin shorthand, `(` … `)` continuations, and `;` comments.
//!
//! ```
//! let b = concat!(
//!     "$ORIGIN example.com.\n",
//!     "$TTL 3600\n",
//!     "@ IN SOA ns1 hostmaster ( 1 7200 600 86400 60 )\n",
//!     "  IN NS ns1\n",
//!     "  IN NS ns2\n",
//!     "www IN A 192 0 2 1\n",
//!     "mail IN MX 10 mail\n"
//! ).as_bytes();
//! assert!(izanagi_kit::zone::detect(b));
//! let c = izanagi_kit::zone::Zone::parse(b).unwrap();
//! assert_eq!(c.directives, 2);
//! assert!(c.records > 0);
//! ```

/// Parsed zone file summary.
#[derive(Debug, Clone)]
pub struct Zone {
    /// `$ORIGIN`/`$TTL`/`$INCLUDE`/`$GENERATE`/`$LOAD`/`$TTL`-style directives.
    pub directives: usize,
    /// Resource-record lines (non-directive, non-comment).
    pub records: usize,
    /// SOA records.
    pub soa: usize,
    /// NS records.
    pub ns: usize,
    /// A records.
    pub a: usize,
    /// AAAA records.
    pub aaaa: usize,
    /// MX records.
    pub mx: usize,
    /// CNAME records.
    pub cname: usize,
    /// TXT records.
    pub txt: usize,
    /// PTR records.
    pub ptr: usize,
    /// SRV records.
    pub srv: usize,
    /// CAA records.
    pub caa: usize,
    /// DNSSEC types (DS, DNSKEY, RRSIG, NSEC, NSEC3, TLSA, SSHFP, DNAME).
    pub dnssec: usize,
    /// `;` comment lines.
    pub comments: usize,
}

const RRS: &[(&str, u8)] = &[
    ("SOA", b's'),
    ("NS", b'n'),
    ("AAAA", b'a'),
    ("A", b'A'),
    ("MX", b'm'),
    ("CNAME", b'c'),
    ("TXT", b't'),
    ("PTR", b'p'),
    ("SRV", b'v'),
    ("CAA", b'C'),
    ("DS", b'd'),
    ("DNSKEY", b'd'),
    ("RRSIG", b'd'),
    ("NSEC", b'd'),
    ("NSEC3", b'd'),
    ("NSEC3PARAM", b'd'),
    ("TLSA", b'd'),
    ("SSHFP", b'd'),
    ("DNAME", b'd'),
];

fn rr_of(tokens: &[&str]) -> Option<u8> {
    // The owner name is never the type; scan forward so the type token
    // wins over rdata words that collide with type names (e.g. RRSIG's
    // covered-type field).
    for tok in tokens.iter().skip(1) {
        for (name, tag) in RRS {
            if tok.eq_ignore_ascii_case(name) {
                return Some(*tag);
            }
        }
    }
    None
}

/// Whether the buffer looks like a DNS zone file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut score = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with(';') {
            continue;
        }
        if tr.starts_with('$') {
            score += 2;
            continue;
        }
        // an RR line has at least 2 tokens and a known type somewhere
        let toks: Vec<&str> = tr.split_whitespace().collect();
        if toks.len() >= 2 && rr_of(&toks).is_some() {
            score += 1;
        }
    }
    score >= 3
}

impl Zone {
    /// Parses a zone file summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            directives: 0,
            records: 0,
            soa: 0,
            ns: 0,
            a: 0,
            aaaa: 0,
            mx: 0,
            cname: 0,
            txt: 0,
            ptr: 0,
            srv: 0,
            caa: 0,
            dnssec: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('$') {
                c.directives += 1;
                continue;
            }
            let toks: Vec<&str> = tr.split_whitespace().collect();
            if let Some(tag) = rr_of(&toks) {
                c.records += 1;
                match tag {
                    b's' => c.soa += 1,
                    b'n' => c.ns += 1,
                    b'a' => c.aaaa += 1,
                    b'A' => c.a += 1,
                    b'm' => c.mx += 1,
                    b'c' => c.cname += 1,
                    b't' => c.txt += 1,
                    b'p' => c.ptr += 1,
                    b'v' => c.srv += 1,
                    b'C' => c.caa += 1,
                    _ => c.dnssec += 1,
                }
            } else {
                c.records += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_zone() {
        let b = concat!(
            "$ORIGIN example.com.\n",
            "$TTL 3600\n",
            "@ IN SOA ns1 hostmaster ( 1 7200 600 86400 60 )\n",
            "  IN NS ns1\n",
            "  IN NS ns2\n",
            "www IN A 192 0 2 1\n",
            "mail IN MX 10 mail\n",
            "txt IN TXT \"v=spf1 -all\"\n",
            "; comment\n",
            "ipv6 IN AAAA 2001 db8 1\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Zone::parse(b).unwrap();
        assert_eq!(c.directives, 2);
        assert_eq!(c.soa, 1);
        assert_eq!(c.ns, 2);
        assert_eq!(c.a, 1);
        assert_eq!(c.mx, 1);
        assert_eq!(c.txt, 1);
        assert_eq!(c.aaaa, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn parses_dnssec() {
        let b = concat!(
            "$ORIGIN ex.com.\n",
            "@ IN SOA ns1 hostmaster 1 7200 600 86400 60\n",
            "@ IN NS ns1\n",
            "@ IN DS 12345 8 2 ABCDEF\n",
            "@ IN DNSKEY 257 3 8 AwEAA\n",
            "@ IN RRSIG A 8 2 3600 2024 12345 ex.com. AA==\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Zone::parse(b).unwrap();
        assert_eq!(c.dnssec, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"just some text\nwith words\n"));
        assert!(Zone::parse(b"x").is_none());
    }
}
