//! JPA `persistence.xml` の検出と構造カウント。
//!
//! `<persistence>` ルート + `<persistence-unit>`/`<provider>`/
//! `<jta-data-source>`/`<non-jta-data-source>`/`<class>`/`<properties>`/
//! `<property>`/`jakarta.persistence.*`/`javax.persistence.*` 属性を
//! 識別する。
//!
//! ```
//! let b = br#"<persistence xmlns="https://jakarta.ee/xml/ns/persistence" version="3.0">
//!   <persistence-unit name="app" transaction-type="JTA">
//!     <provider>org.hibernate.jpa.HibernatePersistenceProvider</provider>
//!     <jta-data-source>jdbc/app</jta-data-source>
//!     <class>x.User</class>
//!     <properties>
//!       <property name="jakarta.persistence.schema-generation" value="drop-and-create"/>
//!     </properties>
//!   </persistence-unit>
//! </persistence>"#;
//! assert!(izanagi_kit::persistencexml::detect(b));
//! let c = izanagi_kit::persistencexml::PersistenceXml::parse(b).unwrap();
//! assert_eq!(c.elements, 6);
//! ```

/// Parsed persistence.xml summary.
#[derive(Debug, Clone)]
pub struct PersistenceXml {
    /// `<persistence>` root present.
    pub has_root: bool,
    /// Recognized descriptor elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<persistence-unit")
        || tr.starts_with("<provider")
        || tr.starts_with("<jta-data-source")
        || tr.starts_with("<non-jta-data-source")
        || tr.starts_with("<mapping-file")
        || tr.starts_with("<jar-file")
        || tr.starts_with("<class>")
        || tr.starts_with("<class ")
        || tr.starts_with("<exclude-unlisted-classes")
        || tr.starts_with("<shared-cache-mode")
        || tr.starts_with("<validation-mode")
        || tr.starts_with("<properties>")
        || tr.starts_with("<property")
        || tr.starts_with("<qualifier")
        || tr.starts_with("<scope")
        || tr.starts_with("<description")
}

fn jpa_marker(tr: &str) -> bool {
    tr.contains("jakarta.persistence.") || tr.contains("javax.persistence.")
}

/// Detect a JPA persistence.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<persistence ") || tr.starts_with("<persistence>") {
            root = true;
            continue;
        }
        if tr.starts_with("<!--") {
            continue;
        }
        if element(tr) || jpa_marker(tr) {
            elems += 1;
        }
    }
    (root && elems >= 1) || elems >= 4
}

impl PersistenceXml {
    /// Count categories. Returns `None` when the input does not look like
    /// a persistence.xml descriptor.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            has_root: false,
            elements: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("<persistence ") || tr.starts_with("<persistence>") {
                c.has_root = true;
            } else if tr.starts_with("<!--") {
                c.comments += 1;
            } else if element(tr) || jpa_marker(tr) {
                c.elements += 1;
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`PersistenceXml::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<PersistenceXml> {
    PersistenceXml::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<?xml version="1.0"?>
<persistence xmlns="https://jakarta.ee/xml/ns/persistence" version="3.0">
  <persistence-unit name="orders" transaction-type="JTA">
    <description>orders PU</description>
    <provider>org.hibernate.jpa.HibernatePersistenceProvider</provider>
    <jta-data-source>java:/ds/orders</jta-data-source>
    <mapping-file>orm.xml</mapping-file>
    <jar-file>entities.jar</jar-file>
    <class>x.Order</class>
    <class>x.OrderLine</class>
    <exclude-unlisted-classes>false</exclude-unlisted-classes>
    <shared-cache-mode>ALL</shared-cache-mode>
    <validation-mode>AUTO</validation-mode>
    <properties>
      <property name="jakarta.persistence.schema-generation.database.action" value="drop-and-create"/>
      <property name="hibernate.dialect" value="org.hibernate.dialect.PostgreSQLDialect"/>
      <property name="javax.persistence.jdbc.url" value="jdbc:postgresql://db/orders"/>
    </properties>
  </persistence-unit>
  <persistence-unit name="audit">
    <non-jta-data-source>java:/ds/audit</non-jta-data-source>
  </persistence-unit>
</persistence>"#;
        assert!(detect(b));
        let c = PersistenceXml::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 16);
    }

    #[test]
    fn rejects_generic_xml() {
        assert!(!detect(b"<persistence-x><a/></persistence-x>"));
        assert!(!detect(b"<config><item>1</item></config>"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"<!-- <persistence-unit name=\"x\"> -->\n<!-- <provider>y</provider> -->"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(PersistenceXml::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"<persistence>");
        assert!(!detect(&b));
    }
}
