//! Puppet manifest (`.pp`) census.
//!
//! Puppet manifests declare classes (`class name {`/`class { 'name':`),
//! nodes (`node 'x' {`), defined types (`define name(`), resources
//! (`package { 'x': ensure => installed }`), resource references
//! (`Package['x']`, `Service['x']`), `$variables`, `=>` hash rockets and
//! metaparameters (`notify`/`subscribe`/`require`/`before`/`ensure`/`tag`).
//! `parse` counts each class.
//!
//! ```rust
//! let p = concat!(
//!     "class web {\n",
//!     "  package { 'nginx':\n",
//!     "    ensure => installed,\n",
//!     "  }\n",
//!     "  service { 'nginx':\n",
//!     "    ensure => running,\n",
//!     "    require => Package['nginx'],\n",
//!     "  }\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::puppet::Puppet::parse(p.as_bytes()).unwrap();
//! assert_eq!(c.classes, 1);
//! assert_eq!(c.resources, 2);
//! assert_eq!(c.references, 1);
//! ```

/// Puppet manifest census.
#[derive(Debug, Clone)]
pub struct Puppet {
    /// `class`/`class {` declarations.
    pub classes: usize,
    /// `node 'x'`/`node default` blocks.
    pub nodes: usize,
    /// `define` blocks.
    pub defines: usize,
    /// `type { 'title':` resource declarations.
    pub resources: usize,
    /// `=>` attribute assignments.
    pub attributes: usize,
    /// `$var` occurrences.
    pub variables: usize,
    /// `Type['title']` resource references.
    pub references: usize,
    /// `notify`/`subscribe`/`require`/`before`/`ensure` metaparameter keys.
    pub metaparams: usize,
    /// `include`/`contain`/`realize`/`require` calls.
    pub includes: usize,
    /// `if`/`elsif`/`else`/`unless`/`case` lines.
    pub conditionals: usize,
}

const TYPES: &[&str] = &[
    "Package",
    "Service",
    "File",
    "Exec",
    "User",
    "Group",
    "Notify",
    "Cron",
    "Mount",
    "File_line",
    "Anchor",
    "Concat",
    "Ini_setting",
    "Firewall",
    "Class",
    "Stage",
    "Vcsrepo",
];

/// Whether the buffer looks like a Puppet manifest.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("=>")
        && (t.contains(" { '")
            || t.contains(" { \"")
            || t.contains("package")
            || t.contains("class ")
            || t.contains("node "))
}

impl Puppet {
    /// Parse a Puppet manifest into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            classes: 0,
            nodes: 0,
            defines: 0,
            resources: 0,
            attributes: 0,
            variables: 0,
            references: 0,
            metaparams: 0,
            includes: 0,
            conditionals: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s.contains("=>") {
                c.attributes += 1;
            }
            c.variables += s.matches('$').count();
            for ty in TYPES {
                c.references += s.matches(&format!("{ty}[")).count();
            }
            if s.starts_with("class ") || s.starts_with("class{") {
                c.classes += 1;
                continue;
            }
            if s.starts_with("node ") {
                c.nodes += 1;
                continue;
            }
            if s.starts_with("define ") {
                c.defines += 1;
                continue;
            }
            if s.starts_with("include") || s.starts_with("contain") || s.starts_with("realize") {
                c.includes += 1;
                continue;
            }
            if s.starts_with("if ")
                || s.starts_with("elsif")
                || s.starts_with("else")
                || s.starts_with("unless ")
                || s.starts_with("case ")
            {
                c.conditionals += 1;
                continue;
            }
            if s.contains("{ '") || s.contains("{ \"") {
                c.resources += 1;
                continue;
            }
            for mp in [
                "notify",
                "subscribe",
                "require",
                "before",
                "tag",
                "schedule",
            ] {
                if s.starts_with(mp) && s[mp.len()..].trim_start().starts_with("=>") {
                    c.metaparams += 1;
                    break;
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
    fn parses_manifest() {
        let b = concat!(
            "class web {\n",
            "  package { 'nginx':\n",
            "    ensure => installed,\n",
            "  }\n",
            "  service { 'nginx':\n",
            "    ensure => running,\n",
            "    require => Package['nginx'],\n",
            "    notify => Service['nginx'],\n",
            "  }\n",
            "  $owner = 'root'\n",
            "  file { '/etc/x':\n",
            "    ensure => file,\n",
            "  }\n",
            "  if $facts['os'] {\n",
            "    notify { 'yes': }\n",
            "  }\n",
            "}\n",
            "node 'web1' {\n",
            "  include web\n",
            "}\n",
        );
        let c = Puppet::parse(b.as_bytes()).unwrap();
        assert_eq!(c.classes, 1);
        assert_eq!(c.nodes, 1);
        assert_eq!(c.resources, 4);
        assert_eq!(c.attributes, 5);
        assert_eq!(c.references, 2);
        assert_eq!(c.metaparams, 2);
        assert_eq!(c.includes, 1);
        assert_eq!(c.conditionals, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Puppet::parse(b"hello").is_none());
    }
}
