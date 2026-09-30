//! Conda `environment.yml` census.
//!
//! `name:` + `channels:` (`- conda-forge` items) + `dependencies:`
//! (`- pkg=ver` items + `- pip:` sub-list of pip specs) +
//! `variables:` (`KEY: value`) + `prefix:`.
//!
//! ```rust
//! let e = "name: myenv\nchannels:\n  - conda-forge\ndependencies:\n  - python=3\n  - pip:\n    - requests\n";
//! let c = izanagi_kit::condaenv::Condaenv::parse(e.as_bytes()).unwrap();
//! assert_eq!(c.channels, 1);
//! assert_eq!(c.deps, 1);
//! assert_eq!(c.pip_deps, 1);
//! ```

/// environment.yml census.
#[derive(Debug, Clone)]
pub struct Condaenv {
    /// `channels:` list items.
    pub channels: usize,
    /// `dependencies:` conda spec items (excludes pip sub-list).
    pub deps: usize,
    /// Pip sub-list items (`- pkg` under `- pip:`).
    pub pip_deps: usize,
    /// `variables:` entries.
    pub variables: usize,
    /// `name:`/`prefix:` headers seen.
    pub named: usize,
}

/// Whether the buffer looks like a conda environment.yml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("dependencies:") || t.contains("channels:"))
        && (t.contains("name:")
            || t.contains("- conda-forge")
            || t.contains("- python")
            || t.contains("prefix:"))
}

impl Condaenv {
    /// Parse an environment.yml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            channels: 0,
            deps: 0,
            pip_deps: 0,
            variables: 0,
            named: 0,
        };
        let mut ctx = "";
        let mut pip = false;
        for l in t.lines() {
            if l.trim().is_empty() || l.trim_start().starts_with('#') {
                continue;
            }
            let s = l.trim_end();
            let indent = s.len() - s.trim_start().len();
            let s = s.trim();
            if indent == 0 {
                pip = false;
                if let Some(colon) = s.find(':') {
                    ctx = &s[..colon];
                    if ctx == "name" || ctx == "prefix" {
                        c.named += 1;
                    }
                }
                continue;
            }
            if ctx == "channels" && s.starts_with('-') {
                c.channels += 1;
            } else if ctx == "dependencies" {
                if s.starts_with("- pip:") {
                    pip = true;
                    continue;
                }
                if s.starts_with('-') {
                    if pip && indent > 2 {
                        c.pip_deps += 1;
                    } else {
                        pip = false;
                        c.deps += 1;
                    }
                }
            } else if ctx == "variables" && s.contains(':') {
                c.variables += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env() {
        let b = concat!(
            "name: datasci\n",
            "channels:\n",
            "  - conda-forge\n",
            "  - bioconda\n",
            "  - defaults\n",
            "dependencies:\n",
            "  - python=3.12\n",
            "  - numpy>=1.26\n",
            "  - pandas\n",
            "  - scikit-learn=1.4\n",
            "  - pip\n",
            "  - pip:\n",
            "    - flask==3.0\n",
            "    - requests\n",
            "    - git+https://github.com/x/y.git\n",
            "variables:\n",
            "  OMP_NUM_THREADS: 4\n",
            "  DATA_DIR: /data\n",
            "prefix: /opt/conda/envs/datasci\n",
        );
        let c = Condaenv::parse(b.as_bytes()).unwrap();
        assert_eq!(c.channels, 3);
        assert_eq!(c.deps, 5);
        assert_eq!(c.pip_deps, 3);
        assert_eq!(c.variables, 2);
        assert_eq!(c.named, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Condaenv::parse(b"foo: 1").is_none());
    }
}
