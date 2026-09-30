//! Census of a Hydra `conf/config.yaml` file.
//!
//! Hydra compose config: `defaults:` list (`- <pkg/name>`/
//! `- <pkg/name@group: item>`/`- _self_`), `_target_` instantiable
//! class references (`pkg.Class` / `${hydra:…}`), a `hydra:` section
//! (`hydra.run.dir`/`hydra.sweep.*`/`hydra.job.*` interpolations),
//! `${oc.env:VAR}`/`${foo.bar}` interpolations and `#` comments.
//! Counts defaults items, `_target_` keys, interpolations, hydra keys.
//!
//! ```rust
//! let c = izanagi_kit::hydraml::HydraMl::parse(
//!     b"defaults:\n  - dataset: cifar10\n  - _self_\n_target_: app.Main\n\
//!       hydra:\n  run:\n    dir: outputs\n",
//! ).unwrap();
//! assert_eq!(c.defaults_items, 2);
//! assert_eq!(c.targets, 1);
//! ```
#![forbid(unsafe_code)]

/// Hydra config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HydraMl {
    /// `defaults:` list items.
    pub defaults_items: usize,
    /// `_target_` instantiable keys.
    pub targets: usize,
    /// `${…}` interpolations.
    pub interpolations: usize,
    /// `hydra:`-scoped keys.
    pub hydra_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// True if `b` looks like a Hydra config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("_target_") || t.contains("defaults:"))
        && (t.contains("${") || t.contains("hydra:") || t.contains("_target_"))
}

impl HydraMl {
    /// Parse a Hydra config file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            defaults_items: 0,
            targets: 0,
            interpolations: t.matches("${").count(),
            hydra_keys: 0,
            comments: 0,
        };
        let mut in_defaults = false;
        let mut in_hydra = false;
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            if l.starts_with('_') && l.contains("_target_") && l.contains(':') {
                c.targets += 1;
                continue;
            }
            if indent == 0 {
                in_defaults = l == "defaults:";
                in_hydra = l == "hydra:";
            }
            if in_defaults && l.starts_with('-') {
                c.defaults_items += 1;
            }
            if in_hydra && l.ends_with(':') && indent > 0 {
                c.hydra_keys += 1;
            }
        }
        if c.defaults_items == 0 && c.targets == 0 {
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
            "defaults:\n",
            "  - dataset: cifar10\n",
            "  - model: resnet\n",
            "  - _self_\n",
            "_target_: app.Main\n",
            "hydra:\n",
            "  run:\n",
            "    dir: outputs/${now:%Y-%m-%d}\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = HydraMl::parse(b.as_bytes()).unwrap();
        assert_eq!(c.defaults_items, 3);
        assert_eq!(c.targets, 1);
        assert_eq!(c.interpolations, 1);
        assert_eq!(c.hydra_keys, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(HydraMl::parse(b"# none\n").is_none());
    }
}
