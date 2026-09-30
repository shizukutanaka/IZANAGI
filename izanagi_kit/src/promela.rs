//! Promela / SPIN model (`.pml`) census.
//!
//! A Promela model is `proctype`/`init`/`active proctype`/`chan`/
//! `mtype`/`typedef`/`d_step`/`atomic`/`inline`/`ltl`/`never`/`trace`/
//! `notrace`/`hidden`/`show`/`xr`/`xs` declarations with `do`/`od` loops,
//! `if`/`fi` selection, `::` guards, `->`/`goto`/`break`, `?`/`!` channel
//! ops, `assert`, `printf`, `skip`, `true`/`false` and `c_code`/`c_decl`/
//! `c_state`/`c_expr`/`c_track` embedded C.
//!
//! ```rust
//! let p = concat!(
//!     "mtype = {req, ack};\n",
//!     "chan q = [2] of {mtype};\n",
//!     "active proctype A() {\n",
//!     "    q ! req;\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::promela::Promela::parse(p.as_bytes()).unwrap();
//! assert_eq!(c.proctypes, 1);
//! ```

/// Promela model census.
#[derive(Debug, Clone)]
pub struct Promela {
    /// `proctype`/`init`/`active`/`never`/`trace`/`notrace`/`d_step`/`atomic`/`inline`/`ltl` declarations.
    pub proctypes: usize,
    /// `chan`/`mtype`/`typedef`/`hidden`/`show`/`xr`/`xs` declarations.
    pub types: usize,
    /// `do`/`od`/`if`/`fi`/`::` guard/loop constructs.
    pub flows: usize,
    /// `->`/`goto`/`break`/`skip`/`else`/`unless`/`goto` control.
    pub controls: usize,
    /// `?`/`!` channel send/receive tokens + `assert`/`printf`/`print`.
    pub comms: usize,
    /// `c_code`/`c_decl`/`c_state`/`c_expr`/`c_track`/`c_var` embedded C + `byte`/`short`/`int`/`bool`/`pid`/`unsigned`/`bit` scalar types.
    pub code: usize,
}

/// Whether the buffer looks like a Promela model.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("proctype")
        || (t.contains("mtype") && t.contains("chan "))
        || (t.contains("do") && t.contains("::") && t.contains("od"))
        || t.contains("ltl ")
}

impl Promela {
    /// Parse a Promela model into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            proctypes: 0,
            types: 0,
            flows: 0,
            controls: 0,
            comms: 0,
            code: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with("//") || s.starts_with("/*") {
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if head == "proctype"
                || head == "init"
                || head == "active"
                || head == "never"
                || head == "trace"
                || head == "notrace"
                || head == "d_step"
                || head == "atomic"
                || head == "inline"
                || head == "ltl"
            {
                c.proctypes += 1;
                continue;
            }
            if head == "chan"
                || head == "mtype"
                || head == "typedef"
                || head == "hidden"
                || head == "show"
                || head == "xr"
                || head == "xs"
                || head == "byte"
                || head == "short"
                || head == "int"
                || head == "bool"
                || head == "pid"
                || head == "unsigned"
                || head == "bit"
            {
                c.types += 1;
                continue;
            }
            if head == "do"
                || head == "od"
                || head == "if"
                || head == "fi"
                || head == "od;"
                || head == "fi;"
                || head == "do;"
            {
                c.flows += 1;
                continue;
            }
            if head == "::" || s.starts_with("::") {
                c.flows += 1;
                if s.contains('!') && !s.contains("!=") {
                    c.comms += 1;
                }
                if s.contains('?') && !s.contains("??") {
                    c.comms += 1;
                }
                continue;
            }
            if head == "->"
                || head == "goto"
                || head == "break"
                || head == "skip"
                || head == "else"
                || head == "unless"
            {
                c.controls += 1;
                continue;
            }
            if head == "assert"
                || head == "printf"
                || head == "print"
                || (s.contains('!') && !s.contains("!="))
                || (s.contains('?') && !s.contains("??"))
            {
                c.comms += 1;
                continue;
            }
            if head == "c_code"
                || head == "c_decl"
                || head == "c_state"
                || head == "c_expr"
                || head == "c_track"
                || head == "c_var"
            {
                c.code += 1;
                continue;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_model() {
        let b = concat!(
            "mtype = {req, ack};\n",
            "chan q = [2] of {mtype};\n",
            "byte n;\n",
            "active proctype A() {\n",
            "    do\n",
            "    :: q ! req;\n",
            "    :: timeout -> break;\n",
            "    od;\n",
            "}\n",
            "proctype B() {\n",
            "    if\n",
            "    :: q ? ack -> skip;\n",
            "    fi;\n",
            "}\n",
            "ltl p { []<> (n > 0) }\n",
        );
        let c = Promela::parse(b.as_bytes()).unwrap();
        assert_eq!(c.proctypes, 3);
        assert_eq!(c.types, 3);
        assert_eq!(c.flows, 7);
        assert_eq!(c.controls, 0);
        assert_eq!(c.comms, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Promela::parse(b"foo = 1").is_none());
    }
}
