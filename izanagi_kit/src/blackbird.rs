//! Xanadu Blackbird photonic quantum circuit parser.
//!
//! Header lines `name`/`version`/`target`/`type`, then one operation per
//! line `GateName(args) | q[i, j, ...]`.
//!
//! ```
//! use izanagi_kit::blackbird::Blackbird;
//! let src = b"name test\nversion 1\x2e0\ntarget gaussian\n\nSgate(1) | q[0]\nBSgate(0, 0) | q[0, 1]\nMeasure | q[0]\n";
//! assert!(izanagi_kit::blackbird::detect(src));
//! let q = Blackbird::parse(src).unwrap();
//! assert_eq!(q.target, "gaussian");
//! assert_eq!(q.operations, 3);
//! assert_eq!(q.modes_used, 2);
//! ```

/// Parsed census of a Blackbird script.
#[derive(Debug, Clone)]
pub struct Blackbird {
    /// `name` header value.
    pub name: String,
    /// `version` header value.
    pub version: String,
    /// `target` header value (e.g. `gaussian`, `fock`, `tdm`).
    pub target: String,
    /// `type` header value (`tdm` for time-domain multiplexing).
    pub type_str: String,
    /// Operation lines `Gate(...) | q[...]`.
    pub operations: usize,
    /// Distinct `q[...]` mode indices referenced.
    pub modes_used: usize,
    /// `Measure`/`MeasureFock`/`MeasureThreshold`/`MeasureHomodyne`/`MeasureHeterodyne` ops.
    pub measurements: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Returns `true` when `b` looks like a Blackbird script.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut head = 0usize;
    for (i, l) in t.lines().enumerate() {
        if i > 8 {
            break;
        }
        let l = l.trim();
        if l.starts_with("name ") || l.starts_with("version ") || l.starts_with("target ") {
            head += 1;
        }
    }
    head >= 2 && t.contains("| q[")
}

impl Blackbird {
    /// Parses a Blackbird script; `None` without header + operation pipe.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let mut bb = Self {
            name: String::new(),
            version: String::new(),
            target: String::new(),
            type_str: String::new(),
            operations: 0,
            modes_used: 0,
            measurements: 0,
            comments: 0,
        };
        let mut modes: Vec<u32> = Vec::new();
        let mut ops = 0usize;
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                bb.comments += 1;
                continue;
            }
            if let Some(rest) = l.strip_prefix("name ") {
                bb.name = rest.trim().to_string();
                continue;
            }
            if let Some(rest) = l.strip_prefix("version ") {
                bb.version = rest.trim().to_string();
                continue;
            }
            if let Some(rest) = l.strip_prefix("target ") {
                bb.target = rest.trim().to_string();
                continue;
            }
            if let Some(rest) = l.strip_prefix("type ") {
                bb.type_str = rest.trim().to_string();
                continue;
            }
            if let Some(pipe) = l.find('|') {
                let gate = l[..pipe].split(['(', ' ']).next().unwrap_or("");
                ops += 1;
                if gate.starts_with("Measure") {
                    bb.measurements += 1;
                }
                let modes_part = &l[pipe + 1..];
                for chunk in modes_part.split(&['[', ']', ','][..]) {
                    let c = chunk.trim();
                    if c.is_empty() || c == "q" {
                        continue;
                    }
                    if let Ok(v) = c.parse::<u32>() {
                        if !modes.contains(&v) {
                            modes.push(v);
                        }
                    }
                }
            }
        }
        bb.operations = ops;
        bb.modes_used = modes.len();
        (ops > 0).then_some(bb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_script() {
        assert!(detect(
            b"name x\nversion 1\x2e0\ntarget tdm\n\nRgate(0) | q[0]\n"
        ));
        assert!(!detect(b"name x\nversion 1\x2e0\n"));
    }

    #[test]
    fn counts_ops_modes() {
        let src = b"name n\nversion 1\x2e0\ntarget fock\n\nDgate(1) | q[0]\nBSgate(0,0) | q[0,2]\nMeasureFock | q[0, 2]\n";
        let q = Blackbird::parse(src).unwrap();
        assert_eq!(q.operations, 3);
        assert_eq!(q.modes_used, 2);
        assert_eq!(q.measurements, 1);
    }
}
