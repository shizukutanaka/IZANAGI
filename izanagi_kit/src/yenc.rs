//! yEnc binary encoding (Usenet `=ybegin`/`=yend` framing, draft by Jürgen
//! Helbing).
//!
//! Payload bytes are `(b + 42) & 0xFF`; critical bytes and line syntax
//! characters are escaped as `=` followed by `b + 64`. The trailer carries
//! `size=` and optionally `part=`/`crc32=`/`pcrc32=`; the CRC is the CRC-32
//! of the decoded bytes and is verified via [`crate::crc`].
//!
//! ```
//! use izanagi_kit::yenc::{encode, parse};
//! let d = encode(b"file.bin", b"hello");
//! let y = parse(&d).unwrap();
//! assert_eq!(y.name, b"file.bin");
//! assert_eq!(y.size, 5);
//! assert_eq!(y.data, b"hello");
//! assert!(y.crc_ok());
//! ```

/// Wrap column for encoded lines (the de facto 128).
pub const WRAP: usize = 128;

/// A decoded `=ybegin .. =yend` section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Yenc {
    /// `name=` from `=ybegin` (bytes after the last space).
    pub name: Vec<u8>,
    /// Declared `size=` from `=ybegin`.
    pub size: u32,
    /// `part=` number when present.
    pub part: Option<u32>,
    /// `crc32=` or `pcrc32=` declared in `=yend`, if any.
    pub crc32: Option<u32>,
    /// Decoded payload.
    pub data: Vec<u8>,
}

impl Yenc {
    /// `true` when the decoded length equals the declared `size=`.
    pub fn size_ok(&self) -> bool {
        self.data.len() as u32 == self.size
    }

    /// `true` when no CRC was declared or it matches the decoded bytes.
    pub fn crc_ok(&self) -> bool {
        match self.crc32 {
            None => true,
            Some(c) => crate::crc::crc32(&self.data) == c,
        }
    }
}

/// Encode `data` as a complete `=ybegin name=<name>`/`=yend` section.
/// Escapes NUL, TAB, LF, CR, space, `.` and `=`; wraps at [`WRAP`].
pub fn encode(name: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("=ybegin line={WRAP} size={} name=", data.len()).as_bytes());
    out.extend_from_slice(name);
    out.push(b'\n');
    let mut col = 0usize;
    let emit = |o: &mut Vec<u8>, c: u8, col: &mut usize| {
        o.push(c);
        *col += 1;
        if *col >= WRAP {
            o.push(b'\n');
            *col = 0;
        }
    };
    for &b in data {
        let c = b.wrapping_add(42);
        if c == 0 || c == b'=' || c == b'\t' || c == b'\n' || c == b'\r' || c == b' ' || c == b'.' {
            emit(&mut out, b'=', &mut col);
            emit(&mut out, c.wrapping_add(64), &mut col);
        } else {
            emit(&mut out, c, &mut col);
        }
    }
    if col > 0 {
        out.push(b'\n');
    }
    out.extend_from_slice(
        format!(
            "=yend size={} crc32={:08x}\n",
            data.len(),
            crate::crc::crc32(data)
        )
        .as_bytes(),
    );
    out
}

/// Parse one yEnc section. `None` on missing markers or malformed `=y`
/// keyword lines.
pub fn parse(d: &[u8]) -> Option<Yenc> {
    let (mut at, end) = (0usize, d.len());
    let head = next_line(d, &mut at, end)?;
    let kb = kv(head)?;
    let name = kb.iter().find(|(k, _)| *k == b"name")?.1.to_vec();
    let size = num(&kb, b"size")?;
    let part = num(&kb, b"part");
    let mut data = Vec::new();
    loop {
        let line = next_line(d, &mut at, end)?;
        if line.starts_with(b"=yend") {
            let ke = kv(line)?;
            let crc = ke
                .iter()
                .find(|(k, _)| *k == b"crc32" || *k == b"pcrc32")
                .and_then(|(_, v)| u32::from_str_radix(core::str::from_utf8(v).ok()?, 16).ok());
            return Some(Yenc {
                name,
                size,
                part,
                crc32: crc,
                data,
            });
        }
        if line.starts_with(b"=ypart") {
            continue;
        }
        decode_line(line, &mut data);
    }
}

/// Decode one payload line, un-escaping `=` pairs and subtracting 42.
fn decode_line(line: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < line.len() {
        let c = line[i];
        if c == b'=' && i + 1 < line.len() {
            out.push(line[i + 1].wrapping_sub(64).wrapping_sub(42));
            i += 2;
        } else {
            out.push(c.wrapping_sub(42));
            i += 1;
        }
    }
}

fn next_line<'a>(d: &'a [u8], at: &mut usize, end: usize) -> Option<&'a [u8]> {
    if *at >= end {
        return None;
    }
    let from = *at;
    let mut i = from;
    while i < end && d[i] != b'\n' {
        i += 1;
    }
    *at = i + 1;
    let mut line = &d[from..i];
    if line.last() == Some(&b'\r') {
        line = &line[..line.len() - 1];
    }
    Some(line)
}

fn kv(line: &[u8]) -> Option<Vec<(&[u8], &[u8])>> {
    if !line.starts_with(b"=y") {
        return None;
    }
    let mut v = Vec::new();
    for f in line.split(|&b| b == b' ').filter(|f| !f.is_empty()) {
        match f.iter().position(|&b| b == b'=') {
            Some(p) => v.push((&f[..p], &f[p + 1..])),
            None => v.push((f, &f[..0])),
        }
    }
    Some(v)
}

fn num(kb: &[(&[u8], &[u8])], k: &[u8]) -> Option<u32> {
    kb.iter()
        .find(|(kk, _)| *kk == k)
        .and_then(|(_, v)| core::str::from_utf8(v).ok()?.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_crc() {
        let mut d = Vec::new();
        for i in 0..512u32 {
            d.push((i % 256) as u8);
        }
        let y = parse(&encode(b"blob", &d)).unwrap();
        assert_eq!(y.name, b"blob");
        assert_eq!(y.size, 512);
        assert_eq!(y.data, d);
        assert!(y.size_ok() && y.crc_ok());
        assert_eq!(y.part, None);
    }

    #[test]
    fn escapes() {
        // '=' + 0x21 encodes byte 0x21-64-42 = -83 mod 256 = 173... craft via encoder instead.
        let y = parse(&encode(b"n", &[0u8, 9, 10, 13, 32, 61])).unwrap();
        assert_eq!(y.data, [0, 9, 10, 13, 32, 61]);
    }

    #[test]
    fn rejects_and_part() {
        assert!(parse(b"").is_none());
        assert!(parse(b"=ybegin name=x\n").is_none()); // no =yend
        let mut d = encode(b"x", b"ab");
        // splice a part= into =ybegin
        let s = String::from_utf8_lossy(&d).replace("name=", "part=1 name=");
        d = s.into_bytes();
        assert_eq!(parse(&d).unwrap().part, Some(1));
    }
}
