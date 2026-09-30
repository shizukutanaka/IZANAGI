//! Rigetti Quil quantum instruction language parser.
//!
//! Line-oriented census: `DECLARE`/`DEFCIRCUIT`/`DEFGATE`/`DEFFRAME`/
//! `DEFWAVEFORM`/`DEFFCAL` definitions, `MEASURE`, `RESET`, `PRAGMA`,
//! `PULSE`/`CAPTURE` pulse ops, `HALT`/`JUMP`/`LABEL` control flow and
//! remaining uppercase gate applications.
//!
//! ```
//! use izanagi_kit::quil::Quil;
//! let src = b"DECLARE ro BIT[2]\nH 0\nCNOT 0 1\nMEASURE 0 ro[0]\nMEASURE 1 ro[1]\n";
//! assert!(izanagi_kit::quil::detect(src));
//! let q = Quil::parse(src).unwrap();
//! assert_eq!(q.declarations, 1);
//! assert_eq!(q.measurements, 2);
//! assert_eq!(q.gate_applications, 2);
//! ```

/// Parsed census of a Quil program.
#[derive(Debug, Clone)]
pub struct Quil {
    /// `DECLARE name TYPE[...]` memory declarations.
    pub declarations: usize,
    /// `DECLARE ... BIT`/`REAL`/`OCTET`/`INTEGER` typed declarations.
    pub typed_declarations: usize,
    /// `DEFCIRCUIT name(params) vars:` definitions.
    pub defcircuits: usize,
    /// `DEFGATE name(...) AS ...:` definitions.
    pub defgates: usize,
    /// `DEFFRAME`/`DEFWAVEFORM`/`DEFFCAL` frame-level definitions.
    pub def_frames: usize,
    /// `MEASURE q [addr]` statements.
    pub measurements: usize,
    /// `RESET`/`RESET q` statements.
    pub resets: usize,
    /// `PRAGMA name ...` directives.
    pub pragmas: usize,
    /// `PULSE`/`CAPTURE`/`RAW-CAPTURE`/`DELAY`/`FENCE` pulse instructions.
    pub pulse_ops: usize,
    /// `JUMP`/`JUMP-WHEN`/`JUMP-UNLESS`/`LABEL`/`HALT`/`WAIT`/`NOP` control flow.
    pub control_flow: usize,
    /// Remaining instructions counted as gate applications.
    pub gate_applications: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Returns `true` when `b` looks like a Quil program.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.lines().any(|l| {
        let l = l.trim_start();
        l.starts_with("DECLARE ")
            || l.starts_with("DEFCIRCUIT")
            || l.starts_with("DEFGATE")
            || l.starts_with("MEASURE ")
            || l.starts_with("MEASURE")
    })
}

impl Quil {
    /// Parses a Quil program; `None` without at least one Quil keyword.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let mut q = Self {
            declarations: 0,
            typed_declarations: 0,
            defcircuits: 0,
            defgates: 0,
            def_frames: 0,
            measurements: 0,
            resets: 0,
            pragmas: 0,
            pulse_ops: 0,
            control_flow: 0,
            gate_applications: 0,
            comments: 0,
        };
        let mut hits = 0usize;
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                q.comments += 1;
                continue;
            }
            let kw = l.split_whitespace().next().unwrap_or("");
            match kw {
                "DECLARE" => {
                    q.declarations += 1;
                    hits += 1;
                    if l.split_whitespace().nth(2).is_some_and(|w| {
                        matches!(w, "BIT" | "REAL" | "OCTET" | "INTEGER")
                            || w.starts_with("BIT[")
                            || w.starts_with("REAL[")
                            || w.starts_with("OCTET[")
                            || w.starts_with("INTEGER[")
                    }) {
                        q.typed_declarations += 1;
                    }
                }
                "DEFCIRCUIT" => {
                    q.defcircuits += 1;
                    hits += 1;
                }
                "DEFGATE" => {
                    q.defgates += 1;
                    hits += 1;
                }
                "DEFFRAME" | "DEFWAVEFORM" | "DEFFCAL" => {
                    q.def_frames += 1;
                    hits += 1;
                }
                "MEASURE" => {
                    q.measurements += 1;
                    hits += 1;
                }
                "RESET" => q.resets += 1,
                "PRAGMA" => {
                    q.pragmas += 1;
                    hits += 1;
                }
                "PULSE" | "CAPTURE" | "RAW-CAPTURE" | "DELAY" | "FENCE" => {
                    q.pulse_ops += 1;
                    hits += 1;
                }
                "JUMP" | "JUMP-WHEN" | "JUMP-UNLESS" | "LABEL" | "HALT" | "WAIT" | "NOP" => {
                    q.control_flow += 1;
                }
                _ => {
                    if !l.is_empty() {
                        q.gate_applications += 1;
                    }
                }
            }
        }
        (hits > 0).then_some(q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_declare() {
        assert!(detect(b"DECLARE ro BIT\n"));
        assert!(!detect(b"hello world\n"));
    }

    #[test]
    fn counts_pulses() {
        let src = b"DEFFRAME 0 \"rx\":\nPULSE 0 \"rx\" flat(duration: 1, iq: 1)\nCAPTURE 0 \"ro\" flat(duration: 1, iq: 1) addr\n";
        let q = Quil::parse(src).unwrap();
        assert_eq!(q.def_frames, 1);
        assert_eq!(q.pulse_ops, 2);
    }

    #[test]
    fn parse_none_on_garbage() {
        assert!(Quil::parse(b"nothing here").is_none());
    }
}
