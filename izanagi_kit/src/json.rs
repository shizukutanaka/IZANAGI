//! Strict JSON parser — the integer-only subset of RFC 8259.
//!
//! [`Json`] models null / bool / **integer** number / string /
//! array / object. Numbers must be integers that fit `i64`:
//! `.`, `e`, `E` exponents and overflow are rejected, matching
//! the crate's no-float contract. Strings are UTF-8 JSON text;
//! `\uXXXX` escapes decode through UTF-16 surrogate pairing and
//! lone surrogates are rejected.
//!
//! [`render`] writes the canonical form: object keys sorted
//! (the `BTreeMap` does it for free), minimal escapes, no
//! whitespace. `parse(render(x)) == x` always.
//!
//! ```
//! use izanagi_kit::json::{parse, render, Json};
//! let v = parse(br#"{"b":[1,2,3],"a":"x"}"#).unwrap();
//! assert_eq!(render(&v), r#"{"a":"x","b":[1,2,3]}"#);
//! assert!(parse(b"[1.5]").is_err()); // no floats
//! ```

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A JSON value — numbers are `i64` integers only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    /// `null`
    Null,
    /// `true` / `false`
    Bool(bool),
    /// Integer literal, `i64` range enforced.
    Int(i64),
    /// Decoded UTF-8 string content (no quotes).
    Str(String),
    /// Array items in order.
    Arr(Vec<Json>),
    /// Object members — `BTreeMap` keeps keys sorted, which is
    /// what makes [`render`] canonical.
    Obj(BTreeMap<String, Json>),
}

/// Byte-offset parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// Offset into the input where the failure occurred.
    pub pos: usize,
    /// One-line description.
    pub msg: &'static str,
}

/// Parse `s` (must be UTF-8 text) into a [`Json`] value.
/// Trailing whitespace is allowed; trailing garbage is an error.
pub fn parse(s: &[u8]) -> Result<Json, Error> {
    let text = match std::str::from_utf8(s) {
        Ok(t) => t,
        Err(_) => {
            return Err(Error {
                pos: 0,
                msg: "input is not UTF-8",
            })
        }
    };
    let mut p = P {
        b: text.as_bytes(),
        i: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(Error {
            pos: p.i,
            msg: "trailing characters after value",
        });
    }
    Ok(v)
}

/// Canonical rendering: sorted keys, minimal escapes, no
/// whitespace. `parse(render(v))` always round-trips.
pub fn render(v: &Json) -> String {
    let mut s = String::new();
    write_val(&mut s, v);
    s
}

fn write_val(s: &mut String, v: &Json) {
    match v {
        Json::Null => s.push_str("null"),
        Json::Bool(true) => s.push_str("true"),
        Json::Bool(false) => s.push_str("false"),
        Json::Int(n) => {
            let _ = write!(s, "{n}");
        }
        Json::Str(t) => write_str(s, t),
        Json::Arr(items) => {
            s.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                write_val(s, it);
            }
            s.push(']');
        }
        Json::Obj(m) => {
            s.push('{');
            for (i, (k, val)) in m.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                write_str(s, k);
                s.push(':');
                write_val(s, val);
            }
            s.push('}');
        }
    }
}

