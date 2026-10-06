//! Bundler `Gemfile` census.
//!
//! Ruby DSL: `source 'https://rubygems.org'` (possibly multiple /
//! `source :rubygems`/`source do`), `gem 'rails'` /
//! `gem 'x', '~> 1.0'` / `gem 'x', github: 'a/b'` /
//! `gem 'x', git: '…'` / `gem 'x', path: '…'` /
//! `gem 'x', require: false` / `gem 'x', platforms: %i[jruby]`,
//! `ruby '3.3.0'`/`ruby "~> 3.3"`/`ruby file: ".ruby-version"`,
//! `gemspec`/`gemspec name:`, `group :development do`,
//! `group :test do`, `group [:development, :test] do`,
//! `platforms :ruby do`, `platform :jruby do`,
//! `eval_gemfile 'Gemfile.x'`, `plugin 'x'`,
//! `install_if -> { ... }`, `gemfile`, `git_source(:github)`,
//! `source 'x' do … gem 'y' … end`, `ruby_gem 'x'`,
//! `bundle config`. Also `gemspec` referencing the sibling .gemspec.
//!
//! ```rust
//! let k = b"source 'https://rubygems.org'\nruby '3.3.0'\ngem 'rails'\ngem 'puma', '~> 6.0'\ngroup :test do\n  gem 'rspec'\nend\n";
//! assert!(izanagi_kit::gemfile::detect(k));
//! ```

/// Gemfile census.
#[derive(Debug, Clone)]
pub struct Gemfile {
    /// `gem`/`group`/`source`/DSL lines.
    pub directives: usize,
    /// `gem 'x'` entries.
    pub gems: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const CALLS: &[&str] = &[
    "source ",
    "gem ",
    "gemspec",
    "ruby ",
    "ruby(",
    "group ",
    "platform ",
    "platforms ",
    "eval_gemfile",
    "plugin ",
    "install_if",
    "git_source",
    "gemfile",
    "path ",
];

fn call(line: &str) -> Option<&'static str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    CALLS.iter().find(|&&c| s.starts_with(c)).copied()
}

/// Detect a `Gemfile`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `source 'x'` + `gem 'x'`/`group :x do`/`gemspec`/
    // `ruby 'x'`/`eval_gemfile` are bundler-only DSL calls.
    let mut n = 0usize;
    for line in t.lines() {
        if call(line).is_some() {
            n += 1;
        }
    }
    n >= 2
}

impl Gemfile {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            directives: 0,
            gems: 0,
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
            if let Some(k) = call(line) {
                c.directives += 1;
                if k == "gem " {
                    c.gems += 1;
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
        let b = b"source 'https://rubygems.org'\nruby '3.3.0'\ngem 'rails'\ngem 'puma', '~> 6.0'\ngroup :test do\n  gem 'rspec'\nend\n";
        assert!(detect(b));
        let c = Gemfile::parse(b).unwrap();
        assert_eq!(c.gems, 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"x = 1\ny = 2\n"));
        assert!(!detect(b"# gem 'x'\n# source 'y'\nz = 1\n"));
    }
}
