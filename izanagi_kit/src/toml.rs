//! TOML v1.0 (Tom Preston-Werner / toml-lang) parser — the config
//! format: `key = value`, `[table]` sections, `[[array-of-tables]]`,
//! dotted keys, basic `"…"` strings with escapes, literal `'…'`
//! strings, multiline `"""…"""`/`'''…'''`, integers in base 10/16/8/2
//! with `_` separators, floats (parsed into [`Fixed`] — `inf`, `nan`
//! and magnitudes past Q16.16 range are rejected), booleans, arrays and
//! inline `{ a = 1 }` tables. Date-time values are preserved verbatim
//! as [`Val::Str`] (the kit is calendar-precise but this module only
//! needs the text).
//!
//! Parsing is total (`None` on any malformed input), depth is capped,
//! and [`encode`] emits a canonical, deterministically ordered
//! document that re-parses to the same tree. Re-declaring a `[table]`
//! merges into the existing section; a duplicate scalar key is
//! last-wins (tolerant, deterministic).
//!
//! ```
//! use izanagi_kit::toml::{parse, Val};
//!
//! let doc = parse("[server]\nhost = \"x\"\nports = [8_000, 8_001]\non = true\n").unwrap();
//! let server = doc.get("server").unwrap();
//! assert_eq!(server.get("ports").unwrap(), &Val::Arr(vec![
//!     Val::Int(8000), Val::Int(8001),
//! ]));
//! ```

use crate::fixed::Fixed;
use std::collections::BTreeMap;
use std::string::{String, ToString};
use std::vec::Vec;

/// One TOML value. Tables keep keys sorted for deterministic emit.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    /// 64-bit integer.
    Int(i64),
    /// Decimal literal held as Q16.16 (spec `inf`/`nan`/huge rejected).
    Float(Fixed),
    /// `true` / `false`.
    Bool(bool),
    /// Any string — including date-time text kept verbatim.
    Str(String),
    /// `[a, b, …]`.
    Arr(Vec<Val>),
    /// `{ k = v, … }` or `[section]` contents.
    Table(BTreeMap<String, Val>),
}

