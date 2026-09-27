//! GNU Makefile structural parsing.
//!
//! Lines: `NAME = value` / `:=` / `?=` / `+=` assignments,
//! `target: prerequisites` rules, tab-indented recipe lines,
//! `include`, `.PHONY`, comments `#`, `\`-continuations.
//!
//! ```
//! use izanagi_kit::makefile;
//! let d = b"CC = gcc\nall: a.o b.o\n\t$(CC) -o $@ $^\n.PHONY: clean\nclean:\n\trm -f x\n";
//! let m = makefile::parse(d).unwrap();
//! assert_eq!(m.rules.len(), 2);
//! assert_eq!(m.rules[0].recipes.len(), 1);
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// Assignment flavour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Assign {
    /// `=` — recursive.
    Recursive,
    /// `:=` — simple/immediate.
    Simple,
    /// `?=` — conditional.
    Conditional,
    /// `+=` — append.
    Append,
}

/// One rule (`target: deps` + recipe lines).
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    /// Targets (usually one).
    pub targets: Vec<String>,
    /// Prerequisites.
    pub deps: Vec<String>,
    /// Recipe lines (tab-indented text).
    pub recipes: Vec<String>,
    /// 1-based line of the rule header.
    pub line: usize,
}

/// A parsed Makefile.
#[derive(Clone, Debug, PartialEq)]
pub struct Makefile {
    /// Variable assignments, last wins per name.
    pub vars: BTreeMap<String, (String, Assign)>,
    /// Rules in file order.
    pub rules: Vec<Rule>,
    /// `.PHONY` targets seen.
    pub phony: Vec<String>,
    /// `include` targets.
    pub includes: Vec<String>,
}

/// Parses a Makefile: joins `\` continuations, then classifies
/// lines. Requires ≥1 rule or assignment.
pub fn parse(d: &[u8]) -> Option<Makefile> {
    let text = std::str::from_utf8(d).ok()?;
    // join backslash continuations (recipe lines keep their tab)
    let mut logical: Vec<(usize, String)> = Vec::new();
    let mut buf = String::new();
    let mut start = 0usize;
    for (i, raw) in text.split('\n').enumerate() {
        let line = raw.trim_end_matches('\r');
        if buf.is_empty() {
            start = i + 1;
        }
        if let Some(stripped) = line.strip_suffix('\\') {
            buf.push_str(stripped);
            continue;
        }
        buf.push_str(line);
        logical.push((start, std::mem::take(&mut buf)));
    }
    if !buf.is_empty() {
        logical.push((start, buf));
    }

    let mut m = Makefile {
        vars: BTreeMap::new(),
        rules: Vec::new(),
        phony: Vec::new(),
        includes: Vec::new(),
    };
    let mut cur_rule: Option<usize> = None;
    for (line_no, raw) in logical {
        if raw.starts_with('\t') {
            if let Some(idx) = cur_rule {
                m.rules[idx].recipes.push(raw.trim_end().to_string());
            }
            continue;
        }
        cur_rule = None;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("include ") {
            m.includes.push(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix(".PHONY:") {
            for w in rest.split_whitespace() {
                m.phony.push(w.to_string());
            }
            continue;
        }
        // assignment?
        let assign = if let Some(p) = line.find(":=") {
            Some((Assign::Simple, p, 2))
        } else if let Some(p) = line.find("?=") {
            Some((Assign::Conditional, p, 2))
        } else if let Some(p) = line.find("+=") {
            Some((Assign::Append, p, 2))
        } else if let Some(eq) = line.find('=') {
            // a `:` before the `=` means this is a rule, not an assignment
            match line.find(':') {
                Some(c) if c < eq => None,
                _ => Some((Assign::Recursive, eq, 1)),
            }
        } else {
            None
        };
        if let Some((kind, pos, oplen)) = assign {
            let name = line[..pos].trim();
            if name.is_empty() {
                return None;
            }
            m.vars.insert(
                name.to_string(),
                (line[pos + oplen..].trim().to_string(), kind),
            );
            continue;
        }
        // rule?
        if let Some((l, r)) = line.split_once(':') {
            let targets: Vec<String> = l.split_whitespace().map(|w| w.to_string()).collect();
            let deps: Vec<String> = r.split_whitespace().map(|w| w.to_string()).collect();
            if targets.is_empty() {
                return None;
            }
            m.rules.push(Rule {
                targets,
                deps,
                recipes: Vec::new(),
                line: line_no,
            });
            cur_rule = Some(m.rules.len() - 1);
            continue;
        }
        return None;
    }
    if m.rules.is_empty() && m.vars.is_empty() {
        return None;
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"CC := gcc\nSRCS = a.c \\\n     b.c\n.PHONY: clean\nall: a.o b.o\n\t$(CC) -o $@ $^\n\techo done\nclean:\n\trm -f a.o\n";
        let m = parse(d).unwrap();
        assert_eq!(m.vars.get("CC").unwrap().1, Assign::Simple);
        assert_eq!(m.rules.len(), 2);
        assert_eq!(m.rules[0].targets, vec!["all".to_string()]);
        assert_eq!(m.rules[0].deps, vec!["a.o".to_string(), "b.o".to_string()]);
        assert_eq!(m.rules[0].recipes.len(), 2);
        assert_eq!(m.phony, vec!["clean".to_string()]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"# only comments\n").is_none());
        assert!(parse(b"   \n").is_none());
        assert!(parse(b"just some words\n").is_none());
    }
}
