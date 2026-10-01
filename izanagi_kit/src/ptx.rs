//! NVIDIA PTX (Parallel Thread Execution) text ISA — `.version X.Y`,
//! `.target sm_NN`, `.visible .entry`/`.func` kernels; directive and
//! opcode census.
//!
//! ```
//! let src = b".version 7.4\n.target sm_70\n\
//!     .visible .entry main() { ret; }\n";
//! let p = izanagi_kit::ptx::parse(src).unwrap();
//! assert_eq!(p.version_major, 7);
//! assert_eq!(p.entries, 1);
//! assert!(izanagi_kit::ptx::detect(src));
//! ```

/// Census of a PTX source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Ptx {
    /// `.version` major.
    pub version_major: u8,
    /// `.version` minor.
    pub version_minor: u8,
    /// `.target` SM index (e.g. 70 for `sm_70`).
    pub target_sm: u16,
    /// `.address_size` (32 or 64).
    pub address_size: u8,
    /// `.entry` kernels.
    pub entries: u32,
    /// `.func` device functions.
    pub functions: u32,
    /// `.visible` declarations.
    pub visible: u32,
    /// `.global`/`.shared`/`.local`/`.const`/`.param` declarations.
    pub memory_decls: u32,
    /// `.reg` register declarations.
    pub reg_decls: u32,
    /// Instruction-ish lines (contain `.` type suffix or opcode).
    pub instructions: u32,
    /// `//` comment lines.
    pub comments: u32,
    /// Lines seen.
    pub lines: u32,
}

fn word_after<'a>(line: &'a [u8], pat: &[u8]) -> Option<&'a [u8]> {
    line.windows(pat.len()).position(|w| w == pat).map(|i| {
        let mut s = &line[i + pat.len()..];
        while let Some((&c, rest)) = s.split_first() {
            if c == b' ' || c == b'\t' {
                s = rest;
            } else {
                break;
            }
        }
        let end = s
            .iter()
            .position(|c| !(c.is_ascii_alphanumeric() || *c == b'_' || *c == b'.'))
            .unwrap_or(s.len());
        &s[..end]
    })
}

fn parse_ver(b: &[u8]) -> (u8, u8) {
    let mut v = [0u8; 2];
    let mut part = 0;
    let mut cur = 0u16;
    let mut seen_digit = false;
    for &c in b {
        if c.is_ascii_digit() {
            cur = cur.saturating_mul(10).saturating_add((c - b'0') as u16);
            seen_digit = true;
        } else {
            if part < 2 {
                v[part] = cur.min(255) as u8;
            }
            part += 1;
            cur = 0;
            if part >= 2 {
                break;
            }
        }
    }
    if seen_digit && part < 2 {
        v[part] = cur.min(255) as u8;
    }
    (v[0], v[1])
}

fn parse_sm(b: &[u8]) -> u16 {
    let mut n = 0u16;
    for &c in b {
        if c.is_ascii_digit() {
            n = n.saturating_mul(10).saturating_add((c - b'0') as u16);
        }
    }
    n
}

/// `true` when a `.version` directive and `.target sm_` are present.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.windows(8).any(|w| w == *b".version") && b.windows(10).any(|w| w == *b".target sm")
}

/// Census; `None` without both directives.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ptx> {
    if !detect(b) {
        return None;
    }
    let mut p = Ptx {
        version_major: 0,
        version_minor: 0,
        target_sm: 0,
        address_size: 0,
        entries: 0,
        functions: 0,
        visible: 0,
        memory_decls: 0,
        reg_decls: 0,
        instructions: 0,
        comments: 0,
        lines: 0,
    };
    for line in b.split(|c| *c == b'\n') {
        p.lines += 1;
        if line.starts_with(b"//") {
            p.comments += 1;
            continue;
        }
        if let Some(v) = word_after(line, b".version") {
            let (maj, min) = parse_ver(v);
            p.version_major = maj;
            p.version_minor = min;
        }
        if let Some(t) = word_after(line, b".target") {
            if t.starts_with(b"sm_") {
                p.target_sm = parse_sm(&t[3..]);
            }
        }
        if let Some(a) = word_after(line, b".address_size") {
            p.address_size = parse_sm(a).min(255) as u8;
        }
        if line.windows(6).any(|w| w == *b".entry") {
            p.entries += 1;
        }
        if line.windows(5).any(|w| w == *b".func") {
            p.functions += 1;
        }
        if line.windows(8).any(|w| w == *b".visible") {
            p.visible += 1;
        }
        for pat in [&b".global"[..], b".shared", b".local", b".const", b".param"] {
            if line.windows(pat.len()).any(|w| w == pat) {
                p.memory_decls += 1;
                break;
            }
        }
        if line.windows(4).any(|w| w == *b".reg") {
            p.reg_decls += 1;
        }
        if !line.starts_with(b".")
            && !line.starts_with(b"//")
            && line.iter().any(|c| c.is_ascii_alphabetic())
        {
            p.instructions += 1;
        }
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"// demo\n\
        .version 7.4\n\
        .target sm_70\n\
        .address_size 64\n\
        .visible .entry main(.param .u64 p) {\n\
        .reg .b32 %r<4>;\n\
        ld.param.u64 %rd1, [p];\n\
        ret;\n\
        }\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(!detect(b"main() { ret; }"));
    }

    #[test]
    fn parses_directives() {
        let p = parse(SRC).unwrap();
        assert_eq!((p.version_major, p.version_minor), (7, 4));
        assert_eq!(p.target_sm, 70);
        assert_eq!(p.address_size, 64);
        assert_eq!(p.entries, 1);
        assert_eq!(p.visible, 1);
        assert_eq!(p.reg_decls, 1);
        assert!(p.memory_decls >= 1);
        assert!(p.instructions >= 2);
        assert_eq!(p.comments, 1);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"version 7.4 target sm_70").is_none());
    }
}