impl Val {
    /// Map lookup helper: `doc.get("a").get("b")`.
    pub fn get(&self, key: &str) -> Option<&Val> {
        match self {
            Val::Table(t) => t.get(key),
            _ => None,
        }
    }
    /// `Some(s)` for strings, `None` otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Val::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Top-level document.
pub type Table = BTreeMap<String, Val>;

const MAX_DEPTH: usize = 64;

/// Parse `src` into the document table; `None` on any syntax error.
pub fn parse(src: &str) -> Option<Table> {
    let mut p = P {
        s: src.as_bytes(),
        i: 0,
    };
    let mut root = Table::new();
    let mut path: Vec<String> = Vec::new(); // current [table] target
    loop {
        p.skip_ws_lines();
        if p.at_end() {
            return Some(root);
        }
        if p.peek() == b'[' {
            p.i += 1;
            let arr = p.eat(b'[');
            path = p.key_path()?;
            p.inline_ws();
            if arr && !p.eat(b']') {
                return None;
            }
            if !p.eat(b']') {
                return None;
            }
            ensure_header(&mut root, &path, arr)?;
            p.must_line_end()?;
        } else {
            let kp = p.key_path()?;
            if !p.eat_ws_eq() {
                return None;
            }
            let v = p.value(0)?;
            *leaf_slot(&mut root, &path, &kp)? = v;
            p.must_line_end()?;
        }
    }
}

/// `[a.b]` → Tables down the path; `[[a.b]]` → final key is an array
/// we append a fresh table to. Returns `None` on kind conflicts
/// (`[a]` then `[[a]]`, scalar shadowed by a table, …).
fn ensure_header(root: &mut Table, path: &[String], arr: bool) -> Option<()> {
    if path.is_empty() {
        return None;
    }
    let mut cur = root;
    for k in &path[..path.len() - 1] {
        let e = cur
            .entry(k.clone())
            .or_insert_with(|| Val::Table(Table::new()));
        cur = match e {
            Val::Table(t) => t,
            Val::Arr(a) => match a.last_mut() {
                Some(Val::Table(t)) => t,
                _ => return None,
            },
            _ => return None,
        };
    }
    let last = path[path.len() - 1].clone();
    if arr {
        let e = cur.entry(last).or_insert_with(|| Val::Arr(Vec::new()));
        match e {
            Val::Arr(a) => {
                a.push(Val::Table(Table::new()));
                Some(())
            }
            _ => None,
        }
    } else {
        let e = cur.entry(last).or_insert_with(|| Val::Table(Table::new()));
        match e {
            Val::Table(_) => Some(()),
            _ => None,
        }
    }
}

/// Descend `path` (arrays → their last element) then `key` (sub-tables
/// created as needed) and return the leaf slot for assignment.
fn leaf_slot<'t>(root: &'t mut Table, path: &[String], key: &[String]) -> Option<&'t mut Val> {
    let mut cur = root;
    for k in path.iter().chain(&key[..key.len() - 1]) {
        let e = cur
            .entry(k.clone())
            .or_insert_with(|| Val::Table(Table::new()));
        cur = match e {
            Val::Table(t) => t,
            Val::Arr(a) => match a.last_mut() {
                Some(Val::Table(t)) => t,
                _ => return None,
            },
            _ => return None,
        };
    }
    Some(cur.entry(key[key.len() - 1].clone()).or_insert(Val::Int(0)))
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn at_end(&self) -> bool {
        self.i >= self.s.len()
    }
    fn peek(&self) -> u8 {
        *self.s.get(self.i).unwrap_or(&0)
    }
    fn peek2(&self) -> u8 {
        *self.s.get(self.i + 1).unwrap_or(&0)
    }
    fn eat(&mut self, b: u8) -> bool {
        if self.peek() == b {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn inline_ws(&mut self) {
        while matches!(self.peek(), b' ' | b'\t') {
            self.i += 1;
        }
    }
    fn skip_ws_lines(&mut self) {
        loop {
            match self.peek() {
                b' ' | b'\t' | b'\r' | b'\n' => self.i += 1,
                b'#' => {
                    while !self.at_end() && self.peek() != b'\n' {
                        self.i += 1;
                    }
                }
                _ => return,
            }
        }
    }
    /// After a statement: ws → optional `#comment` → newline or EOF.
    fn must_line_end(&mut self) -> Option<()> {
        while matches!(self.peek(), b' ' | b'\t' | b'\r') {
            self.i += 1;
        }
        if self.eat(b'#') {
            while !self.at_end() && self.peek() != b'\n' {
                self.i += 1;
            }
        }
        if self.at_end() || self.eat(b'\n') {
            Some(())
        } else {
            None
        }
    }
    fn eat_ws_eq(&mut self) -> bool {
        self.inline_ws();
        self.eat(b'=')
    }
    /// `a.b."c"` — a dotted path of bare or quoted keys.
    fn key_path(&mut self) -> Option<Vec<String>> {
        let mut out = Vec::new();
        loop {
            self.inline_ws();
            let k = match self.peek() {
                b'"' => self.basic_string(false)?,
                b'\'' => self.lit_string()?,
                _ => {
                    let st = self.i;
                    while matches!(
                        self.peek(),
                        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'
                    ) {
                        self.i += 1;
                    }
                    if self.i == st {
                        return None;
                    }
                    String::from_utf8(self.s.get(st..self.i)?.to_vec()).ok()?
                }
            };
            out.push(k);
            self.inline_ws();
            if !self.eat(b'.') {
                return Some(out);
            }
        }
    }
    /// `"…"` basic string (single-line only).
    fn basic_string(&mut self, multi_ok: bool) -> Option<String> {
        if multi_ok && self.peek() == b'"' && self.peek2() == b'"' && self.peek3() == b'"' {
            self.i += 3;
            return self.str_triple(b'"');
        }
        self.i += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            match *self.s.get(self.i)? {
                b'"' => {
                    self.i += 1;
                    return String::from_utf8(out).ok();
                }
                b'\\' => {
                    self.i += 1;
                    self.escape_into(&mut out)?;
                }
                b'\n' => return None,
                c => {
                    out.push(c);
                    self.i += 1;
                }
            }
        }
    }
    fn lit_string(&mut self) -> Option<String> {
        self.i += 1;
        let st = self.i;
        loop {
            match *self.s.get(self.i)? {
                b'\'' => {
                    let v = String::from_utf8(self.s.get(st..self.i)?.to_vec()).ok()?;
                    self.i += 1;
                    return Some(v);
                }
                b'\n' => return None,
                _ => self.i += 1,
            }
        }
    }
    fn peek3(&self) -> u8 {
        *self.s.get(self.i + 2).unwrap_or(&0)
    }
    /// `"""…"""` or `'''…'''` (delimiter byte `d` × 3, already consumed).
    fn str_triple(&mut self, d: u8) -> Option<String> {
        if self.peek() == b'\n' {
            self.i += 1;
        } else if self.peek() == b'\r' && self.peek2() == b'\n' {
            self.i += 2;
        }
        let mut out: Vec<u8> = Vec::new();
        loop {
            if self.i + 2 < self.s.len()
                && self.s[self.i] == d
                && self.s[self.i + 1] == d
                && self.s[self.i + 2] == d
            {
                self.i += 3;
                return String::from_utf8(out).ok();
            }
            match *self.s.get(self.i)? {
                b'\\' if d == b'"' => {
                    self.i += 1;
                    if self.peek() == b'\n' || (self.peek() == b'\r' && self.peek2() == b'\n') {
                        if self.peek() == b'\r' {
                            self.i += 1;
                        }
                        self.i += 1;
                        while matches!(self.peek(), b' ' | b'\t' | b'\r' | b'\n') {
                            self.i += 1;
                        }
                    } else {
                        self.escape_into(&mut out)?;
                    }
                }
                c => {
                    out.push(c);
                    self.i += 1;
                }
            }
        }
    }
    fn escape_into(&mut self, out: &mut Vec<u8>) -> Option<()> {
        let c = *self.s.get(self.i)?;
        self.i += 1;
        match c {
            b'b' => out.push(0x08),
            b't' => out.push(b'\t'),
            b'n' => out.push(b'\n'),
            b'f' => out.push(0x0C),
            b'r' => out.push(b'\r'),
            b'"' => out.push(b'"'),
            b'\\' => out.push(b'\\'),
            b'u' | b'U' => {
                let n = if c == b'u' { 4 } else { 8 };
                let mut v: u32 = 0;
                for _ in 0..n {
                    let h = (*self.s.get(self.i)? as char).to_digit(16)?;
                    v = v.checked_mul(16)?.checked_add(h)?;
                    self.i += 1;
                }
                let ch = char::from_u32(v)?;
                let mut buf = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
            }
            _ => return None,
        }
        Some(())
    }
    fn value(&mut self, depth: usize) -> Option<Val> {
        if depth > MAX_DEPTH {
            return None;
        }
        self.inline_ws();
        match self.peek() {
            b'"' => Some(Val::Str(self.basic_string(true)?)),
            b'\'' => {
                if self.peek2() == b'\'' && self.peek3() == b'\'' {
                    self.i += 3;
                    Some(Val::Str(self.str_triple(b'\'')?))
                } else {
                    Some(Val::Str(self.lit_string()?))
                }
            }
            b'[' => {
                self.i += 1;
                let mut a = Vec::new();
                loop {
                    self.skip_ws_lines();
                    if self.eat(b']') {
                        return Some(Val::Arr(a));
                    }
                    a.push(self.value(depth + 1)?);
                    self.skip_ws_lines();
                    if self.eat(b',') {
                        continue;
                    }
                    if self.eat(b']') {
                        return Some(Val::Arr(a));
                    }
                    return None;
                }
            }
            b'{' => {
                self.i += 1;
                let mut t = Table::new();
                loop {
                    self.inline_ws();
                    if self.eat(b'}') {
                        return Some(Val::Table(t));
                    }
                    let kp = self.key_path()?;
                    if !self.eat_ws_eq() {
                        return None;
                    }
                    let v = self.value(depth + 1)?;
                    *leaf_slot(&mut t, &[], &kp)? = v;
                    self.inline_ws();
                    if self.eat(b',') {
                        continue;
                    }
                    if self.eat(b'}') {
                        return Some(Val::Table(t));
                    }
                    return None;
                }
            }
            _ => self.scalar(),
        }
    }
    /// Number / bool / date-time-ish bare token.
    fn scalar(&mut self) -> Option<Val> {
        let st = self.i;
        while !self.at_end()
            && !matches!(
                self.peek(),
                b'\n' | b'\r' | b',' | b']' | b'}' | b'#' | b' ' | b'\t'
            )
        {
            self.i += 1;
        }
        let tok = std::str::from_utf8(self.s.get(st..self.i)?).ok()?;
        match tok {
            "true" => return Some(Val::Bool(true)),
            "false" => return Some(Val::Bool(false)),
            _ => {}
        }
        if tok.is_empty() {
            return None;
        }
        let clean: String = tok.chars().filter(|&c| c != '_').collect();
        if let Some(hex) = clean.strip_prefix("0x") {
            return i64::from_str_radix(hex, 16).ok().map(Val::Int);
        }
        if let Some(o) = clean.strip_prefix("0o") {
            return i64::from_str_radix(o, 8).ok().map(Val::Int);
        }
        if let Some(b) = clean.strip_prefix("0b") {
            return i64::from_str_radix(b, 2).ok().map(Val::Int);
        }
        if clean.contains('.') || clean.contains('e') || clean.contains('E') {
            return parse_num(&clean).map(Val::Float);
        }
        if tok
            .chars()
            .all(|c| c.is_ascii_digit() || c == '+' || c == '-' || c == '_')
            && tok.chars().any(|c| c.is_ascii_digit())
        {
            return clean.parse::<i64>().ok().map(Val::Int);
        }
        // Date-time-ish or other digit-bearing bare token → verbatim.
        if tok
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b':' | b'.' | b'+'))
            && tok.bytes().any(|c| c.is_ascii_digit())
        {
            return Some(Val::Str(tok.to_string()));
        }
        None
    }
}

