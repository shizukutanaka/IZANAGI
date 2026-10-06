//! RubyGem `.gemspec` census.
//!
//! A gemspec is Ruby: `Gem::Specification.new do |spec|` (or
//! `Gem::Specification.new`), then `spec.name`/`spec.version`/
//! `spec.authors`/`spec.summary`/`spec.description`/`spec.homepage`/
//! `spec.license`/`spec.files`/`spec.require_paths`/
//! `spec.executables`/`spec.bindir`/`spec.extensions`/
//! `spec.metadata`/`spec.post_install_message`/
//! `spec.required_ruby_version`/`spec.required_rubygems_version`/
//! `spec.add_dependency`/`spec.add_runtime_dependency`/
//! `spec.add_development_dependency` — short `s.` receiver also
//! common (`s.name = "x"`).
//!
//! ```rust
//! let k = b"Gem::Specification.new do |spec|\n  spec.name = \"x\"\n  spec.version = \"1.0\"\n  spec.summary = \"y\"\n  spec.add_dependency \"z\"\nend\n";
//! assert!(izanagi_kit::gemspec::detect(k));
//! ```

/// gemspec census.
#[derive(Debug, Clone)]
pub struct Gemspec {
    /// `spec.X`/`s.X` attribute assignments or calls.
    pub attributes: usize,
    /// `add_dependency`-family calls.
    pub dependencies: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const ATTRS: &[&str] = &[
    "name",
    "version",
    "authors",
    "author",
    "email",
    "summary",
    "description",
    "homepage",
    "license",
    "licenses",
    "files",
    "require_paths",
    "require_path",
    "executables",
    "bindir",
    "extensions",
    "metadata",
    "post_install_message",
    "required_ruby_version",
    "required_rubygems_version",
    "platform",
    "date",
    "rubygems_version",
    "specification_version",
    "signing_key",
    "cert_chain",
    "test_files",
    "extra_rdoc_files",
    "rdoc_options",
    "requirements",
    "default_executable",
    "has_rdoc",
];

const DEP_CALLS: &[&str] = &[
    "add_dependency",
    "add_runtime_dependency",
    "add_development_dependency",
];

fn gem_line(line: &str) -> (bool, bool) {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return (false, false);
    }
    let spec_marker = s.contains("Gem::Specification");
    let rest = s
        .strip_prefix("spec.")
        .or_else(|| s.strip_prefix("s."))
        .or_else(|| s.strip_prefix("gem."));
    if rest.is_none() && !spec_marker {
        return (false, false);
    }
    let attr = match rest {
        Some(r) => {
            let k = r.split([' ', '=', '(', ',']).next().unwrap_or("");
            ATTRS.contains(&k) || DEP_CALLS.iter().any(|d| r.starts_with(d))
        }
        None => false,
    };
    (attr || spec_marker, DEP_CALLS.iter().any(|d| s.contains(d)))
}

/// Detect a `.gemspec` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `Gem::Specification` + `spec.name=`/`add_dependency` are
    // gemspec-only constructs.
    let mut spec = 0usize;
    let mut attrs = 0usize;
    for line in t.lines() {
        let (a, _) = gem_line(line);
        if line.trim().contains("Gem::Specification") {
            spec += 1;
        }
        if a {
            attrs += 1;
        }
    }
    spec >= 1 && attrs >= 2 || attrs >= 4
}

impl Gemspec {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            attributes: 0,
            dependencies: 0,
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
            let (a, d) = gem_line(line);
            if a {
                c.attributes += 1;
            }
            if d {
                c.dependencies += 1;
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
        let b = b"Gem::Specification.new do |spec|\n  spec.name = \"x\"\n  spec.version = \"1.0\"\n  spec.summary = \"y\"\n  spec.add_dependency \"z\"\nend\n";
        assert!(detect(b));
        let c = Gemspec::parse(b).unwrap();
        assert!(c.attributes >= 4);
        assert_eq!(c.dependencies, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"name = \"x\"\nversion = \"1\"\n"));
        assert!(!detect(
            b"# Gem::Specification.new\n# spec.name = \"x\"\nz = 1\n"
        ));
    }
}
