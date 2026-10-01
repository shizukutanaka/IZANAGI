//! Maven `pom.xml` format.
//!
//! pom.xml has `<project>` root with `<modelVersion>`, GAV
//! (`groupId`/`artifactId`/`version`/`packaging`), `<parent>`,
//! `<modules>`/`<module>`, `<dependencies>`/`<dependency>`,
//! `<dependencyManagement>`, `<build>`/`<plugins>`/`<plugin>`,
//! `<properties>`, `<profiles>`/`<profile>`,
//! `<repositories>`/`<pluginRepositories>`/`<repository>`,
//! `<distributionManagement>` and `<scope>`/`<optional>`/
//! `<classifier>`/`type` on deps.
//!
//! ```
//! let b = concat!(
//!     "<project>\n",
//!     "  <modelVersion>400</modelVersion>\n",
//!     "  <groupId>g</groupId>\n",
//!     "  <artifactId>a</artifactId>\n",
//!     "  <version>1</version>\n",
//!     "  <dependencies>\n",
//!     "    <dependency><groupId>x</groupId><scope>test</scope></dependency>\n",
//!     "  </dependencies>\n",
//!     "</project>\n"
//! ).as_bytes();
//! assert!(izanagi_kit::pom::detect(b));
//! let c = izanagi_kit::pom::Pom::parse(b).unwrap();
//! assert_eq!(c.dependencies, 1);
//! ```

/// Parsed pom.xml summary.
#[derive(Debug, Clone)]
pub struct Pom {
    /// `<groupId>` occurrences.
    pub group_ids: usize,
    /// `<artifactId>` occurrences.
    pub artifact_ids: usize,
    /// `<version>` occurrences.
    pub versions: usize,
    /// `<dependency>` entries (excluding `<dependencies>` wrapper).
    pub dependencies: usize,
    /// `<plugin>` entries.
    pub plugins: usize,
    /// `<module>` entries.
    pub modules: usize,
    /// `<profile>` entries.
    pub profiles: usize,
    /// `<parent>`/`<dependencyManagement>`/`<build>`/`<properties>`/`<repositories>`/`<pluginRepositories>`/`<distributionManagement>`/`<reporting>`/`<scm>`/`<issueManagement>`/`<ciManagement>`/`<developers>`/`<licenses>`/`<organization>`/`<prerequisites>`/`<mailingLists>`/`<contributors>` section tags.
    pub sections: usize,
    /// `<scope>`/`<optional>`/`<classifier>`/`<type>` qualifiers.
    pub qualifiers: usize,
}

fn tag_count(t: &str, n: &str) -> usize {
    t.matches(&format!("<{n} ")).count()
        + t.matches(&format!("<{n}/")).count()
        + t.matches(&format!("<{n}>")).count()
}

/// Whether the buffer looks like pom.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<project") && t.contains("<artifactId")
        || (t.contains("<modelVersion") && t.contains("<groupId"))
}

const SECTIONS: &[&str] = &[
    "parent",
    "dependencyManagement",
    "build",
    "properties",
    "repositories",
    "pluginRepositories",
    "distributionManagement",
    "reporting",
    "scm",
    "issueManagement",
    "ciManagement",
    "developers",
    "licenses",
    "organization",
    "prerequisites",
    "mailingLists",
    "contributors",
    "profiles",
    "modules",
];

impl Pom {
    /// Parses a pom.xml summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        Some(Self {
            group_ids: tag_count(t, "groupId"),
            artifact_ids: tag_count(t, "artifactId"),
            versions: tag_count(t, "version"),
            dependencies: tag_count(t, "dependency"),
            plugins: tag_count(t, "plugin"),
            modules: tag_count(t, "module"),
            profiles: tag_count(t, "profile"),
            sections: SECTIONS.iter().map(|k| tag_count(t, k)).sum(),
            qualifiers: tag_count(t, "scope")
                + tag_count(t, "optional")
                + tag_count(t, "classifier")
                + tag_count(t, "type"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pom() {
        let b = concat!(
            "<project>\n",
            "  <modelVersion>400</modelVersion>\n",
            "  <parent><groupId>p</groupId></parent>\n",
            "  <groupId>g</groupId>\n",
            "  <artifactId>a</artifactId>\n",
            "  <version>1</version>\n",
            "  <packaging>jar</packaging>\n",
            "  <modules><module>m1</module><module>m2</module></modules>\n",
            "  <properties><x>1</x></properties>\n",
            "  <dependencies>\n",
            "    <dependency><groupId>x</groupId><scope>test</scope></dependency>\n",
            "    <dependency><groupId>y</groupId><optional>true</optional></dependency>\n",
            "  </dependencies>\n",
            "  <build><plugins>\n",
            "    <plugin><artifactId>z</artifactId></plugin>\n",
            "  </plugins></build>\n",
            "  <profiles><profile><id>dev</id></profile></profiles>\n",
            "</project>\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Pom::parse(b).unwrap();
        assert_eq!(c.dependencies, 2);
        assert_eq!(c.plugins, 1);
        assert_eq!(c.modules, 2);
        assert_eq!(c.profiles, 1);
        assert_eq!(c.qualifiers, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"<html/>\n"));
        assert!(Pom::parse(b"x").is_none());
    }
}