/// `[-+]?d+[.d+][eE[-+]d+]` → Fixed by pure decimal-digit arithmetic.
/// Magnitudes whose Q16.16 raw would exceed `i32` are rejected.
fn parse_num(tok: &str) -> Option<Fixed> {
    let b = tok.as_bytes();
    let mut i = 0;
    let neg = match b.first() {
        Some(b'-') => {
            i = 1;
            true
        }
        Some(b'+') => {
            i = 1;
            false
        }
        _ => false,
    };
    let mut ip: i64 = 0;
    let mut saw = false;
    while i < b.len() && b[i].is_ascii_digit() {
        ip = ip.checked_mul(10)?.checked_add((b[i] - b'0') as i64)?;
        if ip > i32::MAX as i64 {
            return None;
        }
        saw = true;
        i += 1;
    }
    let mut frac_num: i64 = 0;
    let mut frac_den: i64 = 1;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            frac_num = frac_num
                .checked_mul(10)?
                .checked_add((b[i] - b'0') as i64)?;
            frac_den = frac_den.checked_mul(10)?;
            if frac_den > 1_000_000_000_000 {
                return None;
            }
            saw = true;
            i += 1;
        }
    }
    if !saw {
        return None;
    }
    let mut exp: i64 = 0;
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        let eneg = match b.get(i) {
            Some(b'-') => {
                i += 1;
                true
            }
            Some(b'+') => {
                i += 1;
                false
            }
            _ => false,
        };
        let mut esaw = false;
        while i < b.len() && b[i].is_ascii_digit() {
            exp = exp.checked_mul(10)?.checked_add((b[i] - b'0') as i64)?;
            esaw = true;
            i += 1;
        }
        if !esaw || exp > 40 {
            return None;
        }
        if eneg {
            exp = -exp;
        }
    }
    if i != b.len() {
        return None;
    }
    let mut num: i128 = (ip as i128)
        .checked_mul(frac_den as i128)?
        .checked_add(frac_num as i128)?;
    let mut den: i128 = frac_den as i128;
    if exp >= 0 {
        for _ in 0..exp {
            num = num.checked_mul(10)?;
            if num > (i32::MAX as i128) * (1_i128 << 16) {
                return None;
            }
        }
    } else {
        for _ in 0..-exp {
            den = den.checked_mul(10)?;
            if den > i64::MAX as i128 {
                return None;
            }
        }
    }
    let raw = num.checked_mul(1_i128 << 16)? / den;
    if raw > i32::MAX as i128 || raw < i32::MIN as i128 {
        return None;
    }
    Some(Fixed::from_raw(if neg {
        -(raw as i32)
    } else {
        raw as i32
    }))
}

