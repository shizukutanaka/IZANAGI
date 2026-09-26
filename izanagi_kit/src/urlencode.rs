//! `application/x-www-form-urlencoded` decoding (WHATWG URL §5.1).
//!
//! `a=b&c=d` pairs: `+` decodes to space, `%XX` is a hex byte, a bare
//! `name` yields `(name, "")`. Invalid percent triplets and stray `%`
//! reject the whole input (form submissions are strict).
//!
//! ```
//! use izanagi_kit::urlencode;
//! let q = urlencode::parse(b"a=1&b=x+y&empty").unwrap();
//! assert_eq!(q[1], (b"b".to_vec(), b"x y".to_vec()));
//! ```

use std::vec::Vec;

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Decodes one component: `+`→space, `%XX`→byte.
pub fn decode(s: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        match s[i] {
            b'+' => out.push(b' '),
            b'%' => {
                let hi = *s.get(i + 1)?;
                let lo = *s.get(i + 2)?;
                out.push(hex(hi)? << 4 | hex(lo)?);
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    Some(out)
}

/// Splits `k=v&...` and decodes each side. `&`/`;` both separate.
pub fn parse(query: &[u8]) -> Option<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut out = Vec::new();
    for pair in query.split(|&b| b == b'&' || b == b';') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.iter().position(|&b| b == b'=') {
            Some(e) => (&pair[..e], &pair[e + 1..]),
            None => (pair, &[][..]),
        };
        out.push((decode(k)?, decode(v)?));
    }
    Some(out)
}

/// First decoded value for `key` (exact byte compare after decoding).
pub fn get<'a>(pairs: &'a [(Vec<u8>, Vec<u8>)], key: &[u8]) -> Option<&'a [u8]> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_plus_and_pct() {
        assert_eq!(decode(b"a+b%21"), Some(b"a b!".to_vec()));
        assert_eq!(decode(b"%e3%81%82"), Some(vec![0xe3, 0x81, 0x82]));
        assert_eq!(decode(b""), Some(vec![]));
    }

    #[test]
    fn rejects_bad_pct() {
        assert!(decode(b"%").is_none());
        assert!(decode(b"%2").is_none());
        assert!(decode(b"%zz").is_none());
        assert!(parse(b"a=%").is_none());
    }

    #[test]
    fn parse_pairs() {
        let q = parse(b"x=1;y=2&flag&z=").unwrap();
        assert_eq!(q.len(), 4);
        assert_eq!(get(&q, b"y"), Some(b"2".as_ref()));
        assert_eq!(get(&q, b"flag"), Some(b"".as_ref()));
        assert!(get(&q, b"missing").is_none());
    }
}
