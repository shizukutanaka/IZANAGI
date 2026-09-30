//! DIMACS formats: `p cnf vars clauses` SAT headers with literal
//! clauses, `p edge`/`p col` graphs (`e`/`a` arcs), `p sp` shortest
//! path, `p max` max-flow. `c` lines are comments.
//!
//! ```
//! use izanagi_kit::dimacs::{detect, parse};
//!
//! let d = b"c comment\np cnf 3 2\n1 -2 0\n3 0\n";
//! assert!(detect(d));
//! let g = parse(d).unwrap();
//! assert_eq!(g.problem.as_deref(), Some("cnf"));
//! assert_eq!(g.declared_vars, Some(3));
//! assert_eq!(g.lines, 2);
//! ```

/// Parsed DIMACS census.
#[derive(Debug, Clone, PartialEq)]
pub struct Dimacs {
    /// Problem keyword after `p ` (`cnf`, `edge`, `col`, `sp`, `max`, `sat`…).
    pub problem: Option<String>,
    /// First size field (variables / nodes).
    pub declared_vars: Option<u32>,
    /// Second size field (clauses / edges).
    pub declared_edges: Option<u32>,
    /// `c` comment lines.
    pub comments: u32,
    /// Data records (clauses for cnf — clauses may wrap lines and end at a
    /// standalone `0` token — `e`/`a`/`n`/`d`/digit lines otherwise).
    pub lines: u32,
    /// `e` edge records (graph problems).
    pub e_edges: u32,
    /// `a` arc records (directed problems).
    pub a_arcs: u32,
    /// Standalone `0` clause terminators (cnf/sat/pbs only).
    pub terminated: u32,
}

/// `true` on `p <kind>` header line.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.lines().any(|l| {
        l.starts_with("p ")
            && l[2..]
                .split_whitespace()
                .next()
                .is_some_and(|k| k.chars().all(|c| c.is_ascii_alphabetic()))
    })
}

/// Census; `None` without the `p` header.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Dimacs> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut d = Dimacs {
        problem: None,
        declared_vars: None,
        declared_edges: None,
        comments: 0,
        lines: 0,
        e_edges: 0,
        a_arcs: 0,
        terminated: 0,
    };
    let mut last_zero = true;
    let mut saw_lit = false;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('c') {
            d.comments += 1;
            continue;
        }
        if let Some(rest) = t.strip_prefix("p ") {
            let mut it = rest.split_whitespace();
            d.problem = it.next().map(str::to_string);
            d.declared_vars = it.next().and_then(|v| v.parse().ok());
            d.declared_edges = it.next().and_then(|v| v.parse().ok());
            continue;
        }
        match t.chars().next() {
            Some('e') => {
                d.e_edges += 1;
                d.lines += 1;
            }
            Some('a') => {
                d.a_arcs += 1;
                d.lines += 1;
            }
            Some('n') | Some('d') => {
                d.lines += 1;
            }
            _ => {
                if t.chars()
                    .all(|c| c.is_ascii_digit() || c == '-' || c == ' ' || c == '+')
                {
                    if matches!(
                        d.problem.as_deref(),
                        Some("cnf") | Some("sat") | Some("pbs")
                    ) {
                        for tok in t.split_whitespace() {
                            if tok == "0" {
                                d.terminated += 1;
                            }
                            last_zero = tok == "0";
                            saw_lit = true;
                        }
                    } else {
                        d.lines += 1;
                    }
                }
            }
        }
    }
    if matches!(
        d.problem.as_deref(),
        Some("cnf") | Some("sat") | Some("pbs")
    ) {
        d.lines = d.terminated + u32::from(saw_lit && !last_zero);
    }
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"c comment\np cnf 3 2\n1 -2 0\n3 0\n";
    const G: &[u8] = b"p edge 4 3\ne 1 2\ne 2 3\ne 3 4\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(G));
        assert!(!detect(b"p ? x"));
    }

    #[test]
    fn parses_cnf() {
        let d = parse(D).unwrap();
        assert_eq!(d.problem.as_deref(), Some("cnf"));
        assert_eq!(d.declared_vars, Some(3));
        assert_eq!(d.declared_edges, Some(2));
        assert_eq!(d.comments, 1);
        assert_eq!(d.lines, 2);
        assert_eq!(d.terminated, 2);
        // a clause may wrap physical lines: count `0` terminators, not lines
        let d = parse(b"p cnf 2 1\n1\n-2 0\n").unwrap();
        assert_eq!(d.lines, 1);
        assert_eq!(d.terminated, 1);
    }

    #[test]
    fn parses_edge() {
        let d = parse(G).unwrap();
        assert_eq!(d.problem.as_deref(), Some("edge"));
        assert_eq!(d.e_edges, 3);
        assert_eq!(d.lines, 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