fn write_str(s: &mut String, t: &str) {
    s.push('"');
    for c in t.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(s, "\\u{:04x}", c as u32);
            }
            c => s.push(c),
        }
    }
    s.push('"');
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn err<T>(&self, msg: &'static str) -> Result<T, Error> {
        Err(Error { pos: self.i, msg })
    }

    fn value(&mut self) -> Result<Json, Error> {
        match self.peek() {
            Some(b'n') => self.lit(b"null", Json::Null),
            Some(b't') => self.lit(b"true", Json::Bool(true)),
            Some(b'f') => self.lit(b"false", Json::Bool(false)),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-') | Some(b'0'..=b'9') => self.number(),
            _ => self.err("expected a JSON value"),
        }
    }

    fn lit(&mut self, w: &[u8], v: Json) -> Result<Json, Error> {
        if self.b[self.i..].starts_with(w) {
            self.i += w.len();
            Ok(v)
        } else {
            self.err("bad literal")
        }
    }

    fn number(&mut self) -> Result<Json, Error> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        // integer part: 0 | [1-9][0-9]*
        match self.peek() {
            Some(b'0') => {
                self.i += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return self.err("leading zero in number");
                }
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.i += 1;
                }
            }
            _ => return self.err("expected digits"),
        }
        // the integer subset: fraction / exponent are rejected
        if matches!(self.peek(), Some(b'.') | Some(b'e') | Some(b'E')) {
            return self.err("non-integer number (floats are not supported)");
        }
        let text = std::str::from_utf8(&self.b[start..self.i]).unwrap_or("0");
        match text.parse::<i64>() {
            Ok(n) => Ok(Json::Int(n)),
            Err(_) => Err(Error {
                pos: start,
                msg: "integer out of i64 range",
            }),
        }
    }

    fn array(&mut self) -> Result<Json, Error> {
        self.i += 1; // '['
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            self.ws();
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b']') => {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                _ => return self.err("expected ',' or ']' in array"),
            }
        }
    }

    fn object(&mut self) -> Result<Json, Error> {
        self.i += 1; // '{'
        let mut m = BTreeMap::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Json::Obj(m));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return self.err("expected object key string");
            }
            let k = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return self.err("expected ':' after object key");
            }
            self.i += 1;
            self.ws();
            let v = self.value()?;
            m.insert(k, v);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Json::Obj(m));
                }
                _ => return self.err("expected ',' or '}' in object"),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, Error> {
        if self.i + 4 > self.b.len() {
            return self.err("truncated \\u escape");
        }
        let mut v = 0u32;
        for k in 0..4 {
            let c = self.b[self.i + k];
            let d = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => return self.err("bad hex in \\u escape"),
            };
            v = v * 16 + d as u32;
        }
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, Error> {
        self.i += 1; // '"'
        let mut out = String::new();
        loop {
            let c = match self.peek() {
                Some(c) => c,
                None => return self.err("unterminated string"),
            };
            match c {
                b'"' => {
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.i += 1;
                    match self.peek() {
                        Some(b'"') => {
                            out.push('"');
                            self.i += 1;
                        }
                        Some(b'\\') => {
                            out.push('\\');
                            self.i += 1;
                        }
                        Some(b'/') => {
                            out.push('/');
                            self.i += 1;
                        }
                        Some(b'b') => {
                            out.push('\u{8}');
                            self.i += 1;
                        }
                        Some(b'f') => {
                            out.push('\u{c}');
                            self.i += 1;
                        }
                        Some(b'n') => {
                            out.push('\n');
                            self.i += 1;
                        }
                        Some(b'r') => {
                            out.push('\r');
                            self.i += 1;
                        }
                        Some(b't') => {
                            out.push('\t');
                            self.i += 1;
                        }
                        Some(b'u') => {
                            self.i += 1;
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                // need a low surrogate
                                if self.peek() == Some(b'\\')
                                    && self.b.get(self.i + 1) == Some(&b'u')
                                {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if !(0xDC00..0xE000).contains(&lo) {
                                        return self.err("bad low surrogate in \\u pair");
                                    }
                                    0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                                } else {
                                    return self.err("lone high surrogate");
                                }
                            } else if (0xDC00..0xE000).contains(&hi) {
                                return self.err("lone low surrogate");
                            } else {
                                hi
                            };
                            match char::from_u32(cp) {
                                Some(ch) => out.push(ch),
                                None => return self.err("invalid code point"),
                            }
                        }
                        _ => return self.err("bad escape"),
                    }
                }
                0x00..=0x1F => return self.err("unescaped control character"),
                _ => {
                    // copy one UTF-8 char verbatim
                    let rest = std::str::from_utf8(&self.b[self.i..]).unwrap_or("");
                    let ch = rest.chars().next();
                    match ch {
                        Some(ch) => {
                            out.push(ch);
                            self.i += ch.len_utf8();
                        }
                        None => return self.err("bad UTF-8 in string"),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        assert_eq!(parse(b"null"), Ok(Json::Null));
        assert_eq!(parse(b"true"), Ok(Json::Bool(true)));
        assert_eq!(parse(b" false "), Ok(Json::Bool(false)));
        assert_eq!(parse(b"0"), Ok(Json::Int(0)));
        assert_eq!(parse(b"-17"), Ok(Json::Int(-17)));
        assert_eq!(parse(b"9223372036854775807"), Ok(Json::Int(i64::MAX)));
        assert_eq!(
            parse(br#""a\"b\\c\n""#),
            Ok(Json::Str("a\"b\\c\n".to_string()))
        );
        assert_eq!(
            parse(b"[1, 2, -3]"),
            Ok(Json::Arr(vec![Json::Int(1), Json::Int(2), Json::Int(-3)]))
        );
        let mut m = BTreeMap::new();
        m.insert("k".to_string(), Json::Int(4));
        m.insert("z".to_string(), Json::Bool(true));
        assert_eq!(parse(br#"{"z":true,"k":4}"#), Ok(Json::Obj(m)));
    }

    /// Everything malformed must be rejected — strict subset.
    #[test]
    fn rejects_invalid() {
        for bad in [
            &b""[..],
            b" ",
            b"[",
            b"[1,]",
            b"{\"a\"}",
            b"{\"a\":1,}",
            b"tru",
            b"nulll",
            b"01",
            b"-",
            b"+1",
            b"1.0",
            b"1e3",
            b"\"\\x\"",
            b"\"\\u12\"",
            b"\"\\ud800\"",         // lone high surrogate
            b"\"\\udc00\"",         // lone low surrogate
            b"\"abc",               // unterminated
            b"\"\x01\"",            // raw control char
            b"1 2",                 // trailing garbage
            b"9223372036854775808", // i64::MAX + 1
        ] {
            assert!(parse(bad).is_err(), "accepted {bad:?}");
        }
    }

    /// Canonical render: `parse(render(x)) == x` — plus the
    /// sorted-key property is verified directly.
    #[test]
    fn render_canonical_roundtrip() {
        let cases: &[&[u8]] = &[
            b"null",
            b"[3, -1, 0]",
            br#"{"b":1,"a":[true,null,"x"]}"#,
            br#"{"deep":{"deeper":[{"z":"\n"}]}}"#,
        ];
        for c in cases {
            let v = parse(c).unwrap();
            let r = render(&v);
            assert_eq!(parse(r.as_bytes()), Ok(v.clone()), "roundtrip {r}");
        }
        // canonical key order regardless of input order
        let v = parse(br#"{"zz":0,"aa":1,"mm":2}"#).unwrap();
        assert_eq!(render(&v), r#"{"aa":1,"mm":2,"zz":0}"#);
    }

    /// Escape handling incl. \uXXXX and surrogate pairs.
    #[test]
    fn escapes_and_unicode() {
        assert_eq!(parse(br#""\u0041""#), Ok(Json::Str("A".into())));
        assert_eq!(
            parse(br#""\ud83d\ude00""#),
            Ok(Json::Str("\u{1F600}".into()))
        );
        // render only escapes what it must
        let v = Json::Str("tab\there".into());
        assert_eq!(render(&v), r#""tab\there""#);
        let v = Json::Str("é".into()); // non-ASCII passes through raw
        assert_eq!(render(&v), "\"é\"");
    }

    /// Random JSON values generated by the rng — render → parse
    /// must reproduce them exactly.
    #[test]
    fn oracle_render_parse_roundtrip() {
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(0x3500);
        fn gen(rng: &mut SplitMix64, depth: usize) -> Json {
            match rng.below(if depth >= 3 { 3 } else { 6 }) {
                0 => Json::Null,
                1 => Json::Bool(rng.next_bool()),
                2 => Json::Int(rng.next_u64() as i64 % 1_000_000 - 500_000),
                3 => Json::Str(
                    (0..rng.below(8))
                        .map(|_| (b'a' + rng.below(26) as u8) as char)
                        .collect(),
                ),
                4 => Json::Arr((0..rng.below(4)).map(|_| gen(rng, depth + 1)).collect()),
                _ => Json::Obj(
                    (0..rng.below(4))
                        .map(|i| (format!("k{i}"), gen(rng, depth + 1)))
                        .collect(),
                ),
            }
        }
        for _ in 0..300 {
            let v = gen(&mut rng, 0);
            let r = render(&v);
            assert_eq!(parse(r.as_bytes()), Ok(v), "roundtrip failed: {r}");
        }
    }
}
