//! Xilinx ISC (IEEE 1532 in-system configuration, `.isc`): ASCII
//! command stream — `ISC_INITIALIZE`, `ISC_PROGRAM`, `ISC_ERASE`,
//! `ISC_VERIFY`, `ISC_READ`, `ISC_SIR`, `ISC_SDR`, `ISC_RUNTEST`,
//! `ISC_IDENTIFIER`, `ISC_DATA`, `ISC_ENABLE`/`ISC_DISABLE`, each
//! `;`-terminated with `TDI()`/`TDO()` hex vectors and bit counts.
//!
//! ```
//! let d = b"// isc\r\nISC_ENABLE();\r\nISC_SIR 8 TDI (8A);\r\nISC_SDR 32 TDI (DEADBEEF) TDO (00000000);\r\nISC_RUNTEST 1000;\r\nISC_DISABLE();\r\n";
//! let p = izanagi_kit::isc::parse(d).unwrap();
//! assert_eq!(p.commands, 5);
//! assert_eq!(p.scan_bits, 40);
//! assert!(izanagi_kit::isc::detect(d));
//! ```

/// Census of an ISC command stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Isc {
    /// Total `ISC_*` commands parsed.
    pub commands: u32,
    /// `ISC_SIR`/`ISC_SDR` scans (and their bit totals below).
    pub sir_commands: u32,
    /// `ISC_SDR` commands.
    pub sdr_commands: u32,
    /// Summed shift bits across `ISC_SIR`/`ISC_SDR`.
    pub scan_bits: u64,
    /// `ISC_PROGRAM` commands.
    pub program_commands: u32,
    /// `ISC_ERASE`/`ISC_VERIFY`/`ISC_READ` commands.
    pub verify_commands: u32,
    /// `ISC_RUNTEST` commands and cycle totals.
    pub runtest_commands: u32,
    /// Total `RUNTEST` cycles.
    pub runtest_cycles: u64,
    /// `ISC_ENABLE`/`ISC_DISABLE` toggles.
    pub enable_commands: u32,
    /// `TDO` expected vectors.
    pub tdo_vectors: u32,
    /// `//` comment lines.
    pub comment_lines: u32,
    /// Non-command garbage seen between `;` boundaries.
    pub stray_text: bool,
}

fn has_isc(b: &[u8]) -> bool {
    b.windows(4).any(|w| w == *b"ISC_")
}

/// `true` when `ISC_` command prefixes appear.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 8 && has_isc(b)
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

fn word_u64(t: &[u8], i: &mut usize) -> u64 {
    let mut v = 0u64;
    while *i < t.len() && (t[*i].is_ascii_whitespace() || t[*i] == b'(') {
        *i += 1;
    }
    while *i < t.len() && t[*i].is_ascii_digit() {
        v = v.saturating_mul(10) + (t[*i] - b'0') as u64;
        *i += 1;
    }
    v
}

/// Census; `None` without `ISC_`. Statements end at `;`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Isc> {
    if !detect(b) {
        return None;
    }
    let mut s = Isc {
        commands: 0,
        sir_commands: 0,
        sdr_commands: 0,
        scan_bits: 0,
        program_commands: 0,
        verify_commands: 0,
        runtest_commands: 0,
        runtest_cycles: 0,
        enable_commands: 0,
        tdo_vectors: 0,
        comment_lines: 0,
        stray_text: false,
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
            if t.is_empty() {
                continue;
            }
            if t.len() >= 4 && &t[..4] == b"ISC_" {
                s.commands += 1;
                let mut i = 4usize;
                let mut w_end = i;
                while w_end < t.len() && (t[w_end].is_ascii_alphanumeric() || t[w_end] == b'_') {
                    w_end += 1;
                }
                let name = &t[i..w_end];
                i = w_end;
                match name {
                    b"SIR" | b"SDR" => {
                        let bits = word_u64(t, &mut i);
                        s.scan_bits += bits;
                        if name == b"SIR" {
                            s.sir_commands += 1;
                        } else {
                            s.sdr_commands += 1;
                        }
                    }
                    b"PROGRAM" => s.program_commands += 1,
                    b"ERASE" | b"VERIFY" | b"READ" | b"BLANK" => s.verify_commands += 1,
                    b"RUNTEST" => {
                        s.runtest_commands += 1;
                        s.runtest_cycles += word_u64(t, &mut i);
                    }
                    b"ENABLE" | b"DISABLE" => s.enable_commands += 1,
                    _ => {}
                }
                s.tdo_vectors += count_paren(t, b"TDO");
            } else {
                s.stray_text = true;
            }
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        b"// config\r\nISC_INITIALIZE();\r\nISC_ENABLE();\r\nISC_SIR 8 TDI (8A);\r\nISC_SDR 32 TDI (DEADBEEF) TDO (00000000) MASK (FF);\r\nISC_PROGRAM SECURITY;\r\nISC_RUNTEST 2000;\r\nISC_DISABLE();\r\n"
            .to_vec()
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"plain"));
        assert!(!detect(b"ISC"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.commands, 7);
        assert_eq!(p.sir_commands, 1);
        assert_eq!(p.sdr_commands, 1);
        assert_eq!(p.scan_bits, 40);
        assert_eq!(p.program_commands, 1);
        assert_eq!(p.runtest_cycles, 2000);
        assert_eq!(p.enable_commands, 2);
        assert_eq!(p.tdo_vectors, 1);
        assert_eq!(p.comment_lines, 1);
        assert!(!p.stray_text);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"nothing").is_none());
    }
}
