//! IBM Qobj (quantum object) JSON parser.
//!
//! Extracts the envelope (`qobj_id`, `type`, `schema_version`) and
//! counts `experiments`, instruction `name` entries, `shots` and
//! `memory_slots` from `config`.
//!
//! ```
//! use izanagi_kit::qobj::Qobj;
//! let src = br#"{"qobj_id":"abc","type":"QASM","schema_version":"1\x2e3\x2e0","header":{},"config":{"shots":1024,"memory_slots":2},"experiments":[{"instructions":[{"name":"h","qubits":[0]},{"name":"cx","qubits":[0,1]}]}]}"#;
//! assert!(izanagi_kit::qobj::detect(src));
//! let q = Qobj::parse(src).unwrap();
//! assert_eq!(q.qobj_id, "abc");
//! assert_eq!(q.type_str, "QASM");
//! assert_eq!(q.experiments, 1);
//! assert_eq!(q.instructions, 2);
//! assert_eq!(q.shots, 1024);
//! ```

/// Parsed census of an IBM Qobj document.
#[derive(Debug, Clone)]
pub struct Qobj {
    /// `"qobj_id"` value.
    pub qobj_id: String,
    /// `"type"` value (`QASM`, `PULSE`, ...).
    pub type_str: String,
    /// `"schema_version"` value.
    pub schema_version: String,
    /// Number of experiment objects inside `"experiments"`.
    pub experiments: usize,
    /// Total `"name":` instruction entries across experiments.
    pub instructions: usize,
    /// `config.shots`.
    pub shots: u32,
    /// `config.memory_slots`.
    pub memory_slots: u32,
}

fn jstr<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{}\":", key);
    let i = t.find(&pat)?;
    let mut s = t[i + pat.len()..].trim_start();
    if !s.starts_with('"') {
        return None;
    }
    s = &s[1..];
    let end = s.find('"')?;
    Some(&s[..end])
}

fn jint(t: &str, key: &str) -> u32 {
    let pat = format!("\"{}\":", key);
    let Some(i) = t.find(&pat) else { return 0 };
    let s = t[i + pat.len()..].trim_start();
    let digits: usize = s.bytes().take_while(u8::is_ascii_digit).count();
    s[..digits].parse().unwrap_or(0)
}
fn jkey(t: &str, key: &str) -> bool {
    // `"key"` が値位置ではなくキー位置(直後が `:`)にあるかを確認。
    let pat = format!("\"{key}\"");
    let mut rest = t;
    while let Some(i) = rest.find(&pat) {
        rest = &rest[i + pat.len()..];
        if rest.trim_start().starts_with(':') {
            return true;
        }
    }
    false
}

/// Returns `true` when `b` looks like a Qobj JSON document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    jkey(t, "qobj_id") && jkey(t, "experiments")
}

impl Qobj {
    /// Parses a Qobj document; `None` when `qobj_id` or `experiments` is absent.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let qobj_id = jstr(t, "qobj_id")?.to_string();
        let type_str = jstr(t, "type").unwrap_or("").to_string();
        let schema_version = jstr(t, "schema_version").unwrap_or("").to_string();
        let exp_i = t.find("\"experiments\"")?;
        // count experiment objects: each `{` inside the experiments array
        // begins an experiment — count `"instructions"` arrays instead.
        let instructions = t.matches("\"name\":").count();
        let experiments = t.matches("\"instructions\"").count();
        let _ = exp_i;
        let shots = jint(t, "shots");
        let memory_slots = jint(t, "memory_slots");
        Some(Self {
            qobj_id,
            type_str,
            schema_version,
            experiments,
            instructions,
            shots,
            memory_slots,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(
            b"{\"data\": \"qobj_id\", \"x\": \"experiments\"}\n"
        ));
    }

    #[test]
    fn detects_envelope() {
        assert!(detect(br#"{"qobj_id":"x","experiments":[]}"#));
        assert!(!detect(b"{\"a\":1}"));
    }

    #[test]
    fn counts_instructions() {
        let src = br#"{"qobj_id":"i","type":"PULSE","schema_version":"1\x2e0\x2e0","experiments":[{"instructions":[{"name":"pv","qubits":[0]},{"name":"fc","qubits":[0]},{"name":"acquire","qubits":[0]}]},{"instructions":[{"name":"u1","qubits":[0]}]}]}"#;
        let q = Qobj::parse(src).unwrap();
        assert_eq!(q.type_str, "PULSE");
        assert_eq!(q.experiments, 2);
        assert_eq!(q.instructions, 4);
    }
}
