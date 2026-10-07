//! Census of an s6-rc service definition (execline script).
//!
//! An s6-rc `run`/`up`/`finish`/`down` file is an execline script:
//! `#!/command/execlineb -S0` shebang, `if { cond }`/`foreground { … }`
//!/`background { … }`/`fdredir`/`redirfd`/`pipeline`/`ifelse`/`backtick`
//!/`importas`/`export`/`define`/`exec`/`s6-svc`/`s6-rc` builtins.
//! Counts block heads, invocations of supervision helpers and comments.
//!
//! ```rust
//! let c = izanagi_kit::s6rc::S6rc::parse(
//!     b"#!/command/execlineb -S0\nforeground { s6-echo up }\n/usr/bin/daemon --foreground\n",
//! ).unwrap();
//! assert_eq!(c.blocks, 1);
//! ```
#![forbid(unsafe_code)]

/// s6-rc/execline script census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct S6rc {
    /// Control-block heads (`if`/`foreground`/`background`/`ifelse`/`pipeline`/`fdredir`/`redirfd`).
    pub blocks: usize,
    /// `backtick`/`importas`/`export`/`define`/`multidefine`/`elgetpositionals` variable ops.
    pub variable_ops: usize,
    /// `s6-*` helper invocations (s6-svc/s6-rc/s6-echo/s6-envdir/…).
    pub s6_helpers: usize,
    /// `exec`/`cd`/`exit`/`wait` plain actions.
    pub actions: usize,
    /// `#` comment lines (shebang excluded).
    pub comments: usize,
}

/// Block-head builtin names.
const BLOCKS: &[&str] = &[
    "if",
    "foreground",
    "background",
    "ifelse",
    "pipeline",
    "fdredir",
    "redirfd",
    "elglob",
    "forstdin",
    "forx",
];

/// Variable-op builtin names.
const VAR_OPS: &[&str] = &[
    "backtick",
    "importas",
    "export",
    "define",
    "multidefine",
    "elgetpositionals",
    "unexport",
];

/// Action builtin names.
const ACTIONS: &[&str] = &["exec", "cd", "exit", "wait", "execve", "loopwhilex"];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// True if `b` looks like an s6-rc/execline script.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.contains("execlineb")
        || (t.contains("{") && (t.contains("s6-svc") || t.contains("s6-rc")))
        || (BLOCKS
            .iter()
            .filter(|h| {
                t.lines()
                    .any(|l| l.trim_start().starts_with(*h) && l.contains('{'))
            })
            .count()
            >= 1
            && t.contains("s6-"))
}

impl S6rc {
    /// Parse an s6-rc script into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            blocks: 0,
            variable_ops: 0,
            s6_helpers: 0,
            actions: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') && !l.starts_with("#!") {
                c.comments += 1;
                continue;
            }
            let head = l.split_whitespace().next().unwrap_or("");
            if BLOCKS.contains(&head) && l.contains('{') {
                c.blocks += 1;
            } else if VAR_OPS.contains(&head) {
                c.variable_ops += 1;
            } else if head.starts_with("s6-") {
                c.s6_helpers += 1;
            } else if ACTIONS.contains(&head) {
                c.actions += 1;
            }
        }
        if c.blocks == 0 && c.s6_helpers == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "#!/command/execlineb -S0\n",
            "# one-shot\n",
            "foreground { s6-echo starting }\n",
            "foreground { importas -u x X }\n",
            "s6-envdir /etc/s6/env\n",
            "/usr/bin/daemon --foreground\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = S6rc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.blocks, 2);
        assert_eq!(c.s6_helpers, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"#!/bin/sh\necho hi\n"));
        assert!(S6rc::parse(b"echo hi\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