/// Canonical emission: scalars/arrays first, then sub-tables as
/// `[path]` sections and arrays-of-tables as `[[path]]`. Deterministic
/// (BTreeMap order) so `encode(parse(x))` is a fixed point.
pub fn encode(root: &Table) -> String {
    let mut out = String::new();
    emit_table(&mut out, root, &[]);
    out
}

fn emit_table(out: &mut String, t: &Table, path: &[String]) {
    for (k, v) in t {
        match v {
            Val::Table(_) => {}
            Val::Arr(a) if is_table_arr(a) => {}
            _ => {
                out.push_str(&emit_key(k));
                out.push_str(" = ");
                emit_val(out, v);
                out.push('\n');
            }
        }
    }
    for (k, v) in t {
        match v {
            Val::Table(st) => {
                let mut p = path.to_vec();
                p.push(k.clone());
                out.push('[');
                out.push_str(&p.iter().map(|x| emit_key(x)).collect::<Vec<_>>().join("."));
                out.push_str("]\n");
                emit_table(out, st, &p);
            }
            Val::Arr(a) if is_table_arr(a) => {
                let mut p = path.to_vec();
                p.push(k.clone());
                for item in a {
                    out.push_str("[[");
                    out.push_str(&p.iter().map(|x| emit_key(x)).collect::<Vec<_>>().join("."));
                    out.push_str("]]\n");
                    if let Val::Table(st) = item {
                        emit_table(out, st, &p);
                    }
                }
            }
            _ => {}
        }
    }
}

