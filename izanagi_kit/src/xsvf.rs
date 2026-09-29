//! Xilinx XSVF (`.xsvf`, binary JTAG stimulus): opcode stream —
//! `XCOMPLETE`(0), `XTDOMASK`(1), `XSIR`(2), `XSDR`(3),
//! `XRUNTEST`(4), `XREPEAT`(7), `XSDRSIZE`(8), `XSTATE`(12),
//! `XENDIR`(13), `XENDDR`(14), `XSIR2`(18), `XCOMMENT`(19),
//! `XWAIT`(20). File begins with `XSDRSIZE` + `u32be` max bit count;
//! vector args are `(bits+7)/8` bytes.
//!
//! ```
//! let mut d = vec![8u8]; // XSDRSIZE
//! d.extend_from_slice(&32u32.to_be_bytes());
//! d.push(3); // XSDR + 4 payload bytes
//! d.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
//! d.push(0); // XCOMPLETE
//! let p = izanagi_kit::xsvf::parse(&d).unwrap();
//! assert_eq!(p.sdr_size_bits, 32);
//! assert_eq!(p.sdr_commands, 1);
//! assert!(izanagi_kit::xsvf::detect(&d));
//! ```

/// Census of an XSVF binary stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Xsvf {
    /// `XSDRSIZE` value — max SDR record bits.
    pub sdr_size_bits: u32,
    /// `XSIR`/`XSIR2` instruction-shift commands.
    pub sir_commands: u32,
    /// `XSDR` data-shift commands.
    pub sdr_commands: u32,
    /// `XRUNTEST` commands and their summed cycle counts.
    pub runtest_commands: u32,
    /// Total `XRUNTEST`/`XWAIT` microseconds.
    pub runtest_cycles: u64,
    /// `XREPEAT` commands (summed counts).
    pub repeat_count: u64,
    /// `XSTATE`/`XENDIR`/`XENDDR` state commands.
    pub state_commands: u32,
    /// `XTDOMASK` commands.
    pub tdo_masks: u32,
    /// `XCOMMENT` records.
    pub comments: u32,
    /// `XCOMPLETE` terminator reached.
    pub complete: bool,
    /// An opcode's argument ran past the buffer.
    pub truncated: bool,
    /// Unknown opcode encountered.
    pub unknown_opcodes: u32,
}

fn be32(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 24) | ((b[i + 1] as u32) << 16) | ((b[i + 2] as u32) << 8) | b[i + 3] as u32
}
fn be16(b: &[u8], i: usize) -> u16 {
    ((b[i] as u16) << 8) | b[i + 1] as u16
}

/// `true` when the stream begins with `XSDRSIZE` + a positive
/// `u32be` bit size.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 5 && b[0] == 8 && be32(b, 1) > 0 && be32(b, 1) <= (1 << 24)
}

/// Census; `None` without `XSDRSIZE`. Vector lengths derive from
/// `sdr_size_bits` (`(bits+7)/8` bytes); `XSIR`/`XSIR2` carry their
/// own bit counts.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xsvf> {
    if !detect(b) {
        return None;
    }
    let mut x = Xsvf {
        sdr_size_bits: be32(b, 1),
        sir_commands: 0,
        sdr_commands: 0,
        runtest_commands: 0,
        runtest_cycles: 0,
        repeat_count: 0,
        state_commands: 0,
        tdo_masks: 0,
        comments: 0,
        complete: false,
        truncated: false,
        unknown_opcodes: 0,
    };
    let sdr_bytes = (x.sdr_size_bits as usize).saturating_add(7) / 8;
    let mut i = 5usize;
    let mut steps = 0;
    while i < b.len() && steps < 65536 {
        steps += 1;
        let op = b[i];
        i += 1;
        match op {
            0 => {
                x.complete = true;
                break;
            }
            1 => {
                // XTDOMASK
                if i + sdr_bytes > b.len() {
                    x.truncated = true;
                    break;
                }
                x.tdo_masks += 1;
                i += sdr_bytes;
            }
            2 => {
                // XSIR: u8 bits + bytes
                if i + 1 > b.len() {
                    x.truncated = true;
                    break;
                }
                let bits = b[i] as usize;
                i += 1;
                let nb = bits.div_ceil(8);
                if i + nb > b.len() {
                    x.truncated = true;
                    break;
                }
                x.sir_commands += 1;
                i += nb;
            }
            3 => {
                // XSDR
                if i + sdr_bytes > b.len() {
                    x.truncated = true;
                    break;
                }
                x.sdr_commands += 1;
                i += sdr_bytes;
            }
            4 => {
                // XRUNTEST u32be
                if i + 4 > b.len() {
                    x.truncated = true;
                    break;
                }
                x.runtest_commands += 1;
                x.runtest_cycles += be32(b, i) as u64;
                i += 4;
            }
            7 => {
                // XREPEAT u8
                if i + 1 > b.len() {
                    x.truncated = true;
                    break;
                }
                x.repeat_count += b[i] as u64;
                i += 1;
            }
            8 => {
                // XSDRSIZE again
                if i + 4 > b.len() {
                    x.truncated = true;
                    break;
                }
                i += 4;
            }
            12..=14 => {
                // XSTATE/XENDIR/XENDDR: u8
                if i + 1 > b.len() {
                    x.truncated = true;
                    break;
                }
                x.state_commands += 1;
                i += 1;
            }
            18 => {
                // XSIR2: u16be bits + bytes
                if i + 2 > b.len() {
                    x.truncated = true;
                    break;
                }
                let bits = be16(b, i) as usize;
                i += 2;
                let nb = bits.div_ceil(8);
                if i + nb > b.len() {
                    x.truncated = true;
                    break;
                }
                x.sir_commands += 1;
                i += nb;
            }
            19 => {
                // XCOMMENT: bytes until 0x00
                let mut j = i;
                while j < b.len() && b[j] != 0 {
                    j += 1;
                }
                if j >= b.len() {
                    x.truncated = true;
                    break;
                }
                x.comments += 1;
                i = j + 1;
            }
            20 => {
                // XWAIT: u8 u8 u32be
                if i + 6 > b.len() {
                    x.truncated = true;
                    break;
                }
                x.runtest_commands += 1;
                x.runtest_cycles += be32(b, i + 2) as u64;
                i += 6;
            }
            _ => {
                x.unknown_opcodes += 1;
            }
        }
    }
    if i >= b.len() && !x.complete {
        x.truncated = true;
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![8u8];
        d.extend_from_slice(&16u32.to_be_bytes()); // sdr size 16
        d.push(2); // XSIR, 8 bits
        d.push(8);
        d.push(0xA5);
        d.push(4); // XRUNTEST 100
        d.extend_from_slice(&100u32.to_be_bytes());
        d.push(3); // XSDR 2 bytes
        d.extend_from_slice(&[0x12, 0x34]);
        d.push(12); // XSTATE IDLE
        d.push(0);
        d.push(0); // XCOMPLETE
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"xsvf"));
        assert!(!detect(&[8, 0, 0, 0, 0]));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.sdr_size_bits, 16);
        assert_eq!(p.sir_commands, 1);
        assert_eq!(p.sdr_commands, 1);
        assert_eq!(p.runtest_cycles, 100);
        assert_eq!(p.state_commands, 1);
        assert!(p.complete);
        assert!(!p.truncated);
    }

    #[test]
    fn truncated_payload() {
        let mut d = fixture();
        let n = d.len();
        d.truncate(n - 4); // cut inside XSDR payload
        let p = parse(&d).unwrap();
        assert!(p.truncated);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"hello").is_none());
    }
}
