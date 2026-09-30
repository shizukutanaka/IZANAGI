//! AIGER (And-Inverter Graph) file census.
//!
//! Header `aag M I L O A` (ASCII) or `aig M I L O A` (binary) —
//! M = max variable index, I = inputs, L = latches, O = outputs,
//! A = AND gates (optional trailing `B C J F` for bad/constraint/
//! justice/fairness). ASCII `aag` then lists input/latch/output/and lines;
//! binary `aig` encodes gates as delta bytes.
//!
//! ```
//! let d = b"aag 3 1 0 1 1\n2\n6\n6 2 4\n";
//! let a = izanagi_kit::aiger::parse(d).unwrap();
//! assert_eq!(a.max_var, 3);
//! assert_eq!(a.ands_declared, 1);
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Aiger {
    /// ASCII (`aag`) vs binary (`aig`).
    pub ascii: bool,
    /// M — maximum variable index.
    pub max_var: u32,
    /// I — declared inputs.
    pub inputs: u32,
    /// L — declared latches.
    pub latches: u32,
    /// O — declared outputs.
    pub outputs: u32,
    /// A — declared AND gates.
    pub ands_declared: u32,
    /// B — declared bad-state properties.
    pub bad: u32,
    /// C — declared invariant constraints.
    pub constraints: u32,
    /// J — declared justice properties.
    pub justice: u32,
    /// F — declared fairness constraints.
    pub fairness: u32,
    /// `aag` input lines actually present.
    pub input_lines: u32,
    /// `aag` AND-gate lines actually present.
    pub and_lines: u32,
    /// `c` comment marker seen.
    pub comment: bool,
}

fn head_nums(line: &str) -> Option<(&str, [u32; 10], usize)> {
    let mut nums = [0u32; 10];
    let mut n = 0;
    let mut it = line.split_ascii_whitespace();
    let magic = it.next()?;
    for t in it {
        if n == 10 {
            return None;
        }
        nums[n] = t.parse().ok()?;
        n += 1;
    }
    Some((magic, nums, n))
}

/// `true` on the `aag`/`aig` header line.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return b.len() >= 4 && (&b[..4] == b"aig " || &b[..4] == b"aig\n"),
    };
    s.lines()
        .next()
        .is_some_and(|l| l.starts_with("aag ") || l.starts_with("aig "))
}

/// Census; `None` on malformed header.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Aiger> {
    if !detect(b) {
        return None;
    }
    let first = b.split(|&c| c == b'\n').next().unwrap_or(b);
    let head = core::str::from_utf8(first).ok()?;
    let (magic, nums, n) = head_nums(head)?;
    if n < 5 || !(magic == "aag" || magic == "aig") {
        return None;
    }
    let mut a = Aiger {
        ascii: magic == "aag",
        max_var: nums[0],
        inputs: nums[1],
        latches: nums[2],
        outputs: nums[3],
        ands_declared: nums[4],
        bad: nums[5],
        constraints: nums[6],
        justice: nums[7],
        fairness: nums[8],
        ..Aiger::default()
    };
    if a.ascii {
        if let Ok(s) = core::str::from_utf8(b) {
            let mut lines = s.lines().skip(1).peekable();
            for _ in 0..a.inputs {
                match lines.next() {
                    Some(l) if !l.is_empty() => a.input_lines += 1,
                    _ => break,
                }
            }
            for _ in 0..a.latches {
                if lines.next().is_none() {
                    break;
                }
            }
            for _ in 0..a.outputs {
                if lines.next().is_none() {
                    break;
                }
            }
            for _ in 0..(a.bad + a.constraints + a.justice + a.fairness) {
                if lines.next().is_none() {
                    break;
                }
            }
            for l in lines {
                let t = l.trim();
                if t.len() == 1 && t.starts_with('c') {
                    a.comment = true;
                    break;
                }
                a.and_lines += 1;
            }
        }
    }
    Some(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"aag 3 1 1 1 1\n2\n4 0\n6\n6 2 4\nc\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"aig 3 1 0 1 1\n"));
        assert!(!detect(b"aagx 3 1 0 1 1\n"));
    }

    #[test]
    fn parses_ascii() {
        let a = parse(D).unwrap();
        assert!(a.ascii);
        assert_eq!(a.max_var, 3);
        assert_eq!(a.inputs, 1);
        assert_eq!(a.latches, 1);
        assert_eq!(a.outputs, 1);
        assert_eq!(a.ands_declared, 1);
        assert_eq!(a.input_lines, 1);
        assert_eq!(a.and_lines, 1);
        assert!(a.comment);
    }

    #[test]
    fn binary_header() {
        let a = parse(b"aig 9 2 1 3 4 1 0 2 1\n").unwrap();
        assert!(!a.ascii);
        assert_eq!(a.bad, 1);
        assert_eq!(a.justice, 2);
        assert_eq!(a.fairness, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"aag x y z\n").is_none());
    }
}
