//! ZPL II (Zebra Programming Language) label scanning.
//!
//! Commands are `^XX...` (caret + two letters + parameters up to the next
//! `^`) or `~XX`. A label starts with `^XA` and ends with `^XZ`.
//!
//! ```
//! use izanagi_kit::zpl::{parse, Cmd};
//!
//! let r = parse(b"^XA^FO50,50^FDHello^FS^XZ").unwrap();
//! assert_eq!(r.cmds.len(), 5);
//! assert_eq!(r.cmds[0].code, Cmd::Xa);
//! assert_eq!(r.cmds[1].params, vec![b"50".to_vec(), b"50".to_vec()]);
//! ```

/// Recognized ZPL command codes (others map to `Other`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    /// `^XA` — label start.
    Xa,
    /// `^XZ` — label end.
    Xz,
    /// `^FO` — field origin.
    Fo,
    /// `^FD` — field data.
    Fd,
    /// `^FS` — field separator.
    Fs,
    /// `^CF` — change font.
    Cf,
    /// `^BC` — barcode.
    Bc,
    /// `^PQ` — print quantity.
    Pq,
    /// Any other `^XX`/`~XX`.
    Other([u8; 2]),
}

/// A decoded ZPL command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZplCmd {
    /// True when issued via `~` instead of `^`.
    pub tilde: bool,
    /// Command code.
    pub code: Cmd,
    /// Comma-split parameters (verbatim bytes).
    pub params: Vec<Vec<u8>>,
}

/// A full ZPL label (`^XA` ... `^XZ`).
#[derive(Debug)]
pub struct Zpl {
    /// Commands in order.
    pub cmds: Vec<ZplCmd>,
}

/// Parses one `^XA`...`^XZ` label.
pub fn parse(d: &[u8]) -> Option<Zpl> {
    let mut cmds = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        let b = d[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b != b'^' && b != b'~' {
            return None;
        }
        let tilde = b == b'~';
        if i + 2 >= d.len() {
            return None;
        }
        let c = [d[i + 1], d[i + 2]];
        if !c[0].is_ascii_alphanumeric() || !c[1].is_ascii_alphanumeric() {
            return None;
        }
        i += 3;
        // parameters until next '^' or '~' (or end)
        let start = i;
        while i < d.len() && d[i] != b'^' && d[i] != b'~' {
            i += 1;
        }
        let body = &d[start..i];
        let params: Vec<Vec<u8>> = if body.is_empty() {
            Vec::new()
        } else {
            body.split(|&x| x == b',').map(|s| s.to_vec()).collect()
        };
        cmds.push(ZplCmd {
            tilde,
            code: match &c {
                b"XA" => Cmd::Xa,
                b"XZ" => Cmd::Xz,
                b"FO" => Cmd::Fo,
                b"FD" => Cmd::Fd,
                b"FS" => Cmd::Fs,
                b"CF" => Cmd::Cf,
                b"BC" => Cmd::Bc,
                b"PQ" => Cmd::Pq,
                _ => Cmd::Other(c),
            },
            params,
        });
    }
    if cmds.first().map(|c| c.code) != Some(Cmd::Xa) || cmds.last().map(|c| c.code) != Some(Cmd::Xz)
    {
        return None;
    }
    Some(Zpl { cmds })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let r = parse(b"^XA^FO50,50^A0N,32,32^FDHello, World^FS^PQ2^XZ").unwrap();
        assert_eq!(r.cmds[0].code, Cmd::Xa);
        assert_eq!(r.cmds[1].code, Cmd::Fo);
        assert_eq!(r.cmds[1].params, vec![b"50".to_vec(), b"50".to_vec()]);
        assert_eq!(r.cmds[3].code, Cmd::Fd);
        assert_eq!(
            r.cmds[3].params,
            vec![b"Hello".to_vec(), b" World".to_vec()]
        );
        assert_eq!(r.cmds[5].code, Cmd::Pq);
        assert_eq!(r.cmds[6].code, Cmd::Xz);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"^FO1,1").is_none()); // no XA/XZ
        assert!(parse(b"^XA^XZ^X").is_none()); // truncated command
        assert!(parse(b"XZ").is_none());
    }
}
