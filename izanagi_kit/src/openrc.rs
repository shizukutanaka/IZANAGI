//! Census of an OpenRC runscript (`/etc/init.d/*`).
//!
//! `#!/sbin/openrc-run` shebang, `depend()` dependency function whose
//! `need`/`use`/`want`/`before`/`after`/`provide`/`keyword` clauses
//! declare ordering, `start()`/`stop()`/`reload()` function bodies and
//! `command=`/`command_args=`/`pidfile=`/`description=` variables.
//! Counts dependency clauses by kind plus functions and variables.
//!
//! ```rust
//! let c = izanagi_kit::openrc::Openrc::parse(
//!     b"#!/sbin/openrc-run\ndepend() {\n  need net\n  use dns logger\n}\n\
//!       start() {\n  ebegin ok\n}\n",
//! ).unwrap();
//! assert_eq!(c.dependencies, 2);
//! ```
#![forbid(unsafe_code)]

/// OpenRC runscript census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Openrc {
    /// `name() {` function definitions.
    pub functions: usize,
    /// `need`/`use`/`want`/`before`/`after`/`provide`/`keyword` clauses.
    pub dependencies: usize,
    /// `command`/`command_args`/`pidfile`/`description*`/`start_stop_daemon_args` assignments.
    pub variables: usize,
    /// `ebegin`/`eend`/`eerror`/`ewarn`/`einfo`/`veinfo` calls.
    pub ecalls: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Dependency clause heads inside `depend()`.
const DEP_HEADS: &[&str] = &[
    "need ",
    "use ",
    "want ",
    "before ",
    "after ",
    "provide ",
    "keyword ",
    "require_any ",
];

/// Variable names counted.
const VARS: &[&str] = &[
    "command",
    "command_args",
    "command_user",
    "pidfile",
    "description",
    "start_stop_daemon_args",
    "retry",
    "supervisor",
    "output_log",
    "error_log",
];

/// True if `b` looks like an OpenRC runscript.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("openrc-run")
        || (t.contains("depend()") && DEP_HEADS.iter().any(|h| t.contains(h)))
        || (t.contains("depend()") && (t.contains("start()") || t.contains("stop()")))
}

impl Openrc {
    /// Parse an OpenRC runscript into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            functions: 0,
            dependencies: 0,
            variables: 0,
            ecalls: 0,
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
            if DEP_HEADS.iter().any(|h| l.starts_with(h)) {
                c.dependencies += 1;
            } else if l.contains("()") && l.ends_with('{') {
                c.functions += 1;
            } else if l.starts_with("ebegin")
                || l.starts_with("eend")
                || l.starts_with("eerror")
                || l.starts_with("ewarn")
                || l.starts_with("einfo")
                || l.starts_with("veinfo")
            {
                c.ecalls += 1;
            } else if l.contains('=') {
                let k = l
                    .split('=')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_start_matches(['\'', '"']);
                if VARS
                    .iter()
                    .any(|v| k == *v || k.starts_with("description_"))
                {
                    c.variables += 1;
                }
            }
        }
        if c.functions == 0 && c.dependencies == 0 {
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
            "#!/sbin/openrc-run\n",
            "description=\"my svc\"\n",
            "command=/usr/bin/mysvc\n",
            "depend() {\n",
            "  need net\n",
            "  use dns logger\n",
            "  after firewall\n",
            "}\n",
            "start() {\n",
            "  ebegin starting\n",
            "  $command $command_args\n",
            "  eend $?\n",
            "}\n",
            "stop() {\n",
            "  ebegin stopping\n",
            "}\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Openrc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.functions, 3);
        assert_eq!(c.dependencies, 3);
        assert_eq!(c.variables, 2);
        assert_eq!(c.ecalls, 3);
        assert_eq!(c.comments, 0);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"#!/bin/sh\necho hi\n"));
        assert!(Openrc::parse(b"# comment\n").is_none());
    }
}
