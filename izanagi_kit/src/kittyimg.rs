//! Kitty graphics protocol — `ESC _ G control ; payload ESC \` where
//! `control` is comma-separated `key=value` data (`a` action, `f` format,
//! `t` medium, `m` more-chunks) and `payload` is base64 image bytes.
//!
//! ```
//! let d = b"\x1b_Ga=T,f=100,t=d,m=0;iVBOR\x1b\\";
//! let k = izanagi_kit::kittyimg::parse(d).unwrap();
//! assert_eq!(k.action, Some(b'T'));
//! assert_eq!(k.format, Some(b'1'));
//! assert_eq!(k.keys, 4);
//! assert!(izanagi_kit::kittyimg::detect(d));
//! ```

/// Census of a Kitty `ESC _ G` graphics command.
#[derive(Debug, Clone)]
pub struct Kittyimg {
    /// Byte offset of the `ESC _ G` introducer.
    pub offset: usize,
    /// Comma-separated control keys seen.
    pub keys: usize,
    /// `a=` action byte (e.g. `T` transmit+display, `q` query).
    pub action: Option<u8>,
    /// `f=` format byte.
    pub format: Option<u8>,
    /// `t=` transmission medium byte.
    pub medium: Option<u8>,
    /// `m=1` continuation flag present.
    pub more: bool,
    /// Payload bytes after the `;` separator.
    pub payload_len: usize,
    /// Whether `ESC \` (ST) terminated the command.
    pub terminated: bool,
}

const MARK: &[u8] = b"\x1b_G";

fn scan(b: &[u8]) -> Option<Kittyimg> {
    let start = b.windows(MARK.len()).position(|w| w == MARK)?;
    let mut i = start + MARK.len();
    let ctrl_end = {
        let mut j = i;
        while j < b.len() && b[j] != b';' && b[j] != 0x1b {
            j += 1;
        }
        j
    };
    if ctrl_end == i {
        return None; // empty control block
    }
    let ctrl = std::str::from_utf8(&b[i..ctrl_end]).ok()?.to_string();
    let mut k = Kittyimg {
        offset: start,
        keys: 0,
        action: None,
        format: None,
        medium: None,
        more: false,
        payload_len: 0,
        terminated: false,
    };
    for part in ctrl.split(',') {
        if part.is_empty() {
            continue;
        }
        k.keys += 1;
        if let Some(v) = part.strip_prefix("a=") {
            k.action = v.as_bytes().first().copied();
        } else if let Some(v) = part.strip_prefix("f=") {
            k.format = v.as_bytes().first().copied();
        } else if let Some(v) = part.strip_prefix("t=") {
            k.medium = v.as_bytes().first().copied();
        } else if part == "m=1" {
            k.more = true;
        }
    }
    if ctrl_end < b.len() && b[ctrl_end] == b';' {
        i = ctrl_end + 1;
        while i < b.len() {
            if b[i] == 0x1b {
                if b.get(i + 1) == Some(&0x5c) {
                    k.terminated = true;
                }
                break;
            }
            k.payload_len += 1;
            i += 1;
        }
    }
    (k.keys > 0).then_some(k)
}

/// Detects a Kitty graphics command: `ESC _ G` with a control block.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    scan(b).is_some()
}

/// Parses a Kitty graphics command; `None` without `ESC _ G` + keys.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Kittyimg> {
    scan(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"z\x1b_Ga=T,f=100,t=d,m=0;iVBORw0=\x1b\\";

    #[test]
    fn parses() {
        let k = parse(D).unwrap();
        assert_eq!(k.offset, 1);
        assert_eq!(k.keys, 4);
        assert_eq!(k.action, Some(b'T'));
        assert_eq!(k.format, Some(b'1'));
        assert_eq!(k.medium, Some(b'd'));
        assert!(!k.more);
        assert_eq!(k.payload_len, 8);
        assert!(k.terminated);
    }

    #[test]
    fn chunked() {
        let k = parse(b"\x1b_Ga=t,m=1;AAAA\x1b\\").unwrap();
        assert!(k.more);
        assert_eq!(k.keys, 2);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"\x1b_G\x1b\\"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
