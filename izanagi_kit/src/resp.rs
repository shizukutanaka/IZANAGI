//! RESP — the Redis serialization protocol (RESP2 + RESP3). Recursive
//! frame parser for `+` simple, `-` error, `:` integer, `$` bulk,
//! `*` array, `_` null, `#` bool, `,` double (raw text), `(` bignum,
//! `!` blob error, `=` verbatim, `%` map, `~` set, `|` attributes,
//! `>` push. Depth is capped at 256.
//!
//! ```
//! use izanagi_kit::resp::parse;
//!
//! let v = parse(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n").unwrap();
//! assert_eq!(v.len(), 1);
//! ```

use std::string::String;
use std::vec::Vec;

/// One RESP value.
#[derive(Clone, Debug, PartialEq)]
pub enum V {
    /// `+` simple string.
    Simple(String),
    /// `-` simple error.
    Err(String),
    /// `:` integer.
    Int(i64),
    /// `$` bulk string (`-1` → `Bulk(None)`), raw bytes.
    Bulk(Option<Vec<u8>>),
    /// `*` array (`-1` → `Array(None)`).
    Array(Option<Vec<V>>),
    /// RESP3 `_` null.
    Null,
    /// RESP3 `#t`/`#f`.
    Bool(bool),
    /// RESP3 `,` double — kept verbatim (`inf`/`nan` stay legal).
    Double(String),
    /// RESP3 `(` big number, verbatim.
    BigNum(String),
    /// RESP3 `!` blob error.
    BlobErr(Vec<u8>),
    /// RESP3 `=` verbatim string `(format, payload)`.
    Verbatim(String, String),
    /// RESP3 `%` map.
    Map(Vec<(V, V)>),
    /// RESP3 `~` set.
    Set(Vec<V>),
    /// RESP3 `|` attribute map.
    Attr(Vec<(V, V)>),
    /// RESP3 `>` push array.
    Push(Vec<V>),
}

struct P<'a> {
    d: &'a [u8],
    at: usize,
}

