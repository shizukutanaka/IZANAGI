//! `tox.ini` census.
//!
//! `[tox]` (`envlist`/`minversion`/`requires`/`isolated_build`/
//! `ignore_basepython_conflict`/`toxworkdir`/`provision`/
//! `skipsdist`/`labels`/`env_list`/`min_python`/`ignore_missing_provision`
//! `requires_within_console`) and `[testenv]`/`[testenv:NAME]`
//! (`deps`/`commands`/`setenv`/`passenv`/`changedir`/`basepython`/
//! `skip_install`/`extras`/`allowlist_externals`/`commands_pre`/
//! `commands_post`/`description`/`depends`/`envdir`/`envtmpdir`/
//! `envlogdir`/`install_command`/`list_dependencies_command`/`pip_pre`/
//! `download`/`sitepackages`/`alwayscopy`/`recreate`/`usedevelop`/
//! `platform`/`parallel_show_output`/`runner`/`package`/`package_env`/
//! `extras`/`labels`/`runner`/`suicide_timeout`/`interrupt_timeout`/
//! `terminate_timeout`/`ignore_errors`/`ignore_outcome`).
//!
//! ```rust
//! let t = "[tox]\nenvlist = py312\n[testenv]\ndeps = pytest\ncommands = pytest\n[testenv:lint]\ndeps = ruff\ncommands = ruff .\n";
//! let c = izanagi_kit::toxini::Toxini::parse(t.as_bytes()).unwrap();
//! assert_eq!(c.envs, 1);
//! ```

/// tox.ini census.
#[derive(Debug, Clone)]
pub struct Toxini {
    /// `[tox]`/`[testenv]`/`[testenv:NAME]`/`[gh]`/`[pytest]` sections.
    pub sections: usize,
    /// `[testenv:NAME]` env sections.
    pub envs: usize,
    /// `key = value`/`key =` entries.
    pub settings: usize,
    /// Continuation items (indented list lines).
    pub items: usize,
    /// Recognised tox key names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "envlist",
    "env_list",
    "minversion",
    "requires",
    "isolated_build",
    "ignore_basepython_conflict",
    "toxworkdir",
    "provision",
    "skipsdist",
    "labels",
    "min_python",
    "ignore_missing_provision",
    "requires_within_console",
    "deps",
    "commands",
    "setenv",
    "passenv",
    "changedir",
    "basepython",
    "skip_install",
    "extras",
    "allowlist_externals",
    "whitelist_externals",
    "commands_pre",
    "commands_post",
    "description",
    "depends",
    "envdir",
    "envtmpdir",
    "envlogdir",
    "install_command",
    "list_dependencies_command",
    "pip_pre",
    "download",
    "sitepackages",
    "alwayscopy",
    "recreate",
    "usedevelop",
    "platform",
    "parallel_show_output",
    "runner",
    "package",
    "package_env",
    "package_root",
    "package_tox_env_type",
    "suicide_timeout",
    "interrupt_timeout",
    "terminate_timeout",
    "ignore_errors",
    "ignore_outcome",
    "args_are_paths",
    "legacy_tox_ini",
    "build_root",
    "temp_dir",
    "work_dir",
    "no_package",
    "skip_missing_interpreters",
    "skip_missing_interpreters_json",
    "factor",
    "constraints",
    "dependency_groups",
    "use_frozen_constraints",
    "seed",
    "parallel",
    "label",
    "description_file",
];

/// Whether the buffer looks like a tox.ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("[tox]")
        || t.contains("[testenv]")
        || t.contains("[testenv:")
        || t.contains("envlist")
        || t.contains("env_list")
}

impl Toxini {
    /// Parse a tox.ini into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            envs: 0,
            settings: 0,
            items: 0,
            named: 0,
        };
        for l in t.lines() {
            if l.trim().is_empty() || l.trim_start().starts_with(['#', ';']) {
                continue;
            }
            let s = l.trim_end();
            let indent = s.len() - s.trim_start().len();
            let s = s.trim();
            if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
                let name = s.trim_matches(['[', ']']);
                if name.starts_with("testenv:") {
                    c.envs += 1;
                }
                continue;
            }
            if let Some(eq) = s.find('=') {
                c.settings += 1;
                let key = s[..eq].trim();
                if KEYS.contains(&key) {
                    c.named += 1;
                }
            } else if indent > 0 {
                c.items += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ini() {
        let b = concat!(
            "[tox]\n",
            "envlist = py39, py310, py311, py312, lint\n",
            "minversion = 4.0\n",
            "requires =\n",
            "    tox>=4\n",
            "isolated_build = true\n",
            "labels =\n",
            "    test = py312\n",
            "[testenv]\n",
            "description = run the tests\n",
            "deps =\n",
            "    pytest\n",
            "    pytest-cov\n",
            "setenv =\n",
            "    PYTHONPATH = {toxinidir}\n",
            "passenv =\n",
            "    HOME\n",
            "commands =\n",
            "    pytest {posargs}\n",
            "commands_pre =\n",
            "    python -V\n",
            "commands_post =\n",
            "    coverage report\n",
            "[testenv:lint]\n",
            "deps = ruff\n",
            "commands = ruff check .\n",
            "skip_install = true\n",
            "[testenv:docs]\n",
            "deps =\n",
            "    sphinx\n",
            "    furo\n",
            "changedir = docs\n",
            "commands = sphinx-build -W -b html . _build\n",
            "[testenv:typecheck]\n",
            "deps =\n",
            "    mypy\n",
            "commands = mypy src\n",
            "[gh]\n",
            "python =\n",
            "    3.12 = py312\n",
        );
        let c = Toxini::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.envs, 3);
        assert_eq!(c.settings, 25);
        assert_eq!(c.items, 9);
        assert!(c.named >= 15);
    }

    #[test]
    fn rejects_other() {
        assert!(Toxini::parse(b"[foo]\na = 1").is_none());
    }
}
