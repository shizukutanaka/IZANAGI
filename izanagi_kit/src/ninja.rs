//! Ninja build file (`build.ninja`) parsing.
//!
//! Directives: `name = value` variables, `rule NAME` + indented
//! vars, `build OUT: RULE INs` (+ implicit `|`, order-only `||`),
//! `default`, `include`, `subninja`, `pool`, `phony`. Comments
//! start with `#`; continuations use `$` at end of line.
//!
//! ```
//! use izanagi_kit::ninja;
//! let d = b"cc = gcc\nrule link\n  command = $cc $in -o $out\nbuild app: link main.o\n";
//! let n = ninja::parse(d).unwrap();
//! assert_eq!(n.rules.len(), 1);
//! assert_eq!(n.edges[0].rule, "link");
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// One `build` edge.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    /// Rule name.
    pub rule: String,
    /// Explicit outputs.
    pub outputs: Vec<String>,
    /// Explicit inputs.
    pub inputs: Vec<String>,
    /// Implicit inputs (`|`).
    pub implicit: Vec<String>,
    /// Order-only inputs (`||`).
    pub order_only: Vec<String>,
    /// 1-based line.
    pub line: usize,
}

/// One `rule` block.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    /// Rule name.
    pub name: String,
    /// Variables inside the rule (`command`, `depfile`…).
    pub vars: BTreeMap<String, String>,
    /// 1-based line.
    pub line: usize,
}

/// A parsed ninja file.
#[derive(Clone, Debug, PartialEq)]
pub struct Ninja {
    /// Global variables.
    pub vars: BTreeMap<String, String>,
    /// `rule` blocks.
    pub rules: Vec<Rule>,
    /// `build` edges.
    pub edges: Vec<Edge>,
    /// `default` target list.
    pub defaults: Vec<String>,
    /// `include`/`subninja` paths.
    pub includes: Vec<String>,
}

/// Parses a ninja file: unescapes `$ ` continuations (`$` at
/// line end joins), collects rules/edges/vars; requires ≥1 rule
/// or edge.
pub fn parse(d: &[u8]) -> Option<Ninja> {
    let text = std::str::from_utf8(d).ok()?;
    // join "$\n" continuations
    let mut logical: Vec<(usize, String)> = Vec::new();
    let mut buf = String::new();
    let mut start = 0usize;
    for (i, raw) in text.split('\n').enumerate() {
        let line = raw.trim_end_matches('\r');
        if buf.is_empty() {
            start = i + 1;
        }
        if let Some(stripped) = line.strip_suffix('$') {
            buf.push_str(stripped);
            continue;
        }
        buf.push_str(line);
        logical.push((start, std::mem::take(&mut buf)));
    }
    if !buf.is_empty() {
        logical.push((start, buf));
    }

    let mut n = Ninja {
        vars: BTreeMap::new(),
        rules: Vec::new(),
        edges: Vec::new(),
        defaults: Vec::new(),
        includes: Vec::new(),
    };
    // context: at top level, or inside a rule collecting indented vars
    let mut cur_rule: Option<usize> = None;
    for (line_no, raw) in logical {
        let line = raw.trim_end();
        if line.is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indented = raw.starts_with(' ') || raw.starts_with('\t');
        if indented {
            // variable inside a rule or edge — attach to last rule
            if let Some(idx) = cur_rule {
                if let Some((k, v)) = line.trim().split_once('=') {
                    let key = k.trim().to_string();
                    let val = v.trim().to_string();
                    if key.is_empty() {
                        return None;
                    }
                    n.rules[idx].vars.insert(key, val);
                    continue;
                }
            }
            continue;
        }
        cur_rule = None;
        let head = line.trim();
        if let Some(rest) = head.strip_prefix("rule ") {
            let name = rest.trim().to_string();
            if name.is_empty() {
                return None;
            }
            n.rules.push(Rule {
                name,
                vars: BTreeMap::new(),
                line: line_no,
            });
            cur_rule = Some(n.rules.len() - 1);
        } else if let Some(rest) = head.strip_prefix("build ") {
            let rest = rest.trim();
            let (outs_part, rest2) = rest.split_once(':')?;
            let outputs = words(outs_part);
            let (rule, ins) = rest2.trim().split_once(' ').unwrap_or((rest2.trim(), ""));
            if rule.is_empty() || outputs.is_empty() {
                return None;
            }
            let (explicit, implicit, order_only) = split_ins(ins);
            n.edges.push(Edge {
                rule: rule.to_string(),
                outputs,
                inputs: explicit,
                implicit,
                order_only,
                line: line_no,
            });
        } else if let Some(rest) = head.strip_prefix("default ") {
            for w in rest.split_whitespace() {
                n.defaults.push(w.to_string());
            }
        } else if let Some(rest) = head.strip_prefix("include ") {
            n.includes.push(rest.trim().to_string());
        } else if let Some(rest) = head.strip_prefix("subninja ") {
            n.includes.push(rest.trim().to_string());
        } else if head.starts_with("pool ") || head.starts_with("phony") {
            // pool defs and phony edge shorthand ignored at top level
        } else if let Some((k, v)) = head.split_once('=') {
            let key = k.trim().to_string();
            if key.is_empty() {
                return None;
            }
            n.vars.insert(key, v.trim().to_string());
        } else {
            return None;
        }
    }
    if n.rules.is_empty() && n.edges.is_empty() {
        return None;
    }
    Some(n)
}

fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.to_string()).collect()
}

fn split_ins(s: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut exp = Vec::new();
    let mut imp = Vec::new();
    let mut oo = Vec::new();
    let mut mode = 0u8;
    for w in s.split_whitespace() {
        match w {
            "|" => mode = 1,
            "||" => mode = 2,
            w => match mode {
                0 => exp.push(w.to_string()),
                1 => imp.push(w.to_string()),
                _ => oo.push(w.to_string()),
            },
        }
    }
    (exp, imp, oo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"# c\ncc = gcc\nrule compile\n  command = $cc -c $in -o $out\nbuild a.o: compile a.c\nbuild app: link a.o | lib.a || order\n  extra = x\ndefault app\nsubninja sub.ninja\n";
        let n = parse(d).unwrap();
        assert_eq!(n.vars.get("cc").unwrap(), "gcc");
        assert_eq!(n.rules.len(), 1);
        assert_eq!(
            n.rules[0].vars.get("command").unwrap(),
            "$cc -c $in -o $out"
        );
        assert_eq!(n.edges.len(), 2);
        assert_eq!(n.edges[1].implicit, vec!["lib.a".to_string()]);
        assert_eq!(n.edges[1].order_only, vec!["order".to_string()]);
        assert_eq!(n.defaults, vec!["app".to_string()]);
        assert_eq!(n.includes, vec!["sub.ninja".to_string()]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"# only\n").is_none());
        assert!(parse(b"build a:\n").is_none()); // no rule
        assert!(parse(b"nonsense line\n").is_none());
    }
}
