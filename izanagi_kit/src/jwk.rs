//! JSON Web Key `.jwk`/JWKS — `{"kty":"RSA|EC|OKP|oct"}` with
//! `n`/`e`/`x`/`y`/`k`/`crv`/`d`/`use`/`kid`/`alg` members, or a
//! `{"keys":[…]}` set.
//!
//! ```
//! let d = b"{\"kty\":\"RSA\",\"n\":\"abc\",\"e\":\"AQAB\",\"kid\":\"k1\"}";
//! let j = izanagi_kit::jwk::parse(d).unwrap();
//! assert_eq!(j.kty, Some(3));
//! assert_eq!(j.kids, 1);
//! assert!(!j.set);
//! assert!(izanagi_kit::jwk::detect(d));
//! ```

/// Census of a JWK or JWKS document.
#[derive(Debug, Clone)]
pub struct Jwk {
    /// `{"keys":[…]}` set form.
    pub set: bool,
    /// `kty` value length (RSA/EC/OKP/oct).
    pub kty: Option<usize>,
    /// `"n"` modulus members.
    pub moduli: usize,
    /// `"e"` exponent members.
    pub exponents: usize,
    /// `"x"`/`"y"` coordinate members.
    pub coords: usize,
    /// `"k"` symmetric key members.
    pub keys_sym: usize,
    /// `"crv"` curve members.
    pub curves: usize,
    /// `"d"` private material members.
    pub private: usize,
    /// `"use"` members.
    pub uses: usize,
    /// `"key_ops"` members.
    pub key_ops: usize,
    /// `"kid"` members.
    pub kids: usize,
    /// `"alg"` members.
    pub algs: usize,
    /// `"x5c"`/`"x5t"` certificate members.
    pub x5: usize,
}

fn count_key(t: &str, key: &str) -> usize {
    let needle = format!("\"{key}\"");
    t.matches(&needle).count()
}

/// Detects a JWK: `{` + `"kty"` or `"keys"`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains('{') && (t.contains("\"kty\"") || t.contains("\"keys\""))
}

/// Parses a JWK/JWKS; `None` on non-UTF-8 or missing markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Jwk> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let kty = count_key(t, "kty");
    Some(Jwk {
        set: t.contains("\"keys\""),
        kty: (kty > 0).then_some(3),
        moduli: count_key(t, "n"),
        exponents: count_key(t, "e"),
        coords: count_key(t, "x") + count_key(t, "y"),
        keys_sym: count_key(t, "k"),
        curves: count_key(t, "crv"),
        private: count_key(t, "d"),
        uses: count_key(t, "use"),
        key_ops: count_key(t, "key_ops"),
        kids: count_key(t, "kid"),
        algs: count_key(t, "alg"),
        x5: count_key(t, "x5c") + count_key(t, "x5t"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"{\"kty\":\"RSA\",\"n\":\"abc\",\"e\":\"AQAB\",\"use\":\"sig\",\"kid\":\"k1\",\"alg\":\"RS256\",\"x5c\":[\"c\"],\"d\":\"p\"}";

    #[test]
    fn parses_single() {
        let j = parse(D).unwrap();
        assert!(!j.set);
        assert_eq!(j.kty, Some(3));
        assert_eq!(j.moduli, 1);
        assert_eq!(j.exponents, 1);
        assert_eq!(j.private, 1);
        assert_eq!(j.uses, 1);
        assert_eq!(j.kids, 1);
        assert_eq!(j.algs, 1);
        assert_eq!(j.x5, 1);
    }

    #[test]
    fn parses_set() {
        let j = parse(b"{\"keys\":[{\"kty\":\"oct\",\"k\":\"x\"},{\"kty\":\"EC\",\"crv\":\"P-256\",\"x\":\"1\",\"y\":\"2\"}]}").unwrap();
        assert!(j.set);
        assert_eq!(j.keys_sym, 1);
        assert_eq!(j.curves, 1);
        assert_eq!(j.coords, 3); // key "x" + key "y" + value "x"
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"{\"keys\":[]}"));
        assert!(!detect(b"{\"a\":1}"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"[]").is_none());
    }
}
