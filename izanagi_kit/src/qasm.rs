//! OpenQASM (2.x / 3.x) quantum assembly text parser.
//!
//! Counts statements: `qreg`/`creg`/`qubit`/`bit` declarations, `gate`
//! and `opaque` definitions, `include`, `measure`, `barrier`, `reset`,
//! `if(...)` conditionals and remaining gate applications.
//!
//! ```
//! use izanagi_kit::qasm::Qasm;
//! let src = b"OPENQASM 2\x2e0;\ninclude \"qelib1.inc\";\nqreg q[2];\ncreg c[2];\nh q[0];\ncx q[0],q[1];\nmeasure q -> c;\n";
//! assert!(izanagi_kit::qasm::detect(src));
//! let q = Qasm::parse(src).unwrap();
//! assert_eq!(q.version_major, 2);
//! assert_eq!(q.qregs, 1);
//! assert_eq!(q.qubits_declared, 2);
//! assert_eq!(q.measurements, 1);
//! assert_eq!(q.gate_applications, 2);
//! ```

/// Parsed census of an OpenQASM source.
#[derive(Debug, Clone)]
pub struct Qasm {
    /// `OPENQASM <major>.<minor>` major version.
    pub version_major: u32,
    /// `OPENQASM <major>.<minor>` minor version.
    pub version_minor: u32,
    /// `qreg name[n];` statements (v2).
    pub qregs: usize,
    /// `creg name[n];` statements (v2).
    pub cregs: usize,
    /// `qubit[n] name;` / `qubit name;` declarations (v3).
    pub qubit_decls: usize,
    /// `bit[n] name;` / `bit name;` declarations (v3).
    pub bit_decls: usize,
    /// Total declared qubit slots summed over `qreg`/`qubit[...]`.
    pub qubits_declared: u32,
    /// Total declared classical bits summed over `creg`/`bit[...]`.
    pub bits_declared: u32,
    /// `gate name(params) qargs { body }` definitions.
    pub gates_defined: usize,
    /// `opaque name qargs;` declarations.
    pub opaque: usize,
    /// `include "file";` statements.
    pub includes: usize,
    /// `measure` statements.
    pub measurements: usize,
    /// `barrier` statements.
    pub barriers: usize,
    /// `reset` statements.
    pub resets: usize,
    /// `if(...)` conditional statements.
    pub ifs: usize,
    /// All other semicolon/brace statements counted as gate applications.
    pub gate_applications: usize,
    /// `//` comment lines.
    pub comments: usize,
}

/// Returns `true` when `b` looks like an OpenQASM source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.trim_start().starts_with("OPENQASM")
}

impl Qasm {
    /// Parses an OpenQASM source; `None` when no version header is present.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let tt = t.trim_start();
        let rest = tt.strip_prefix("OPENQASM")?;
        let mut q = Self {
            version_major: 0,
            version_minor: 0,
            qregs: 0,
            cregs: 0,
            qubit_decls: 0,
            bit_decls: 0,
            qubits_declared: 0,
            bits_declared: 0,
            gates_defined: 0,
            opaque: 0,
            includes: 0,
            measurements: 0,
            barriers: 0,
            resets: 0,
            ifs: 0,
            gate_applications: 0,
            comments: 0,
        };
        let mut chars = rest.trim_start();
        let mut major = String::new();
        while let Some(c) = chars.chars().next() {
            if c.is_ascii_digit() {
                major.push(c);
                chars = &chars[1..];
            } else {
                break;
            }
        }
        q.version_major = major.parse().unwrap_or(0);
        if chars.starts_with('.') {
            chars = &chars[1..];
            let mut minor = String::new();
            while let Some(c) = chars.chars().next() {
                if c.is_ascii_digit() {
                    minor.push(c);
                    chars = &chars[1..];
                } else {
                    break;
                }
            }
            q.version_minor = minor.parse().unwrap_or(0);
        }
        // split into statements on ';', keep brace blocks on one statement
        let mut stmts: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut depth = 0i32;
        for line in t.lines() {
            let line = match line.find("//") {
                Some(i) => {
                    q.comments += 1;
                    &line[..i]
                }
                None => line,
            };
            for ch in line.chars() {
                match ch {
                    '{' => {
                        depth += 1;
                        cur.push(ch);
                    }
                    '}' => {
                        depth -= 1;
                        cur.push(ch);
                        if depth <= 0 {
                            depth = 0;
                            stmts.push(core::mem::take(&mut cur));
                        }
                    }
                    ';' if depth == 0 => {
                        stmts.push(core::mem::take(&mut cur));
                    }
                    _ => cur.push(ch),
                }
            }
        }
        if !cur.trim().is_empty() {
            stmts.push(cur);
        }
        for stmt in &stmts {
            let s = stmt.trim();
            let first = s.split_whitespace().next().unwrap_or("");
            match first {
                "OPENQASM" => {}
                "include" => q.includes += 1,
                "qreg" => {
                    q.qregs += 1;
                    q.qubits_declared += bracket_num(s);
                }
                "creg" => {
                    q.cregs += 1;
                    q.bits_declared += bracket_num(s);
                }
                _ if first == "qubit" || first.starts_with("qubit[") => {
                    q.qubit_decls += 1;
                    q.qubits_declared += bracket_num(s).max(1);
                }
                _ if first == "bit" || first.starts_with("bit[") => {
                    q.bit_decls += 1;
                    q.bits_declared += bracket_num(s).max(1);
                }
                "gate" => q.gates_defined += 1,
                "opaque" => q.opaque += 1,
                "measure" => q.measurements += 1,
                "barrier" => q.barriers += 1,
                "reset" => q.resets += 1,
                "if" => q.ifs += 1,
                _ => {
                    if !s.is_empty() {
                        q.gate_applications += 1;
                    }
                }
            }
        }
        Some(q)
    }
}

fn bracket_num(s: &str) -> u32 {
    match (s.find('['), s.find(']')) {
        (Some(a), Some(bz)) if bz > a => s[a + 1..bz].trim().parse().unwrap_or(1),
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_qasm2() {
        let src = b"OPENQASM 2\x2e0;\nqreg q[1];\n";
        assert!(detect(src));
        let q = Qasm::parse(src).unwrap();
        assert_eq!(q.version_major, 2);
        assert_eq!(q.qregs, 1);
    }

    #[test]
    fn counts_v3_decls() {
        let src = b"OPENQASM 3\x2e0;\nqubit[4] q;\nbit[4] c;\ndefcal x $0 { }\n";
        let q = Qasm::parse(src).unwrap();
        assert_eq!(q.version_major, 3);
        assert_eq!(q.qubit_decls, 1);
        assert_eq!(q.qubits_declared, 4);
        assert_eq!(q.bits_declared, 4);
    }

    #[test]
    fn rejects_plain() {
        assert!(!detect(b"not quantum"));
        assert!(Qasm::parse(b"nonsense").is_none());
    }
}
