//! JWE compact serialization — five base64url segments
//! `header.encrypted_key.iv.ciphertext.tag`, the first a JSON object
//! with `alg`/`enc`/`kid`/`zip`/`crit`/`epk`/`cty` claims.
//!
//! ```
//! let d = b"eyJhbGciOiJSU0EtT0FFUCIsImVuYyI6IkEyNTZHQ00ifQ.aa.bb.cc.dd";
//! let j = izanagi_kit::jwe::parse(d).unwrap();
//! assert_eq!(j.segments, 5);
//! assert!(j.header_json);
//! assert_eq!(j.alg, Some(8));
//! assert_eq!(j.enc, Some(7));
//! assert!(izanagi_kit::jwe::detect(d));
//! ```

/// A JWE compact token census.
#[derive(Debug, Clone)]
pub struct Jwe {
    /// Segment count (5 = compact, 4 = empty encrypted key).
    pub segments: usize,
    /// Protected header decoded to a `{` JSON object.
    pub header_json: bool,
    /// `alg` claim value length.
    pub alg: Option<usize>,
    /// `enc` claim value length.
    pub enc: Option<usize>,
    /// `kid` claim present.
    pub kid: bool,
    /// `zip` claim present.
    pub zip: bool,
    /// `crit` claim present.
    pub crit: bool,
    /// `epk` (ephemeral public key) claim present.
    pub epk: bool,
    /// Base64url payload byte count (all segments summed).
    pub payload_bytes: usize,
    /// Named claims seen in the header.
    pub claims: usize,
}

fn b64val(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some((c - b'A') as u32),
        b'a'..=b'z' => Some((c - b'a' + 26) as u32),
        b'0'..=b'9' => Some((c - b'0' + 52) as u32),
        b'-' | b'+' => Some(62),
        b'_' | b'/' => Some(63),
        b'=' => Some(0),
        _ => None,
    }
}

/// Decodes base64url (padding optional) into a fresh `Vec<u8>`.
fn b64url(seg: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0u32;
    for c in seg.bytes() {
        let v = b64val(c)?;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

fn seg_ok(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'=')
}

fn json_str(t: &str, k: &str) -> Option<usize> {
    let pat = format!("\"{k}\":\"");
    let at = t.find(&pat)? + pat.len();
    t[at..].find('"')
}

/// Detects a JWE compact token: 4–5 nonempty base64url segments.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let segs: Vec<&str> = t.trim().split('.').collect();
    (4..=5).contains(&segs.len()) && segs.iter().all(|s| seg_ok(s))
}

/// Parses a JWE; `None` on bad segments or undecodable header.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Jwe> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let segs: Vec<&str> = t.trim().split('.').collect();
    let head = b64url(segs[0])?;
    let ht = std::str::from_utf8(&head).unwrap_or("");
    let header_json = ht.contains('{');
    const CLAIMS: &[&str] = &[
        "alg", "enc", "kid", "zip", "crit", "cty", "typ", "epk", "apu", "apv",
    ];
    Some(Jwe {
        segments: segs.len(),
        header_json,
        alg: json_str(ht, "alg"),
        enc: json_str(ht, "enc"),
        kid: ht.contains("\"kid\""),
        zip: ht.contains("\"zip\""),
        crit: ht.contains("\"crit\""),
        epk: ht.contains("\"epk\""),
        payload_bytes: segs.iter().map(|s| s.len()).sum(),
        claims: CLAIMS
            .iter()
            .map(|k| ht.matches(&format!("\"{k}\"")).count())
            .sum(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // header {"alg":"RSA-OAEP","enc":"A256GCM"}
    const D: &[u8] = b"eyJhbGciOiJSU0EtT0FFUCIsImVuYyI6IkEyNTZHQ00ifQ.aa.bb.cc.dd";

    #[test]
    fn parses() {
        let j = parse(D).unwrap();
        assert_eq!(j.segments, 5);
        assert!(j.header_json);
        assert_eq!(j.alg, Some(8));
        assert_eq!(j.enc, Some(7));
        assert!(j.payload_bytes > 40);
        assert!(j.claims >= 2);
    }

    #[test]
    fn decodes() {
        let d = b64url("aGVsbG8").unwrap();
        assert_eq!(&d, b"hello");
        assert!(b64url("!!!").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"a.b.c"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"too.many.parts.in.this.token").is_none());
        assert!(parse(b"notb64!.a.a.a.a").is_none());
    }
}
