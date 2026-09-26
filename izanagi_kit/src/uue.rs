//! uuencode / uudecode — the ASCII-armored binary transfer format from
//! BSD mail-era Unix (POSIX.1 `uuencode`).
//!
//! A uuencoded file is `begin <mode> <name>` (octal permission bits and a
//! filename), then data lines whose first character encodes the payload
//! length (`' ' + n`, with '`' / space meaning zero), groups of four
//! printable characters carrying six bits each, and a trailing `end`.
//!
//! ```
//! use izanagi_kit::uue::{parse, encode};
//! let d = encode(0o644, b"hi.bin", b"hello world");
//! let u = parse(&d).unwrap();
//! assert_eq!(u.mode, 0o644);
//! assert_eq!(u.name, b"hi.bin");
//! assert_eq!(u.data, b"hello world");
//! ```

/// Maximum payload bytes one uuencode line carries (the `M` line).
pub const PAYLOAD_MAX: usize = 45;

/// Decoded file from a `begin .. end` envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Uue {
    /// Octal permission bits from the `begin` line.
    pub mode: u32,
    /// Filename bytes after the mode field.
    pub name: Vec<u8>,
    /// Decoded payload.
    pub data: Vec<u8>,
}

/// Decode one data line into its raw bytes; `None` on malformed input.
/// The length character is `b[0]` (`' '` + n); the rest is `4*ceil(n/3)`
/// six-bit characters each written as `b - 0x20`.
pub fn decode_line(line: &[u8]) -> Option<Vec<u8>> {
    let (&head, body) = line.split_first()?;
    // The length char is `c - 0x20`; encoders may emit '`' (0x60) for 0.
    let n = if head == b'`' {
        0
    } else {
        head.wrapping_sub(0x20) as usize
    };
    if n > PAYLOAD_MAX || body.len() < n.div_ceil(3) * 4 {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    let mut at = 0;
    while out.len() < n {
        if at + 4 > body.len() {
            return None;
        }
        let mut v: u32 = 0;
        for &c in &body[at..at + 4] {
            v = (v << 6) | u32::from(c.wrapping_sub(0x20) & 0x3f);
        }
        out.push((v >> 16) as u8);
        if out.len() < n {
            out.push((v >> 8) as u8);
        }
        if out.len() < n {
            out.push(v as u8);
        }
        at += 4;
    }
    out.truncate(n);
    Some(out)
}

/// Encode `data` into one or more data lines (without `begin`/`end`).
pub fn encode_data(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in data.chunks(PAYLOAD_MAX) {
        out.push(0x20 + chunk.len() as u8);
        for trio in chunk.chunks(3) {
            let mut v: u32 = 0;
            for i in 0..3 {
                v = (v << 8) | u32::from(*trio.get(i).unwrap_or(&0));
            }
            for s in [18, 12, 6, 0] {
                let c = ((v >> s) & 0x3f) as u8;
                out.push(if c == 0 { b'`' } else { c + 0x20 });
            }
        }
        out.push(b'\n');
    }
    out.extend_from_slice(b"`\n");
    out
}

/// Wrap `encode_data` in a `begin <octal-mode> <name>` / `end` envelope.
pub fn encode(mode: u32, name: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("begin {:o} ", mode).as_bytes());
    out.extend_from_slice(name);
    out.push(b'\n');
    out.extend_from_slice(&encode_data(data));
    out.extend_from_slice(b"end\n");
    out
}

/// Parse a uuencoded file. `None` when the envelope or any data line is
/// malformed.
pub fn parse(d: &[u8]) -> Option<Uue> {
    let (mut at, end) = (0usize, d.len());
    let head = next_line(d, &mut at, end)?;
    let fields: Vec<&[u8]> = head
        .split(|&b| b == b' ')
        .filter(|f| !f.is_empty())
        .collect();
    if fields.len() < 3 || fields[0] != b"begin" {
        return None;
    }
    let mode = fields[1].iter().fold(0u32, |a, &c| {
        if (b'0'..=b'7').contains(&c) {
            a * 8 + u32::from(c - b'0')
        } else {
            a
        }
    });
    let name = fields[2..].join(&b' ');
    let mut data = Vec::new();
    loop {
        let line = next_line(d, &mut at, end)?;
        if line == b"end" {
            return Some(Uue { mode, name, data });
        }
        data.extend_from_slice(&decode_line(line)?);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_vector() {
        // 'Cat' encodes to #0V%T per the BSD man page.
        let d = b"begin 644 t\n#0V%T\n`\nend\n";
        let u = parse(d).unwrap();
        assert_eq!(u.mode, 0o644);
        assert_eq!(u.data, b"Cat");
    }

    #[test]
    fn roundtrip_and_line_codec() {
        let data: Vec<u8> = (0..=200).collect();
        let u = parse(&encode(0o755, b"big", &data)).unwrap();
        assert_eq!(u.mode, 0o755);
        assert_eq!(u.data, data);
        // encode_data without the envelope wraps at 45 bytes and ends
        // with a zero-length "`" line.
        let body = encode_data(b"Cat");
        assert_eq!(body, b"#0V%T\n`\n");
        let wide = encode_data(&data);
        let lines: Vec<&[u8]> = wide
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .collect();
        for line in &lines[..lines.len() - 1] {
            assert!(line.len() <= 1 + 4 * PAYLOAD_MAX.div_ceil(3));
        }
        assert_eq!(lines[lines.len() - 1], b"`");
        assert_eq!(decode_line(b"#0V%T").unwrap(), b"Cat");
        assert_eq!(decode_line(b"`").unwrap(), Vec::<u8>::new());
        assert!(decode_line(b"]").is_none()); // 0x5D-0x20 = 61 > 45
        assert!(decode_line(b"#0V").is_none()); // too short
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"begin 644 x\n").is_none()); // no end
        assert!(parse(b"start 644 x\nend\n").is_none());
        assert!(parse(&encode(0o644, b"n", b"abc")[..8]).is_none());
    }
}
