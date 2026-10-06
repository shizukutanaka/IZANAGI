//! Elixir `mix.exs` census.
//!
//! `defmodule X.MixProject do` + `use Mix.Project`,
//! `def project do` returning `[ app: :x, version: "1.0",
//! elixir: "~> 1.x", elixirc_paths: ..., start_permanent: ...,
//! deps: deps(), package: package(), docs: docs(),
//! preferred_cli_env: ..., description: ..., source_url: ...,
//! homepage_url: ..., dialyzer: ..., escript: ..., releases: ...,
//! aliases: ..., elixirc_options: ..., compilers: ...,
//! consolidate_protocols: ..., xref: ...]`,
//! `defp deps do [{:poison, "~> 4.0"}, {:x, git: "…"},
//! {:x, path: "…"}, {:x, github: "…"}, {:x, ">= 1.0", only: :dev},
//! {:x, in_umbrella: true}] end`,
//! `def application do [mod: {X.Application, []},
//! extra_applications: [:logger], applications: [:x]] end`,
//! `defp package`, `defp aliases`, `def cli`, `defp dialyzer`.
//!
//! ```rust
//! let k = b"defmodule X.MixProject do\n  use Mix.Project\n  def project do\n    [app: :x, version: \"1.0\", deps: deps()]\n  end\n  defp deps do\n    [{:poison, \"~> 4.0\"}, {:y, github: \"a/b\"}]\n  end\nend\n";
//! assert!(izanagi_kit::mix::detect(k));
//! ```

/// mix.exs census.
#[derive(Debug, Clone)]
pub struct Mix {
    /// `def`/`defp`/`use`/`app:`/keyword lines.
    pub directives: usize,
    /// `{:` tuple deps.
    pub deps: usize,
    /// recognised mix markers present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const MARKERS: &[&str] = &[
    "use Mix.Project",
    "MixProject",
    "def project",
    "defp deps",
    "def application",
    "app:",
    "elixir:",
    "elixirc_paths",
    "start_permanent",
    "preferred_cli_env",
    "elixirc_options",
    "consolidate_protocols",
    "extra_applications",
    "applications:",
    "in_umbrella",
    "escript:",
    "releases:",
    "aliases:",
    "defp package",
    "source_url",
    "homepage_url",
    "dialyzer",
    "xref:",
    "xref",
    "docs:",
    "main_module",
    "modules:",
    "compilers:",
    "def cli",
    "deps:",
    "description:",
    "licenses:",
    "links:",
    "files:",
    "maintainers:",
];

fn tuple_dep(line: &str) -> bool {
    let s = line.trim();
    s.starts_with("{:") || s.contains("{:")
}

/// Detect a `mix.exs` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `use Mix.Project`/`MixProject`/`defp deps`/`app:`/`elixir:`
    // are mix-exclusive markers.
    let mut n = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if MARKERS.iter().any(|m| s.contains(m)) {
            n += 1;
        }
    }
    n >= 2
}

impl Mix {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            directives: 0,
            deps: 0,
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
            if s.starts_with("def")
                || s.starts_with("use ")
                || s.starts_with('@')
                || s.contains(':')
            {
                c.directives += 1;
            }
            if tuple_dep(line) {
                c.deps += 1;
            }
            if MARKERS.iter().any(|m| s.contains(m)) {
                c.keys += 1;
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
        let b = b"defmodule X.MixProject do\n  use Mix.Project\n  def project do\n    [app: :x, version: \"1.0\", deps: deps()]\n  end\n  defp deps do\n    [{:poison, \"~> 4.0\"}, {:y, github: \"a/b\"}]\n  end\nend\n";
        assert!(detect(b));
        let c = Mix::parse(b).unwrap();
        assert!(c.keys >= 4);
        assert!(c.deps >= 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"x = 1\ny = 2\n"));
        assert!(!detect(b"# use Mix.Project\n# defp deps\nx = 1\n"));
    }
}
