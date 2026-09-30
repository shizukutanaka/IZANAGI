//! Lingua Franca (`.lf`) program census.
//!
//! An LF file: `target C|Cpp|Python|TypeScript|Rust;`, `import`,
//! `preamble {= =}`, `reactor name(...) {` blocks with `input`/`output`/
//! `state`/`timer`/`action`/`parameter`/`method`/`local`/`prelude`/
//! `initial`/`reset`/`shutdown`/`startup` members, `main reactor`/
//! `federated reactor`, `reaction(...) {...}` with `->`/`after`/
//! `physical`/`logical`/`STP`/`deadline`/`policy`, `instantiation`,
//! `connection` (`a.out -> b.in`), `mutation`, `logical/physical action`,
//! `STP`, `DEADLINE`, `STAA`, `srs`/`pod`/`instantiation`.
//!
//! ```rust
//! let l = concat!(
//!     "target C;\n",
//!     "reactor A {\n",
//!     "    output out:int;\n",
//!     "    input inp:int;\n",
//!     "    reaction(startup) -> out {=\n",
//!     "    =}\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::lf::Lf::parse(l.as_bytes()).unwrap();
//! assert_eq!(c.reactors, 2);
//! ```

/// Lingua Franca program census.
#[derive(Debug, Clone)]
pub struct Lf {
    /// `target`/`import`/`preamble`/`main reactor`/`federated reactor`/`reactor`/`instantiation` declarations.
    pub reactors: usize,
    /// `input`/`output`/`state`/`timer`/`action`/`parameter`/`method`/`local`/`prelude`/`initial`/`reset`/`shutdown`/`startup` member declarations.
    pub members: usize,
    /// `reaction`/`mutation`/`reaction(...)` blocks.
    pub reactions: usize,
    /// `->`/`after`/`physical`/`logical`/`STP`/`deadline`/`policy`/`STAA`/`srs`/`pod` connection/trigger constructs.
    pub connects: usize,
    /// `{=`/`=}` code fences.
    pub code: usize,
    /// `width`/`bank`/`bank_index`/`runtime`/`authentication`/`tracing`/`logging`/`workers`/`timeout`/`keepalive`/`fast`/`threads`/`coordination`/`scheduler`/`files`/`clock-sync`/`fed-setup`/`cmake-include`/`build`/`external-runtime-path`/`no-compile`/`verify`/`federated`/`single-threaded`/`worker-thread-count`/`multiport`/`enclave`/`serializer`/`interface` options.
    pub options: usize,
}

/// Whether the buffer looks like a Lingua Franca program.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("reactor") && (t.contains("reaction") || t.contains("target"))
        || t.contains("target ")
}

impl Lf {
    /// Parse an LF program into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            reactors: 0,
            members: 0,
            reactions: 0,
            connects: 0,
            code: 0,
            options: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with("//") || s.starts_with("/*") || s == "}" {
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if head == "target"
                || head == "import"
                || head == "preamble"
                || head == "reactor"
                || head == "instantiation"
                || head == "main"
                || head == "federated"
                || head == "realtime"
                || head == "interface"
            {
                c.reactors += 1;
                continue;
            }
            if head == "input"
                || head == "output"
                || head == "state"
                || head == "timer"
                || head == "action"
                || head == "parameter"
                || head == "method"
                || head == "local"
                || head == "prelude"
                || head == "initial"
                || head == "reset"
                || head == "shutdown"
                || head == "startup"
            {
                c.members += 1;
                continue;
            }
            if head.starts_with("reaction") || head == "mutation" || s.starts_with("reaction(") {
                c.reactions += 1;
                continue;
            }
            if s.contains("->")
                || s.contains(" after ")
                || s.contains(" after ")
                || head == "physical"
                || head == "logical"
                || head == "STP"
                || head == "deadline"
                || head == "policy"
                || head == "STAA"
                || s.contains("{=")
                || s.contains("=}")
            {
                c.connects += 1;
                continue;
            }
            if head == "width"
                || head == "bank"
                || head == "bank_index"
                || head == "runtime"
                || head == "authentication"
                || head == "tracing"
                || head == "logging"
                || head == "workers"
                || head == "timeout"
                || head == "keepalive"
                || head == "fast"
                || head == "threads"
                || head == "coordination"
                || head == "scheduler"
                || head == "files"
                || head == "clock-sync"
                || head == "fed-setup"
                || head == "cmake-include"
                || head == "build"
                || head == "external-runtime-path"
                || head == "no-compile"
                || head == "verify"
                || head == "federated"
                || head == "single-threaded"
                || head == "worker-thread-count"
                || head == "multiport"
                || head == "enclave"
                || head == "serializer"
                || head == "platform"
                || head == "threading"
                || head == "cargo-dependencies"
            {
                c.options += 1;
                continue;
            }
            if s.starts_with('{') && s.ends_with('}') && s.contains('=') {
                c.code += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_program() {
        let b = concat!(
            "target C;\n",
            "import util.lf\n",
            "reactor Src {\n",
            "    output out:int;\n",
            "    timer t;\n",
            "    reaction(t) -> out {=\n",
            "    =}\n",
            "}\n",
            "reactor Dst {\n",
            "    input inp:int;\n",
            "    state s:int;\n",
            "}\n",
            "main reactor {\n",
            "    a = new Src();\n",
            "    b = new Dst();\n",
            "    a.out -> b.in after 10 msec;\n",
            "}\n",
        );
        let c = Lf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.reactors, 5);
        assert_eq!(c.members, 4);
        assert_eq!(c.reactions, 1);
        assert_eq!(c.connects, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Lf::parse(b"foo = 1").is_none());
    }
}
