//! Apache Ivy `ivy.xml` format.
//!
//! ivy.xml has `<ivy-module version=` root with `<info organisation=
//! module= revision=>`, `<configurations>`/`<conf>`,
//! `<publications>`/`<artifact>`, `<dependencies>`/`<dependency
//! org name rev conf=>`, `<exclude>`/`<override>`/`<conflict>`/
//! `<extends>` elements.
//!
//! ```
//! let b = concat!(
//!     "<ivy-module version=\"200\">\n",
//!     "  <info organisation=\"org\" module=\"m\"/>\n",
//!     "  <configurations><conf name=\"default\"/></configurations>\n",
//!     "  <publications><artifact name=\"m\" type=\"jar\"/></publications>\n",
//!     "  <dependencies>\n",
//!     "    <dependency org=\"x\" name=\"y\" rev=\"1\"/>\n",
//!     "    <dependency org=\"a\" name=\"b\" rev=\"2\"><exclude org=\"z\"/></dependency>\n",
//!     "  </dependencies>\n",
//!     "</ivy-module>\n"
//! ).as_bytes();
//! assert!(izanagi_kit::ivy::detect(b));
//! let c = izanagi_kit::ivy::Ivy::parse(b).unwrap();
//! assert_eq!(c.dependencies, 2);
//! ```

/// Parsed ivy.xml summary.
#[derive(Debug, Clone)]
pub struct Ivy {
    /// `<info>` occurrences.
    pub info: usize,
    /// `<conf>` configurations.
    pub configurations: usize,
    /// `<artifact>` publications.
    pub publications: usize,
    /// `<dependency>` entries.
    pub dependencies: usize,
    /// `<exclude>`/`<override>`/`<conflict>`/`<extends>` entries.
    pub overrides: usize,
}

fn tag_count(t: &str, n: &str) -> usize {
    t.matches(&format!("<{n} ")).count()
        + t.matches(&format!("<{n}/")).count()
        + t.matches(&format!("<{n}>")).count()
}

/// Whether the buffer looks like ivy.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<ivy-module") && (t.contains("<info") || t.contains("<dependency"))
}

impl Ivy {
    /// Parses an ivy.xml summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        Some(Self {
            info: tag_count(t, "info"),
            configurations: tag_count(t, "conf"),
            publications: tag_count(t, "artifact"),
            dependencies: tag_count(t, "dependency"),
            overrides: tag_count(t, "exclude")
                + tag_count(t, "override")
                + tag_count(t, "conflict")
                + tag_count(t, "extends"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ivy() {
        let b = concat!(
            "<ivy-module version=\"200\">\n",
            "  <info organisation=\"org\" module=\"m\"/>\n",
            "  <configurations>\n",
            "    <conf name=\"default\"/>\n",
            "    <conf name=\"test\"/>\n",
            "  </configurations>\n",
            "  <publications>\n",
            "    <artifact name=\"m\" type=\"jar\"/>\n",
            "    <artifact name=\"m\" type=\"source\"/>\n",
            "  </publications>\n",
            "  <dependencies>\n",
            "    <dependency org=\"x\" name=\"y\" rev=\"1\"/>\n",
            "    <dependency org=\"a\" name=\"b\" rev=\"2\"><exclude org=\"z\"/></dependency>\n",
            "    <override org=\"c\" module=\"d\" rev=\"3\"/>\n",
            "  </dependencies>\n",
            "</ivy-module>\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Ivy::parse(b).unwrap();
        assert_eq!(c.info, 1);
        assert_eq!(c.configurations, 2);
        assert_eq!(c.publications, 2);
        assert_eq!(c.dependencies, 2);
        assert_eq!(c.overrides, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"<html/>\n"));
        assert!(Ivy::parse(b"x").is_none());
    }
}
