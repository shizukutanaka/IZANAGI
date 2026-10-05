//! Elixir `mix.exs` プロジェクト定義の検出と構造カウント。
//!
//! `defmodule X.MixProject do`、`def project do`/`def deps do`/
//! `def application do` ブロック、`app:`/`version:`/`deps:` 等のアトムキー、
//! `{:dep, "~> x"}` 依存タプルを識別する。
//!
//! ```
//! let c = izanagi_kit::mixexs::parse(
//!     b"defmodule M.MixProject do\n  def project do\n    [app: :m, version: \"0.1\", deps: deps()]\n  end\n  defp deps do\n    [{:plug, \"~> 1.0\"}]\n  end\nend\n").unwrap();
//! assert_eq!(c.sections, 3);
//! assert!(izanagi_kit::mixexs::detect(
//!     b"defmodule X.MixProject do\n  def project, do: []\nend\n"));
//! ```

/// `def`/`defp` ブロック名(一般化せず既知名のみ別カウントは行わない)。
/// プロジェクト既知アトムキー。
const ATOM_KEYS: &[&str] = &[
    "aliases:",
    "app:",
    "archives:",
    "build_embedded:",
    "build_path:",
    "compilers:",
    "consolidate_protocols:",
    "default_task:",
    "deps:",
    "deps_path:",
    "description:",
    "docs:",
    "elixirc_options:",
    "elixirc_paths:",
    "elixir:",
    "env:",
    "escript:",
    "extra_applications:",
    "homepage_url:",
    "lockfile:",
    "mod:",
    "name:",
    "package:",
    "preferred_cli_env:",
    "preferred_cli_target:",
    "releases:",
    "source_url:",
    "source_url_pattern:",
    "start_permanent:",
    "test_coverage:",
    "version:",
    "xref:",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `defmodule`/`def ... do` ブロック行数。
    pub sections: usize,
    /// 既知アトムキー出現行数。
    pub options: usize,
    /// `{:name, ...}` 依存タプル行数。
    pub entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn is_dep_tuple(t: &str) -> bool {
    t.starts_with('{') && t.starts_with("{:") && t.contains(',')
}

fn atom_hits(t: &str) -> usize {
    ATOM_KEYS.iter().filter(|k| t.contains(**k)).count()
}

/// `mix.exs` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut mixproj = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("defmodule") && t.contains("MixProject") {
            mixproj = true;
        }
        if (t.starts_with("def ") || t.starts_with("defp "))
            && (t.contains("project") || t.contains("deps") || t.contains("application"))
        {
            hits += 1;
        }
        if atom_hits(t) >= 1 {
            hits += 1;
        }
        if mixproj && hits >= 2 {
            return true;
        }
    }
    mixproj && hits >= 1
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        entries: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with("defmodule") || t.starts_with("def ") || t.starts_with("defp ") {
            c.sections += 1;
            continue;
        }
        if is_dep_tuple(t.trim_start_matches('[')) {
            c.entries += 1;
            continue;
        }
        if atom_hits(t) > 0 {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"defmodule Demo.MixProject do\n  use Mix.Project\n\n  def project do\n    [\n      app: :demo,\n      version: \"0.1.0\",\n      elixir: \"~> 1.15\",\n      deps: deps(),\n      aliases: aliases()\n    ]\n  end\n\n  def application do\n    [extra_applications: [:logger]]\n  end\n\n  defp deps do\n    [\n      {:plug, \"~> 1.14\"},\n      {:phoenix, \"~> 1.7\", only: :prod}\n    ]\n  end\n\n  defp aliases do\n    []\n  end\nend\n";

    #[test]
    fn mixexs() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.options, 6);
        assert_eq!(c.entries, 2);
        assert_eq!(c.misc, 11);
    }

    #[test]
    fn not_mixexs() {
        assert!(!detect(b"defmodule A do\nend\n"));
        assert!(parse(b"text\n").is_none());
    }
}
