//! Lattice Preference File (`.lpf`, ispLEVER/Diamond): ASCII
//! constraints — `BLOCK`/`UNBLOCK`, `LOCATE COMP "name" SITE "pin"`,
//! `SYSCONFIG`, `FREQUENCY PORT "clk" NNN MHz`, `IOBUF PORT`,
//! `PREFER`, `TIMESPEC`; `#` comments, `;` statement terminator.
//!
//! ```
//! let d = b"# lpf\nLOCATE COMP \"clk\" SITE \"A8\";\nFREQUENCY PORT \"clk\" 100.0 MHz;\nIOBUF PORT \"din\" IO_TYPE=LVCMOS33;\nSYSCONFIG CONFIG_MODE=SERIAL;\n";
//! let p = izanagi_kit::lpf::parse(d).unwrap();
//! assert_eq!(p.locate, 1);
//! assert_eq!(p.frequency, 1);
//! assert!(izanagi_kit::lpf::detect(d));
//! ```

/// Census of an LPF constraints file.
#[derive(Debug, Clone, PartialEq)]
pub struct Lpf {
    /// `LOCATE COMP … SITE "…"` pin placements.
    pub locate: u32,
    /// `FREQUENCY`/`PERIOD` preferences.
    pub frequency: u32,
    /// `IOBUF` buffer-type preferences.
    pub iobuf: u32,
    /// `SYSCONFIG` sections.
    pub sysconfig: u32,
    /// `BLOCK`/`UNBLOCK` path filters.
    pub block: u32,
    /// `PREFER`/`PROHIBIT` preferences.
    pub prefer: u32,
    /// `TIMESPEC` declarations.
    pub timespec: u32,
    /// `SLICE`/`UGROUP`/`ASIGPATH` misc preferences.
    pub misc_prefs: u32,
    /// `SITE "…"` quoted site names.
    pub sites: u32,
    /// `MHz`/`KHz`/`Hz` frequency tokens.
    pub freq_tokens: u32,
    /// `#` comment lines.
    pub comment_lines: u32,
    /// Statements without terminating `;`.
    pub unterminated: bool,
}

const MARKERS: [&[u8]; 6] = [
    b"LOCATE",
    b"SYSCONFIG",
    b"FREQUENCY",
    b"IOBUF",
    b"BLOCK",
    b"TIMESPEC",
];

/// `true` on any uppercase LPF keyword.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 8
        && MARKERS.iter().any(|m| {
            b.windows(m.len() + 1)
                .any(|w| w[..m.len()] == **m && (w[m.len()] == b' ' || w[m.len()] == b'\t'))
        })
}

/// Census; `None` without LPF keywords.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Lpf> {
    if !detect(b) {
        return None;
    }
    let mut l = Lpf {
        locate: 0,
        frequency: 0,
        iobuf: 0,
        sysconfig: 0,
        block: 0,
        prefer: 0,
        timespec: 0,
        misc_prefs: 0,
        sites: 0,
        freq_tokens: 0,
        comment_lines: 0,
        unterminated: false,
    };
    for line in b.split(|c| *c == b'\n') {
        let t = {
            let mut k = 0;
            while k < line.len() && line[k].is_ascii_whitespace() {
                k += 1;
            }
            &line[k..]
        };
        if t.starts_with(b"#") {
            l.comment_lines += 1;
            continue;
        }
        for stmt in t.split(|c| *c == b';') {
            let s0 = {
                let mut k = 0;
                while k < stmt.len() && stmt[k].is_ascii_whitespace() {
                    k += 1;
                }
                &stmt[k..]
            };
            if s0.is_empty() {
                continue;
            }
            let mut w_end = 0;
            while w_end < s0.len() && (s0[w_end].is_ascii_alphanumeric() || s0[w_end] == b'_') {
                w_end += 1;
            }
            match &s0[..w_end] {
                b"LOCATE" => l.locate += 1,
                b"FREQUENCY" | b"PERIOD" => l.frequency += 1,
                b"IOBUF" => l.iobuf += 1,
                b"SYSCONFIG" => l.sysconfig += 1,
                b"BLOCK" | b"UNBLOCK" => l.block += 1,
                b"PREFER" | b"PROHIBIT" => l.prefer += 1,
                b"TIMESPEC" => l.timespec += 1,
                b"SLICE" | b"UGROUP" | b"ASIGPATH" | b"VOLTAGE" | b"CLAMP" => l.misc_prefs += 1,
                _ => {
                    l.unterminated |= !s0[..w_end].is_empty() && s0[..w_end][0].is_ascii_uppercase()
                }
            }
            l.sites += s0.windows(5).filter(|w| **w == *b"SITE ").count() as u32;
            l.freq_tokens += (s0.windows(3).filter(|w| **w == *b"MHz").count()
                + s0.windows(3).filter(|w| **w == *b"KHz").count())
                as u32;
        }
    }
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        b"# lattice\nLOCATE COMP \"clk\" SITE \"A8\";\nLOCATE COMP \"din\" SITE \"B1\";\nFREQUENCY PORT \"clk\" 100.0 MHz;\nIOBUF PORT \"din\" IO_TYPE=LVCMOS33 PULLMODE=UP;\nSYSCONFIG CONFIG_MODE=SERIAL COMPRESS_CONFIG=ON;\nBLOCK RESETPATHS;\nTIMESPEC TS1 = PERIOD \"clk\" 10 ns;\n"
            .to_vec()
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"plain text"));
        assert!(!detect(b"LOCATE"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.locate, 2);
        assert_eq!(p.frequency, 1);
        assert_eq!(p.iobuf, 1);
        assert_eq!(p.sysconfig, 1);
        assert_eq!(p.block, 1);
        assert_eq!(p.timespec, 1);
        assert_eq!(p.sites, 2);
        assert_eq!(p.freq_tokens, 1);
        assert_eq!(p.comment_lines, 1);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"nothing").is_none());
    }
}
