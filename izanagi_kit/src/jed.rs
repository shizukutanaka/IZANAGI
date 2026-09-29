//! JEDEC fusemap (`.jed`, JEDEC JESD3-C): text records, each ending
//! `*` — `QF` total fuse count, `F` default fuse state, `G` security
//! fuse, `L` fuse arrays (`L<address> <bits>`), `C` transmission
//! checksum, `N`/`D`/`E` note/device records, `ETX` end.
//!
//! ```
//! let d = b"\x02QF1024*\r\nF0*\r\nL0000 11110000*\r\nL0016 10101010*\r\nC0A5*\r\nN device* \x03";
//! let p = izanagi_kit::jed::parse(d).unwrap();
//! assert_eq!(p.qf, 1024);
//! assert_eq!(p.l_blocks, 2);
//! assert_eq!(p.set_bits, 8);
//! assert!(izanagi_kit::jed::detect(d));
//! ```

/// Census of a JEDEC fusemap file.
#[derive(Debug, Clone, PartialEq)]
pub struct Jed {
    /// `QF` declared total fuse count.
    pub qf: u32,
    /// `F` default fuse state (0/1), `None` when absent.
    pub default_state: Option<u8>,
    /// `L` fuse-array records.
    pub l_blocks: u32,
    /// `1` bits across all `L` records.
    pub set_bits: u32,
    /// `0` bits across all `L` records.
    pub clear_bits: u32,
    /// Highest fuse address covered by `L` records.
    pub max_addr: u32,
    /// `C` checksum field (hex) when present.
    pub checksum: Option<u16>,
    /// `G` security-fuse record present.
    pub has_security: bool,
    /// `N`/`D`/`E` note records.
    pub notes: u32,
    /// `STX`/`ETX` framing seen.
    pub framed: bool,
    /// `L`/`QF` address arithmetic overflowed the buffer census.
    pub truncated: bool,
}

fn is_jed(b: &[u8]) -> bool {
    // `QF` + digits + `*`, or an `L<addr>` record.
    let has_qf = b
        .windows(3)
        .any(|w| w[0] == b'Q' && w[1] == b'F' && w[2].is_ascii_digit());
    let has_l = b.windows(2).any(|w| w[0] == b'L' && w[1].is_ascii_digit());
    has_qf && has_l
}

/// `true` on `QF…*…L…` JEDEC structure.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 8 && is_jed(b)
}

/// Census; `None` without JEDEC markers. Records are terminated by
/// `*`; `L` records carry `<addr> <fuse bits>` (decimal start addr).
#[must_use]
pub fn parse(b: &[u8]) -> Option<Jed> {
    if !detect(b) {
        return None;
    }
    let mut j = Jed {
        qf: 0,
        default_state: None,
        l_blocks: 0,
        set_bits: 0,
        clear_bits: 0,
        max_addr: 0,
        checksum: None,
        has_security: false,
        notes: 0,
        framed: false,
        truncated: false,
    };
    if b.first() == Some(&2) {
        j.framed = true;
    }
    // Split into `*`-terminated records; whitespace separates
    // tokens inside a record (`L<addr> <bit groups>`).
    for rec in b.split(|c| *c == b'*') {
        let mut toks = rec
            .split(|c| c.is_ascii_whitespace())
            .filter(|t| !t.is_empty());
        let Some(mut head) = toks.next() else {
            continue;
        };
        // Skip STX/ETX and other control bytes that share the token.
        while head.len() > 1 && head[0] < 0x20 {
            head = &head[1..];
        }
        match head[0] {
            b'Q' if head.len() > 2 && head[1] == b'F' => {
                j.qf = atoi(&head[2..]);
            }
            b'F' if head.len() == 2 => {
                j.default_state = Some(head[1] - b'0');
            }
            b'L' if head.len() > 1 && head[1].is_ascii_digit() => {
                let addr = atoi(&head[1..]);
                j.l_blocks += 1;
                for t in toks {
                    for &c in t {
                        if c == b'1' {
                            j.set_bits += 1;
                        } else if c == b'0' {
                            j.clear_bits += 1;
                        }
                    }
                }
                if addr > j.max_addr {
                    j.max_addr = addr;
                }
            }
            b'C' if head.len() > 1 => {
                j.checksum = Some(hexval(&head[1..]) as u16);
            }
            b'G' => j.has_security = true,
            b'N' | b'D' | b'E' if head.len() == 1 || head[1] != b'T' => {
                j.notes += 1;
            }
            _ => {}
        }
    }
    if b.last() == Some(&3) {
        j.framed = true;
    }
    Some(j)
}

fn atoi(d: &[u8]) -> u32 {
    let mut v = 0u32;
    for &c in d {
        if !c.is_ascii_digit() {
            break;
        }
        v = v.saturating_mul(10).saturating_add((c - b'0') as u32);
    }
    v
}

fn hexval(d: &[u8]) -> u32 {
    let mut v = 0u32;
    for &c in d {
        let h = match c {
            b'0'..=b'9' => c - b'0',
            b'A'..=b'F' => c - b'A' + 10,
            b'a'..=b'f' => c - b'a' + 10,
            _ => break,
        };
        v = v.saturating_mul(16).saturating_add(h as u32);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        b"\x02QF64*\r\nF0*\r\nL0000 111100001111*\r\nL0012 00001111*\r\nC0FF*\r\nN PART-16V8*\r\n\x03"
            .to_vec()
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"QF only"));
        assert!(!detect(b"L0000 1010*"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.qf, 64);
        assert_eq!(p.default_state, Some(0));
        assert_eq!(p.l_blocks, 2);
        assert_eq!(p.set_bits, 8 + 4);
        assert_eq!(p.clear_bits, 4 + 4);
        assert_eq!(p.max_addr, 12);
        assert_eq!(p.checksum, Some(0xFF));
        assert!(p.framed);
        assert_eq!(p.notes, 1);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"just text").is_none());
    }
}
