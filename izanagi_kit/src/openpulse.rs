//! OpenPulse (OpenQASM 3 pulse-level grammar) parser.
//!
//! Counts `cal`/`defcal` calibration blocks, `port`/`frame`/`waveform`
//! declarations, `play`/`capture`/`delay`/`shift_phase`/`set_phase`/
//! `set_frequency`/`shift_frequency` pulse statements and `extern`
//! declarations.
//!
//! ```
//! use izanagi_kit::openpulse::OpenPulse;
//! let src = b"OPENQASM 3\x2e0;\ndefcalgrammar \"openpulse\";\ncal {\n    port p0;\n    extern drag(complex[size], duration, angle, angle) -> waveform;\n}\ndefcal x $0 {\n    play(drag(1, 2, 3, 4), frame0);\n}\n";
//! assert!(izanagi_kit::openpulse::detect(src));
//! let q = OpenPulse::parse(src).unwrap();
//! assert_eq!(q.cals, 1);
//! assert_eq!(q.defcals, 1);
//! assert_eq!(q.ports, 1);
//! assert_eq!(q.externs, 1);
//! assert_eq!(q.plays, 1);
//! ```

/// Parsed census of an OpenPulse program.
#[derive(Debug, Clone)]
pub struct OpenPulse {
    /// `cal { ... }` calibration blocks.
    pub cals: usize,
    /// `defcal name qargs { ... }` calibration definitions.
    pub defcals: usize,
    /// `defcalgrammar "openpulse";` directives.
    pub defcalgrammars: usize,
    /// `port name;` declarations.
    pub ports: usize,
    /// `frame name = newframe(...)` declarations.
    pub frames: usize,
    /// `waveform name = ...` declarations.
    pub waveforms: usize,
    /// `extern name(...) -> type;` declarations.
    pub externs: usize,
    /// `play(waveform, frame)` statements.
    pub plays: usize,
    /// `capture(frame)`/`capture_vN` statements.
    pub captures: usize,
    /// `delay[...]` statements.
    pub delays: usize,
    /// `set_phase`/`shift_phase`/`set_frequency`/`shift_frequency`/`set_scale` statements.
    pub phase_freq: usize,
    /// `//` comment lines.
    pub comments: usize,
}

fn word_count(t: &str, kw: &str) -> usize {
    let mut n = 0usize;
    for line in t.lines() {
        let l = line.trim();
        if l.split_whitespace().next() == Some(kw) {
            n += 1;
        }
    }
    n
}
fn code_has(t: &str, needle: &str) -> bool {
    // `//`/`/*` コメント行内の言及は証拠にしない。
    t.lines().any(|l| {
        let l = l.trim_start();
        !l.starts_with("//") && !l.starts_with("/*") && l.contains(needle)
    })
}

/// Returns `true` when `b` looks like an OpenPulse program.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    code_has(t, "defcal")
        || (code_has(t, "cal {")
            && (code_has(t, "frame") || code_has(t, "port ") || code_has(t, "waveform")))
        || code_has(t, "defcalgrammar")
}

impl OpenPulse {
    /// Parses an OpenPulse program; `None` with fewer than one pulse keyword.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let mut q = Self {
            cals: word_count(t, "cal"),
            defcals: word_count(t, "defcal"),
            defcalgrammars: word_count(t, "defcalgrammar"),
            ports: word_count(t, "port"),
            frames: word_count(t, "frame"),
            waveforms: word_count(t, "waveform"),
            externs: word_count(t, "extern"),
            plays: t
                .lines()
                .filter(|l| {
                    let l = l.trim();
                    l.starts_with("play(") || l.starts_with("play ")
                })
                .count(),
            captures: word_count(t, "capture") + t.matches("capture_v").count(),
            delays: 0,
            phase_freq: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.starts_with("//") {
                q.comments += 1;
            }
            if l.starts_with("delay") {
                q.delays += 1;
            }
            for kw in [
                "set_phase",
                "shift_phase",
                "set_frequency",
                "shift_frequency",
                "set_scale",
            ] {
                if l.starts_with(kw) {
                    q.phase_freq += 1;
                    break;
                }
            }
        }
        let hits = q.cals + q.defcals + q.defcalgrammars + q.ports + q.frames + q.waveforms;
        (hits > 0).then_some(q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"// defcal x %0;\n"));
    }

    #[test]
    fn detects_defcal() {
        assert!(detect(b"defcal x $0 { play(w, f); }\n"));
        assert!(!detect(b"hello"));
    }

    #[test]
    fn counts_cal_block() {
        let src = b"cal {\n    port p0;\n    frame f0 = newframe(p0, 5e9, 0);\n}\n";
        let q = OpenPulse::parse(src).unwrap();
        assert_eq!(q.cals, 1);
        assert_eq!(q.ports, 1);
        assert_eq!(q.frames, 1);
    }

    #[test]
    fn parse_none_on_garbage() {
        assert!(OpenPulse::parse(b"nothing").is_none());
    }
}
