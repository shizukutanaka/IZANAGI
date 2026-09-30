//! Serial Vector Format (`.svf`, JTAG stimulus, IEEE 1149.1):
//! ASCII commands — `HIR`/`TIR`/`HDR`/`TDR`/`SIR`/`SDR` with bit
//! counts and `TDI()`/`TDO()`/`MASK()`/`SMASK()` hex vectors,
//! `RUNTEST`, `STATE`, `ENDIR`/`ENDDR`, `FREQUENCY`, `TRST`,
//! `//` comments.
//!
//! ```
//! let d = b"// svf\r\nHIR 0;\r\nSIR 8 TDI (FE);\r\nSDR 32 TDI (DEADBEEF) TDO (0) MASK (FF);\r\nRUNTEST 100 TCK;\r\nSTATE RESET;\r\n";
//! let p = izanagi_kit::svf::parse(d).unwrap();
//! assert_eq!(p.sir_commands, 1);
//! assert_eq!(p.sdr_commands, 1);
//! assert_eq!(p.scanned_bits, 40);
//! assert!(izanagi_kit::svf::detect(d));
//! ```

/// Census of an SVF stimulus file.
#[derive(Debug, Clone, PartialEq)]
pub struct Svf {
    /// `HIR`/`TIR`/`HDR`/`TDR` header/trailer commands.
    pub pad_commands: u32,
    /// `SIR` scan-instruction commands.
    pub sir_commands: u32,
    /// `SDR` scan-data commands.
    pub sdr_commands: u32,
    /// Total shift bits across `SIR`/`SDR`.
    pub scanned_bits: u64,
    /// `TDO` vectors (expected-compare count).
    pub tdo_vectors: u32,
    /// `MASK` vectors.
    pub mask_vectors: u32,
    /// `RUNTEST` commands and their total cycle counts.
    pub runtest_commands: u32,
    /// Total `RUNTEST` cycles.
    pub runtest_cycles: u64,
    /// `STATE` commands.
    pub state_commands: u32,
    /// `ENDIR`/`ENDDR` commands.
    pub end_commands: u32,
    /// `TRST` commands.
    pub trst_commands: u32,
    /// `FREQUENCY` commands.
    pub frequency_commands: u32,
    /// `//` comment lines.
    pub comment_lines: u32,
    /// A vector was cut off mid-token.
    pub truncated: bool,
}

const CMDS: [&[u8]; 12] = [
    b"HIR",
    b"TIR",
    b"HDR",
    b"TDR",
    b"SIR",
    b"SDR",
    b"RUNTEST",
    b"STATE",
    b"ENDIR",
    b"ENDDR",
    b"TRST",
    b"FREQUENCY",
];

fn has_cmd(b: &[u8]) -> bool {
    CMDS.iter().any(|c| {
        b.windows(c.len() + 1)
            .any(|w| w[..c.len()] == **c && (w[c.len()] == b' ' || w[c.len()] == b'\t'))
    })
}

/// `true` when an SVF command word (`SIR`/`SDR`/`HIR`…) appears
/// followed by whitespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 6 && has_cmd(b)
}

fn word_u64(r: &[u8], i: &mut usize) -> u64 {
    let mut v = 0u64;
    while *i < r.len() && (r[*i].is_ascii_whitespace() || r[*i] == b'(') {
        *i += 1;
    }
    while *i < r.len() && r[*i].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add((r[*i] - b'0') as u64);
        *i += 1;
    }
    v
}

/// Count `key (` occurrences allowing whitespace before `(`.
fn count_paren(t: &[u8], key: &[u8]) -> u32 {
    let mut n = 0;
    let mut i = 0;
    while i + key.len() <= t.len() {
        if &t[i..i + key.len()] == key {
            let mut j = i + key.len();
            while j < t.len() && t[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < t.len() && t[j] == b'(' {
                n += 1;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    n
}

/// Census; `None` without SVF commands. Each statement ends at `;`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Svf> {
    if !detect(b) {
        return None;
    }
    let mut s = Svf {
        pad_commands: 0,
        sir_commands: 0,
        sdr_commands: 0,
        scanned_bits: 0,
        tdo_vectors: 0,
        mask_vectors: 0,
        runtest_commands: 0,
        runtest_cycles: 0,
        state_commands: 0,
        end_commands: 0,
        trst_commands: 0,
        frequency_commands: 0,
        comment_lines: 0,
        truncated: false,
    };
    for line in b.split(|c| *c == b'\n') {
        let lt = {
            let mut k = 0;
            while k < line.len() && line[k].is_ascii_whitespace() {
                k += 1;
            }
            &line[k..]
        };
        if lt.starts_with(b"//") {
            s.comment_lines += 1;
            continue;
        }
        for stmt in lt.split(|c| *c == b';') {
            let t = {
                let mut k = 0;
                while k < stmt.len() && stmt[k].is_ascii_whitespace() {
                    k += 1;
                }
                &stmt[k..]
            };
            let cmd: &[u8] = {
                let mut hit: &[u8] = b"";
                for c in CMDS {
                    if t.len() >= c.len() && &t[..c.len()] == c {
                        hit = c;
                        break;
                    }
                }
                hit
            };
            if cmd.is_empty() {
                continue;
            }
            let mut i = cmd.len();
            match cmd {
                b"HIR" | b"TIR" | b"HDR" | b"TDR" => s.pad_commands += 1,
                b"SIR" | b"SDR" => {
                    let bits = word_u64(t, &mut i);
                    s.scanned_bits = s.scanned_bits.saturating_add(bits);
                    if cmd == b"SIR" {
                        s.sir_commands += 1;
                    } else {
                        s.sdr_commands += 1;
                    }
                }
                b"RUNTEST" => {
                    s.runtest_commands += 1;
                    s.runtest_cycles = s.runtest_cycles.saturating_add(word_u64(t, &mut i));
                }
                b"STATE" => s.state_commands += 1,
                b"ENDIR" | b"ENDDR" => s.end_commands += 1,
                b"TRST" => s.trst_commands += 1,
                b"FREQUENCY" => s.frequency_commands += 1,
                _ => {}
            }
            s.tdo_vectors += count_paren(t, b"TDO");
            s.mask_vectors += count_paren(t, b"MASK");
        }
    }
    if s.sir_commands == 0 && s.sdr_commands == 0 && s.pad_commands == 0 {
        s.truncated = true;
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_counts_saturate() {
        let s = parse(b"SIR 18446744073709551615;\nSIR 99999999999999999999999;\n").unwrap();
        assert_eq!(s.scanned_bits, u64::MAX);
    }

    fn fixture() -> Vec<u8> {
        b"// test\r\nHIR 4 TDI (0) SMASK (F);\r\nSIR 10 TDI (355) TDO (0) MASK (3FF);\r\nSDR 24 TDI (A5A5A5);\r\nRUNTEST 250 TCK;\r\nSTATE RESET;\r\nENDDR IDLE;\r\n"
            .to_vec()
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"plain text"));
        assert!(!detect(b"SIRX 8 TDI(0)"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.pad_commands, 1);
        assert_eq!(p.sir_commands, 1);
        assert_eq!(p.sdr_commands, 1);
        assert_eq!(p.scanned_bits, 34);
        assert_eq!(p.runtest_cycles, 250);
        assert_eq!(p.state_commands, 1);
        assert_eq!(p.end_commands, 1);
        assert_eq!(p.comment_lines, 1);
        assert_eq!(p.tdo_vectors, 1);
        assert_eq!(p.mask_vectors, 2);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"not svf").is_none());
    }
}
