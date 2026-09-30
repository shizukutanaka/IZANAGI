//! Census of a `jupyter_notebook_config.py` file.
//!
//! Traitlets config in Python syntax: `c.NotebookApp.<key> = value`,
//! `c.ServerApp.<key>`, `c.LabApp.<key>`, `c.ExtensionApp.<key>`,
//! `c.MultiKernelManager.<key>`, `c.Terminal.<key>`,
//! `c.KernelSpecManager.<key>`, `c.FileContentsManager.<key>`,
//! `c.ContentsManager.<key>` — plus `import`/`from` lines and
//! `#` comments. Counts assignments by namespace and comments.
//!
//! ```rust
//! let c = izanagi_kit::jupyterconf::JupyterConf::parse(
//!     b"# cfg\nc.NotebookApp.ip = '*'\nc.ServerApp.port = 8888\nc.ServerApp.open_browser = False\n",
//! ).unwrap();
//! assert_eq!(c.assignments, 3);
//! assert_eq!(c.namespaces, 2);
//! ```
#![forbid(unsafe_code)]

/// jupyter config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JupyterConf {
    /// `c.<Ns>.<key> = …` assignments.
    pub assignments: usize,
    /// Distinct `c.<Ns>` namespaces touched.
    pub namespaces: usize,
    /// `import`/`from … import` lines.
    pub imports: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Recognised namespaces after `c.`.
const NS: &[&str] = &[
    "NotebookApp",
    "ServerApp",
    "LabApp",
    "ExtensionApp",
    "MultiKernelManager",
    "Terminal",
    "KernelSpecManager",
    "FileContentsManager",
    "ContentsManager",
    "Session",
    "KernelManager",
    "MappingKernelManager",
    "GatewayKernelManager",
    "AsyncMultiKernelManager",
    "JupyterApp",
    "Application",
    "Config",
];

/// True if `b` looks like a jupyter config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    NS.iter().any(|n| t.contains(&format!("c.{n}."))) || t.contains("get_config()")
}

impl JupyterConf {
    /// Parse a jupyter config file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            assignments: 0,
            namespaces: 0,
            imports: 0,
            comments: 0,
        };
        let mut seen: usize = 0;
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
                    if NS.contains(&ns) {
                        let bit = 1usize << NS.iter().position(|n| *n == ns).unwrap_or(0);
                        if seen & bit == 0 {
                            seen |= bit;
                            c.namespaces += 1;
                        }
                    }
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
            "# jupyter\n",
            "import os\n",
            "c.NotebookApp.ip = '*'\n",
            "c.ServerApp.port = 8888\n",
            "c.ServerApp.open_browser = False\n",
            "c.KernelSpecManager.allowed_kernelspecs = ['python3']\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = JupyterConf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.assignments, 4);
        assert_eq!(c.namespaces, 3);
        assert_eq!(c.imports, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(JupyterConf::parse(b"# none\n").is_none());
    }
}
