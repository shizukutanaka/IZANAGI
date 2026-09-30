//! Census of an `ipython_config.py` file.
//!
//! Traitlets config: `c.InteractiveShellApp.*`,
//! `c.TerminalInteractiveShell.*`, `c.TerminalIPythonApp.*`,
//! `c.BaseIPythonApplication.*`, `c.Profile.*`, `c.HistoryManager.*`,
//! `c.PromptManager.*`, `c.StoreMagics.*`, `c.AliasManager.*`,
//! `c.InteractiveShell.*`, `c.Completer.*`, `c.Magics.*`, `c.Terminal`
//! — plus `c.InteractiveShellApp.exec_lines`/`exec_files` and
//! `#` comments. Counts assignments, namespaces, exec lists, imports.
//!
//! ```rust
//! let c = izanagi_kit::ipythonconf::IpythonConf::parse(
//!     b"# ipython\nc.InteractiveShellApp.extensions = ['autoreload']\n\
//!       c.TerminalInteractiveShell.confirm_exit = False\n",
//! ).unwrap();
//! assert_eq!(c.assignments, 2);
//! ```
#![forbid(unsafe_code)]

/// ipython config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpythonConf {
    /// `c.<Ns>.<key> = …` assignments.
    pub assignments: usize,
    /// Distinct `c.<Ns>` namespaces touched.
    pub namespaces: usize,
    /// `exec_lines`/`exec_files`/`extensions` assignments.
    pub exec_lists: usize,
    /// `import`/`from … import` lines.
    pub imports: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Recognised IPython namespaces.
const NS: &[&str] = &[
    "InteractiveShellApp",
    "TerminalInteractiveShell",
    "TerminalIPythonApp",
    "BaseIPythonApplication",
    "Profile",
    "HistoryManager",
    "PromptManager",
    "StoreMagics",
    "AliasManager",
    "InteractiveShell",
    "Completer",
    "Magics",
    "Terminal",
    "IPCompleter",
    "InlineBackend",
    "PlainTextFormatter",
    "LoggingMagics",
];

/// exec-list keys.
const EXEC_KEYS: &[&str] = &["exec_lines", "exec_files", "extensions"];

/// True if `b` looks like ipython_config.py.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    NS.iter().any(|n| t.contains(&format!("c.{n}."))) || t.contains("get_config()")
}

impl IpythonConf {
    /// Parse an ipython config file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            assignments: 0,
            namespaces: 0,
            exec_lists: 0,
            imports: 0,
            comments: 0,
        };
        let mut seen: u32 = 0;
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
            } else if l.starts_with("import ") || l.starts_with("from ") {
                c.imports += 1;
            } else if l.starts_with("c.") && l.contains('=') {
                c.assignments += 1;
                let rest = &l[2..];
                if let Some(dot) = rest.find('.') {
                    let ns = &rest[..dot];
                    if let Some(pos) = NS.iter().position(|n| *n == ns) {
                        let bit = 1u32 << pos;
                        if seen & bit == 0 {
                            seen |= bit;
                            c.namespaces += 1;
                        }
                    }
                }
                let key = rest
                    .rsplit('.')
                    .next()
                    .unwrap_or("")
                    .split('=')
                    .next()
                    .unwrap_or("")
                    .trim();
                if EXEC_KEYS.contains(&key) {
                    c.exec_lists += 1;
                }
            }
        }
        if c.assignments == 0 {
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
            "# ipython\n",
            "c.InteractiveShellApp.extensions = ['autoreload']\n",
            "c.InteractiveShellApp.exec_lines = ['%autoreload 2']\n",
            "c.TerminalInteractiveShell.confirm_exit = False\n",
            "c.HistoryManager.hist_file = ':memory:'\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = IpythonConf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.assignments, 4);
        assert_eq!(c.namespaces, 3);
        assert_eq!(c.exec_lists, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[section]\nkey=1\n"));
        assert!(IpythonConf::parse(b"# none\n").is_none());
    }
}
