//! Gradle Module Metadata (`.module` JSON).
//!
//! `.module` files carry `formatVersion`, `component`
//! (`group`/`module`/`version`/`attributes`/`url`), and `variants`
//! with `name`/`attributes`/`files`/`dependencies`/
//! `dependency-locking`/`capabilities`/`available-at`.
//!
//! ```
//! let b = br#"{
//!   "formatVersion": "1",
//!   "component": {
//!     "group": "g", "module": "m", "version": "1",
//!     "attributes": { "org.gradle.status": "release" },
//!     "url": "m-1.pom"
//!   },
//!   "variants": [
//!     {
//!       "name": "apiElements",
//!       "attributes": { "org.gradle.usage": "java-api" },
//!       "files": [ { "name": "m-1.jar" } ],
//!       "dependencies": [ { "group": "x" } ]
//!     }
//!   ]
//! }"#;
//! assert!(izanagi_kit::gradlemod::detect(b));
//! let c = izanagi_kit::gradlemod::Gradlemod::parse(b).unwrap();
//! assert_eq!(c.variants, 1);
//! assert_eq!(c.files, 1);
//! ```

/// Parsed .module summary.
#[derive(Debug, Clone)]
pub struct Gradlemod {
    /// `variants` array entries.
    pub variants: usize,
    /// `files` entries across variants.
    pub files: usize,
    /// `dependencies` entries across variants.
    pub dependencies: usize,
    /// `org.gradle.*` attribute keys.
    pub attributes: usize,
    /// `capabilities`/`available-at` entries.
    pub capabilities: usize,
    /// `dependency-locking` entries.
    pub dependency_locking: usize,
}

/// Whether the buffer looks like Gradle module metadata.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"formatVersion\"")
        && (t.contains("\"variants\"") || t.contains("\"component\""))
        && t.contains("org.gradle.")
}

impl Gradlemod {
    /// Parses a .module summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        // Each variant has exactly one `"attributes"` block; subtract
        // the component-level one. Each variant and each `files` entry
        // has `"name":`.
        let variants = t.matches("\"attributes\":").count().saturating_sub(1);
        let files = t.matches("\"name\":").count().saturating_sub(variants);
        Some(Self {
            variants,
            files,
            dependencies: t.matches("\"group\":").count().saturating_sub(1),
            attributes: t.matches("org.gradle.").count(),
            capabilities: t.matches("\"capabilities\":").count()
                + t.matches("\"available-at\":").count(),
            dependency_locking: t.matches("\"dependency-locking\":").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gradlemod() {
        let b = br#"{
  "formatVersion": "1",
  "component": {
    "group": "g", "module": "m", "version": "1",
    "attributes": { "org.gradle.status": "release" },
    "url": "m-1.pom"
  },
  "variants": [
    {
      "name": "apiElements",
      "attributes": { "org.gradle.usage": "java-api", "org.gradle.category": "library" },
      "files": [ { "name": "m-1.jar" } ],
      "dependencies": [ { "group": "x" } ]
    },
    {
      "name": "runtimeElements",
      "attributes": { "org.gradle.usage": "java-runtime" },
      "files": [ { "name": "m-1.jar" } ],
      "dependency-locking": { "dependencies": [] }
    }
  ]
}"#;
        assert!(detect(b));
        let c = Gradlemod::parse(b).unwrap();
        assert_eq!(c.variants, 2);
        assert_eq!(c.files, 2);
        assert_eq!(c.dependencies, 1);
        assert_eq!(c.attributes, 4);
        assert_eq!(c.dependency_locking, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{}"));
        assert!(Gradlemod::parse(b"x").is_none());
    }
}