impl<'a> P<'a> {
    fn line(&mut self) -> Option<&'a str> {
        let start = self.at;
        while self.at < self.d.len() && self.d[self.at] != b'\r' {
            self.at += 1;
        }
        if self.at + 1 >= self.d.len() || self.d[self.at + 1] != b'\n' {
            return None;
        }
        let s = std::str::from_utf8(&self.d[start..self.at]).ok()?;
        self.at += 2;
        Some(s)
    }
    fn blob(&mut self) -> Option<Option<&'a [u8]>> {
        let n: i64 = self.line()?.trim().parse().ok()?;
        if n < 0 {
            return Some(None);
        }
        let n = n as usize;
        if self.at.checked_add(n + 2)? > self.d.len() {
            return None;
        }
        let s: &[u8] = &self.d[self.at..self.at + n];
        if self.d[self.at + n] != b'\r' || self.d[self.at + n + 1] != b'\n' {
            return None;
        }
        self.at += n + 2;
        Some(Some(s))
    }
    fn frame(&mut self, depth: usize) -> Option<V> {
        if depth > 256 {
            return None;
        }
        match *self.d.get(self.at)? {
            b'+' => {
                self.at += 1;
                Some(V::Simple(self.line()?.to_string()))
            }
            b'-' => {
                self.at += 1;
                Some(V::Err(self.line()?.to_string()))
            }
            b':' => {
                self.at += 1;
                Some(V::Int(self.line()?.trim().parse().ok()?))
            }
            b'_' => {
                self.at += 1;
                self.line()?;
                Some(V::Null)
            }
            b'#' => {
                self.at += 1;
                match self.line()? {
                    "t" => Some(V::Bool(true)),
                    "f" => Some(V::Bool(false)),
                    _ => None,
                }
            }
            b',' => {
                self.at += 1;
                Some(V::Double(self.line()?.to_string()))
            }
            b'(' => {
                self.at += 1;
                let s = self.line()?;
                if !s
                    .bytes()
                    .all(|c| c.is_ascii_digit() || c == b'-' || c == b'+')
                {
                    return None;
                }
                Some(V::BigNum(s.to_string()))
            }
            b'$' => {
                self.at += 1;
                Some(V::Bulk(self.blob()?.map(|b| b.to_vec())))
            }
            b'!' => {
                self.at += 1;
                Some(V::BlobErr(self.blob()??.to_vec()))
            }
            b'=' => {
                self.at += 1;
                let b = self.blob()??;
                if b.len() < 4 || b[3] != b':' {
                    return None;
                }
                Some(V::Verbatim(
                    String::from_utf8_lossy(&b[..3]).into_owned(),
                    String::from_utf8_lossy(&b[4..]).into_owned(),
                ))
            }
            k @ (b'*' | b'~' | b'>') => {
                self.at += 1;
                let n: i64 = self.line()?.trim().parse().ok()?;
                if n < 0 {
                    return Some(if k == b'*' {
                        V::Array(None)
                    } else if k == b'~' {
                        V::Set(Vec::new())
                    } else {
                        V::Push(Vec::new())
                    });
                }
                let mut items = Vec::new();
                for _ in 0..n {
                    items.push(self.frame(depth + 1)?);
                }
                Some(match k {
                    b'*' => V::Array(Some(items)),
                    b'~' => V::Set(items),
                    _ => V::Push(items),
                })
            }
            k @ (b'%' | b'|') => {
                self.at += 1;
                let n: i64 = self.line()?.trim().parse().ok()?;
                if n < 0 {
                    return None;
                }
                let mut items = Vec::new();
                for _ in 0..n {
                    let kk = self.frame(depth + 1)?;
                    let vv = self.frame(depth + 1)?;
                    items.push((kk, vv));
                }
                Some(if k == b'%' {
                    V::Map(items)
                } else {
                    V::Attr(items)
                })
            }
            _ => None,
        }
    }
}

/// Parse a stream of RESP frames (e.g. an `.aof` transcript).
pub fn parse(d: &[u8]) -> Option<Vec<V>> {
    let mut p = P { d, at: 0 };
    let mut out = Vec::new();
    while p.at < d.len() {
        out.push(p.frame(0)?);
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_and_replies() {
        let v = parse(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n+OK\r\n:42\r\n$-1\r\n").unwrap();
        assert_eq!(v.len(), 4);
        match &v[0] {
            V::Array(Some(a)) => {
                assert_eq!(a.len(), 2);
                assert_eq!(a[0], V::Bulk(Some(b"GET".to_vec())));
            }
            _ => panic!(),
        }
        assert_eq!(v[1], V::Simple("OK".to_string()));
        assert_eq!(v[2], V::Int(42));
        assert_eq!(v[3], V::Bulk(None));
    }

    #[test]
    fn resp3_kinds() {
        let v = parse(b"_\r\n#t\r\n,inf\r\n(123456789012345678901234567890\r\n%1\r\n+a\r\n:1\r\n~2\r\n:1\r\n:2\r\n>1\r\n+x\r\n").unwrap();
        assert_eq!(v[0], V::Null);
        assert_eq!(v[1], V::Bool(true));
        assert_eq!(v[2], V::Double("inf".to_string()));
        assert_eq!(
            v[3],
            V::BigNum("123456789012345678901234567890".to_string())
        );
        match &v[4] {
            V::Map(m) => assert_eq!(m.len(), 1),
            _ => panic!(),
        }
        match &v[5] {
            V::Set(s) => assert_eq!(s.len(), 2),
            _ => panic!(),
        }
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"?\r\n").is_none());
        assert!(parse(b"$3\r\nab\r\n").is_none()); // truncated bulk
        assert!(parse(b"*1\r\n").is_none()); // missing element
    }
}
