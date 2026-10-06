//! `pyproject.toml` census (PEP 517/518/621).
//!
//! Canonical sections: `[project]` (`name`/`version`/`dependencies`/
//! `requires-python`/`scripts`/`optional-dependencies`/`entry-points`),
//! `[build-system]` (`requires`/`build-backend`), `[tool.*]` tables
//! (`tool.poetry`/`tool.setuptools`/`tool.black`/`tool.isort`/
//! `tool.mypy`/`tool.pytest.ini_options`/`tool.coverage.*`/
//! `tool.ruff`/`tool.hatch`/`tool.pdm`/`tool.flit`/`tool.uv`/
//! `tool.bandit`/`tool.pylint`/`tool.commitizen`/`tool.cibuildwheel`),
//! `[dependency-groups]` (PEP 735).
//!
//! ```rust
//! let k = b"[build-system]\nrequires = [\"setuptools\"]\nbuild-backend = \"setuptools.build_meta\"\n[project]\nname = \"x\"\nversion = \"1.0\"\n";
//! assert!(izanagi_kit::pyproject::detect(k));
//! ```

/// pyproject.toml census.
#[derive(Debug, Clone)]
pub struct Pyproject {
    /// `[x]`/`[[x]]` section headers.
    pub sections: usize,
    /// `key = value` assignments.
    pub settings: usize,
    /// recognised pyproject sections present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "project",
    "build-system",
    "dependency-groups",
    "poetry",
    "setuptools",
    "setuptools_scm",
    "black",
    "isort",
    "mypy",
    "pytest",
    "coverage",
    "ruff",
    "hatch",
    "pdm",
    "flit",
    "uv",
    "bandit",
    "pylint",
    "commitizen",
    "cibuildwheel",
    "yapf",
    "autopep8",
    "tox",
    "pyright",
    "djlint",
    "maturin",
    "scikit-build",
    "meson-python",
    "setuptools-rust",
    "cargo",
    "vendoring",
    "conda",
    "bumpversion",
    "changelog",
    "copier",
    "cruft",
    "dephell",
    "devpi",
    "docformatter",
    "docstr-coverage",
    "flake8",
    "nbqa",
    "poe",
    "profimp",
    "pydantic",
    "pyrefly",
    "typos",
];

const WEAK: &[&str] = &[
    "optional-dependencies",
    "entry-points",
    "urls",
    "scripts",
    "gui-scripts",
    "requires",
    "build-backend",
    "backend-path",
    "name",
    "version",
    "description",
    "authors",
    "maintainers",
    "license",
    "readme",
    "requires-python",
    "dependencies",
    "dynamic",
    "classifiers",
    "keywords",
    "tool",
    "channels",
    "platforms",
    "feature",
    "activation",
    "system-requirements",
    "pypi-dependencies",
    "tasks",
    "target",
    "environments",
    "solve-groups",
];

fn toml_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    let s = s.split('#').next()?.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(inner) = s.strip_prefix("[[") {
        let inner = inner.strip_suffix("]]").unwrap_or(inner);
        let inner = inner.trim().trim_matches('"');
        return inner
            .split('.')
            .nth(1)
            .or_else(|| inner.split('.').next())
            .filter(|k| !k.is_empty());
    }
    if let Some(inner) = s.strip_prefix('[') {
        let inner = inner.strip_suffix(']').unwrap_or(inner);
        let inner = inner.trim().trim_matches('"');
        // `[tool.X]` → "X"; `[project.X]` → "X"; `[x]` → "x"
        return match inner.strip_prefix("tool.") {
            Some(rest) => rest.split('.').next().filter(|k| !k.is_empty()),
            None => match inner.strip_prefix("project.") {
                Some(rest) => rest.split('.').next().filter(|k| !k.is_empty()),
                None => Some(inner),
            },
        };
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"');
            if k.is_empty() || k.contains(' ') {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a `pyproject.toml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `[project]`/`[build-system]`/`[tool.*]`/`[dependency-groups]`
    // are pyproject-exclusive sections.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = toml_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 1 && strong + weak >= 2
}

impl Pyproject {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') {
                c.sections += 1;
            } else if s.contains('=') {
                c.settings += 1;
            }
            if let Some(k) = toml_key(line) {
                if STRONG.contains(&k) || WEAK.contains(&k) {
                    c.keys += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"[build-system]\nrequires = [\"setuptools\"]\nbuild-backend = \"setuptools.build_meta\"\n[project]\nname = \"x\"\nversion = \"1.0\"\n";
        assert!(detect(b));
        let c = Pyproject::parse(b).unwrap();
        assert!(c.keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[package]\nname = \"x\"\nversion = \"1\"\n"));
        assert!(!detect(b"# [project]\n# name = \"x\"\n[a]\nx = 1\n"));
    }
}
