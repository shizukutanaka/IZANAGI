//! ANSI art / `.ans` escape-sequence census — scans for `ESC [` CSI
//! sequences (SGR colour, cursor motion, screen clears), `ESC ]` OSC
//! strings, and high-byte CP437 art characters.
//!
//! ```
//! let d = b"\x1b[1;31mRED\x1b[0m\n\x1b[2J\x1b[1;1HOK";
//! let a = izanagi_kit::ansi::parse(d).unwrap();
//! assert_eq!(a.csi, 4);
//! assert_eq!(a.sgr, 2);
//! assert_eq!(a.cursor_moves, 1);
//! assert_eq!(a.clears, 1);
//! assert!(izanagi_kit::ansi::detect(d));
//! ```

/// Escape-sequence census of an ANSI/`.ans` buffer.
#[derive(Debug, Clone)]
pub struct Ansi {
    /// CSI sequences (`ESC [ params final`).
    pub csi: usize,
    /// CSI finals `m` (SGR — colour/attribute).
    pub sgr: usize,
    /// CSI finals moving the cursor (`H A B C D E F G s u`).
    pub cursor_moves: usize,
    /// CSI finals `J`/`K` (erase display/line).
    pub clears: usize,
    /// OSC sequences (`ESC ] … BEL|ST`).
    pub osc: usize,
    /// Other `ESC` sequences not CSI/OSC.
    pub other_esc: usize,
    /// Bytes `>= 0x80` (CP437 block art).
    pub high_bytes: usize,
    /// `\n` line count.
    pub lines: usize,
}

fn csi_kind(finalb: u8) -> usize {
    match finalb {
        b'm' => 0,        // SGR
        b'J' | b'K' => 1, // erase
        b'H' | b'A' | b'B' | b'C' | b'D' | b'E' | b'F' | b'G' | b's' | b'u' => 2,
        _ => 3,
    }
}

fn scan(b: &[u8]) -> Option<Ansi> {
    let mut a = Ansi {
        csi: 0,
        sgr: 0,
        cursor_moves: 0,
        clears: 0,
        osc: 0,
        other_esc: 0,
        high_bytes: 0,
        lines: 0,
    };
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\n' => a.lines += 1,
            0x1b => {
                if b.get(i + 1) == Some(&b'[') {
                    // CSI: parameters/intermediates 0x20..=0x3F, final 0x40..=0x7E
                    let mut j = i + 2;
                    while j < b.len() && (0x20..=0x3f).contains(&b[j]) {
                        j += 1;
                    }
                    if j < b.len() && (0x40..=0x7e).contains(&b[j]) {
                        a.csi += 1;
                        match csi_kind(b[j]) {
                            0 => a.sgr += 1,
                            1 => a.clears += 1,
                            2 => a.cursor_moves += 1,
                            _ => {}
                        }
                        i = j;
                    } else {
                        a.other_esc += 1;
                    }
                } else if b.get(i + 1) == Some(&b']') {
                    a.osc += 1;
                    let mut j = i + 2;
                    while j < b.len() && b[j] != 0x07 {
                        if b[j] == 0x1b && b.get(j + 1) == Some(&0x5c) {
                            break;
                        }
                        j += 1;
                    }
                    i = j;
                } else {
                    a.other_esc += 1;
                }
            }
            x if x >= 0x80 => a.high_bytes += 1,
            _ => {}
        }
        i += 1;
    }
    (a.csi > 0 || a.osc > 0).then_some(a)
}

/// Detects ANSI content: at least one well-formed CSI or OSC sequence.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    scan(b).is_some()
}

/// Parses an ANSI buffer; `None` when no escape sequence is present.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ansi> {
    scan(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"\x1b[1;31mRED\x1b[0m\n\x1b[2J\x1b[1;1H\x80\xfeOK";

    #[test]
    fn parses() {
        let a = parse(D).unwrap();
        assert_eq!(a.csi, 4);
        assert_eq!(a.sgr, 2);
        assert_eq!(a.clears, 1);
        assert_eq!(a.cursor_moves, 1);
        assert_eq!(a.high_bytes, 2);
        assert_eq!(a.lines, 1);
    }

    #[test]
    fn osc_counted() {
        let a = parse(b"\x1b]0;title\x07text\x1b[0m").unwrap();
        assert_eq!(a.osc, 1);
        assert_eq!(a.sgr, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"plain text no escapes"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
        assert!(parse(b"").is_none());
    }
}