fn is_table_arr(a: &[Val]) -> bool {
    !a.is_empty() && a.iter().all(|x| matches!(x, Val::Table(_)))
}

fn emit_key(k: &str) -> String {
    if !k.is_empty()
        && k.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        k.to_string()
    } else {
        std::format!("\"{}\"", k.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn emit_val(out: &mut String, v: &Val) {
    match v {
        Val::Int(i) => out.push_str(&std::format!("{i}")),
        Val::Float(f) => out.push_str(&emit_fixed(*f)),
        Val::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Val::Str(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    '\r' => out.push_str("\\r"),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        Val::Arr(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_val(out, x);
            }
            out.push(']');
        }
        Val::Table(t) => {
            out.push('{');
            for (i, (k, x)) in t.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&emit_key(k));
                out.push_str(" = ");
                emit_val(out, x);
            }
            out.push('}');
        }
    }
}

/// Shortest exact decimal for a Fixed raw (same scheme as `wkt`).
fn emit_fixed(f: Fixed) -> String {
    let raw = f.raw();
    let neg = raw < 0;
    let mag = (raw as i64).unsigned_abs();
    let ip = mag >> 16;
    let mut fp = mag & 0xFFFF;
    let mut out = if neg { "-".to_string() } else { String::new() };
    out.push_str(&std::format!("{ip}"));
    if fp != 0 {
        out.push('.');
        let mut digs = String::new();
        for _ in 0..16 {
            fp *= 10;
            digs.push((b'0' + (fp >> 16) as u8) as char);
            fp &= 0xFFFF;
            if fp == 0 {
                break;
            }
        }
        out.push_str(&digs);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_document() {
        let d = parse("a = 1\nb = \"x\"\n[t]\nc = [1, 2.5, true]\n").unwrap();
        assert_eq!(d.get("a"), Some(&Val::Int(1)));
        assert_eq!(d.get("b"), Some(&Val::Str("x".into())));
        let t = d.get("t").unwrap().get("c").unwrap();
        assert_eq!(
            t,
            &Val::Arr(vec![
                Val::Int(1),
                Val::Float(Fixed::from_ratio(5, 2)),
                Val::Bool(true)
            ])
        );
    }

    #[test]
    fn dotted_keys_and_inline() {
        let d = parse("a.b.c = 7\np = { x = 1, y = \"z\" }\n").unwrap();
        assert_eq!(
            d.get("a").unwrap().get("b").unwrap().get("c"),
            Some(&Val::Int(7))
        );
        assert_eq!(d.get("p").unwrap().get("y"), Some(&Val::Str("z".into())));
    }

    #[test]
    fn strings_multiline_and_escapes() {
        let d =
            parse("s = \"\"\"abc\n    def\"\"\"\nl = 'raw\\\\x'\nu = \"\\u0041\\n\"\n").unwrap();
        assert_eq!(d.get("s").unwrap().as_str(), Some("abc\n    def"));
        assert_eq!(d.get("l").unwrap().as_str(), Some("raw\\\\x"));
        assert_eq!(d.get("u").unwrap().as_str(), Some("A\n"));
    }

    #[test]
    fn numbers_bases_floats() {
        let d = parse("h = 0xFF\no = 0o17\nb = 0b101\ni = 1_000\nf = -1.25\ne = 1e2\n").unwrap();
        assert_eq!(d.get("h"), Some(&Val::Int(255)));
        assert_eq!(d.get("o"), Some(&Val::Int(15)));
        assert_eq!(d.get("b"), Some(&Val::Int(5)));
        assert_eq!(d.get("i"), Some(&Val::Int(1000)));
        assert_eq!(d.get("f"), Some(&Val::Float(Fixed::from_ratio(-5, 4))));
        assert_eq!(d.get("e"), Some(&Val::Float(Fixed::from_int(100))));
    }

    #[test]
    fn array_of_tables_and_encode_roundtrip() {
        let src = "[[p]]\nx = 1\n[[p]]\nx = 2\n[q]\ny = true\n";
        let d = parse(src).unwrap();
        let p = d.get("p").unwrap();
        if let Val::Arr(a) = p {
            assert_eq!(a.len(), 2);
            assert_eq!(a[1].get("x"), Some(&Val::Int(2)));
        } else {
            panic!("expected array");
        }
        assert_eq!(d, parse(&encode(&d)).unwrap());
    }

    #[test]
    fn datetime_is_verbatim_string() {
        let d = parse("t = 1979-05-27T07:32:00Z\n").unwrap();
        assert_eq!(d.get("t").unwrap().as_str(), Some("1979-05-27T07:32:00Z"));
    }

    #[test]
    fn malformed_rejected() {
        for bad in [
            "a",
            "a = ",
            "= 1",
            "[x",
            "a = [1,",
            "a = {b",
            "a = \"\\q\"",
            "a = 0x",
            "a = 1e9999",
            "a = 40000.0",
            "a = 1 b = 2",
            "a = \"unclosed",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn determinism_twice() {
        let src = "[[p]]\nx = 1\n[[p]]\nx = 2.5\n[q.r]\ny = \"s\"\n";
        assert_eq!(parse(src), parse(src));
        assert_eq!(encode(&parse(src).unwrap()), encode(&parse(src).unwrap()));
    }
}
