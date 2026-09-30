//! iTerm2 inline-image escape — `ESC ] 1337 ; File = key=val ; … : base64`
//! terminated by `BEL` or `ESC \`. The `File` arguments are `;`-separated
//! `key=value` pairs and the payload follows a `:` separator.
//!
//! ```
//! let d = b"\x1b]1337;File=inline=1;width=2;name=aGk=:QUJD\x07";
//! let i = izanagi_kit::iterm::parse(d).unwrap();
//! assert!(i.inline);
//! assert!(i.width);
//! assert_eq!(i.payload_len, 4);
//! assert!(izanagi_kit::iterm::detect(d));
//! ```

/// Census of an iTerm2 OSC-1337 file transfer.
#[derive(Debug, Clone)]
pub struct Iterm {
    /// Byte offset of the `ESC ]` introducer.
    pub offset: usize,
    /// `key=value` pairs seen before the payload `:`.
    pub kv_pairs: usize,
    /// `inline=1` present.
    pub inline: bool,
    /// `width=` present.
    pub width: bool,
    /// `height=` present.
    pub height: bool,
    /// `name=` present.
    pub name: bool,
    /// Base64 payload bytes before the terminator.
    pub payload_len: usize,
    /// Whether `BEL` or `ESC \` terminated the sequence.
    pub terminated: bool,
}

const MARK: &[u8] = b"\x1b]1337;";

fn scan(b: &[u8]) -> Option<Iterm> {
    let start = b.windows(MARK.len()).position(|w| w == MARK)?;
    let mut i = start + MARK.len();
    // argument region: File=... until ':' payload separator or terminator
    let arg_end = {
        let mut j = i;
        while j < b.len() && b[j] != b':' && b[j] != 0x07 {
            if b[j] == 0x1b {
                break;
            }
            j += 1;
        }
        if j >= b.len() || b[j] != b':' {
            return None;
        }
        j
    };
    let args = std::str::from_utf8(&b[i..arg_end]).ok()?.to_string();
    let body = args
        .strip_prefix("File=")
        .or_else(|| args.strip_prefix("file="))?
        .to_string();
    let mut it = Iterm {
        offset: start,
        kv_pairs: 0,
        inline: false,
        width: false,
        height: false,
        name: false,
        payload_len: 0,
        terminated: false,
    };
    for part in body.split(';') {
        it.kv_pairs += 1;
        if part == "inline=1" {
            it.inline = true;
        }
        if part.starts_with("width=") {
            it.width = true;
        }
        if part.starts_with("height=") {
            it.height = true;
        }
        if part.starts_with("name=") {
            it.name = true;
        }
    }
    i = arg_end + 1;
    while i < b.len() {
        if b[i] == 0x07 || (b[i] == 0x1b && b.get(i + 1) == Some(&0x5c)) {
            it.terminated = true;
            break;
        }
        if b[i] == 0x1b {
            break;
        }
        it.payload_len += 1;
        i += 1;
    }
    Some(it)
}

/// Detects an iTerm2 OSC-1337 sequence with a `File=` argument block.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    scan(b).is_some()
}

/// Parses an iTerm2 sequence; `None` without the `1337` marker and `:`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Iterm> {
    scan(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"x\x1b]1337;File=inline=1;width=2;height=1;name=aGk=:QUJD\x07";

    #[test]
    fn parses() {
        let i = parse(D).unwrap();
        assert_eq!(i.offset, 1);
        assert_eq!(i.kv_pairs, 4);
        assert!(i.inline && i.width && i.height && i.name);
        assert_eq!(i.payload_len, 4);
        assert!(i.terminated);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"\x1b]0;title\x07"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"\x1b]1337;Other=x:y\x07").is_none()); // no File=
        assert!(parse(b"plain").is_none());
    }
}
