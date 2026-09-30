//! SMT-LIB v2 script census (`.smt2`).
//!
//! S-expr command stream: `(set-logic L)`, `(declare-fun n () T)`,
//! `(declare-sort n k)`, `(define-fun f (args) T body)`, `(assert …)`,
//! `(check-sat)`, `(get-model)`, `(push n)`/`(pop n)`, `(set-option …)`,
//! `(exit)`; `;` line comments.
//!
//! ```
//! let d = b"; demo\n(set-logic QF_LIA)\n(declare-fun x () Int)\n\
//! (assert (> x 0))\n(check-sat)\n(get-model)\n";
//! let s = izanagi_kit::smt2::parse(d).unwrap();
//! assert_eq!(s.logic, Some(6)); // "QF_LIA" len
//! assert_eq!(s.asserts, 1);
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Smt2 {
    /// `set-logic` logic name length (chars) — `Some(0)` = seen but empty.
    pub logic: Option<usize>,
    /// `(declare-fun` count.
    pub declare_fun: u32,
    /// `(declare-sort` count.
    pub declare_sort: u32,
    /// `(define-fun`/`define-fun-rec`/`define-sort` count.
    pub defines: u32,
    /// `(assert` count.
    pub asserts: u32,
    /// `(check-sat`/`check-sat-assuming` count.
    pub check_sat: u32,
    /// `(get-model`/`get-value`/`get-proof`/`get-unsat-core`/`get-assignment` count.
    pub get_cmds: u32,
    /// `(push`/`(pop` count.
    pub push_pop: u32,
    /// `(set-option`/`set-info` count.
    pub set_cmds: u32,
    /// `(exit` seen.
    pub exit: bool,
}

fn head_command(t: &str) -> &str {
    let end = t.find([' ', ')', '\t', '(']).unwrap_or(t.len());
    &t[..end]
}

fn logic_name(t: &str) -> Option<usize> {
    let rest = t.strip_prefix("set-logic")?;
    let rest = rest.trim_start();
    let end = rest
        .find(|c: char| c == ')' || c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    Some(end)
}

/// `true` on a `(set-logic|check-sat|assert|declare-` command.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("(set-logic") || t.starts_with("(check-sat") || t.starts_with("(declare-")
    })
}

/// Census; `None` without SMT-LIB commands.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Smt2> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut r = Smt2::default();
    for line in s.lines() {
        let line = line.split(';').next().unwrap_or("");
        for piece in line.split('(').skip(1) {
            let t = piece.trim_start();
            let cmd = head_command(t);
            match cmd {
                "set-logic" => r.logic = Some(logic_name(t).unwrap_or(0)),
                "declare-fun" | "declare-const" => r.declare_fun += 1,
                "declare-sort" => r.declare_sort += 1,
                "define-fun" | "define-fun-rec" | "define-sort" => r.defines += 1,
                "assert" => r.asserts += 1,
                "check-sat" | "check-sat-assuming" => r.check_sat += 1,
                "get-model" | "get-value" | "get-proof" | "get-unsat-core" | "get-assignment"
                | "get-info" => r.get_cmds += 1,
                "push" | "pop" => r.push_pop += 1,
                "set-option" | "set-info" => r.set_cmds += 1,
                "exit" => r.exit = true,
                _ => {}
            }
        }
    }
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"; demo\n(set-logic QF_LIA)\n(set-option :produce-models true)\n\
(declare-fun x () Int)\n(declare-sort S 0)\n(define-fun f ((a Int)) Int a)\n\
(assert (> x 0))\n(assert (< x 9))\n(push 1)\n(check-sat)\n(get-model)\n(pop 1)\n(exit)\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"(print hello)\n"));
    }

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.logic, Some(6));
        assert_eq!(s.declare_fun, 1);
        assert_eq!(s.declare_sort, 1);
        assert_eq!(s.defines, 1);
        assert_eq!(s.asserts, 2);
        assert_eq!(s.check_sat, 1);
        assert_eq!(s.get_cmds, 1);
        assert_eq!(s.push_pop, 2);
        assert_eq!(s.set_cmds, 1);
        assert!(s.exit);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"; only comments\n").is_none());
    }
}
