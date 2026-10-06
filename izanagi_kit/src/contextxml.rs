//! Tomcat `context.xml` の検出と構造カウント。
//!
//! `<Context>` ルート + `<Resource>`/`<ResourceLink>`/`<Environment>`/
//! `<Valve>`/`<Realm>`/`<Loader>`/`<Manager>`/`<Parameter>`/
//! `<WatchedResource>`/`docBase`/`path`/`jndi` 属性を識別する。
//!
//! ```
//! let b = br#"<Context path="/app" docBase="app.war" reloadable="false">
//!   <Resource name="jdbc/app" auth="Container" type="javax.sql.DataSource"/>
//!   <Environment name="env" value="prod" type="java.lang.String"/>
//!   <WatchedResource>WEB-INF/web.xml</WatchedResource>
//! </Context>"#;
//! assert!(izanagi_kit::contextxml::detect(b));
//! let c = izanagi_kit::contextxml::ContextXml::parse(b).unwrap();
//! assert_eq!(c.elements, 3);
//! ```

/// Parsed context.xml summary.
#[derive(Debug, Clone)]
pub struct ContextXml {
    /// `<Context>` root present.
    pub has_root: bool,
    /// Recognized elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<Resource ")
        || tr.starts_with("<Resource>")
        || tr.starts_with("<ResourceLink ")
        || tr.starts_with("<ResourceLink>")
        || tr.starts_with("<ResourceParams")
        || tr.starts_with("<ResourceParams ")
        || tr.starts_with("<Environment ")
        || tr.starts_with("<Environment>")
        || tr.starts_with("<Valve ")
        || tr.starts_with("<Valve>")
        || tr.starts_with("<Realm ")
        || tr.starts_with("<Realm>")
        || tr.starts_with("<Loader ")
        || tr.starts_with("<Loader>")
        || tr.starts_with("<Manager ")
        || tr.starts_with("<Manager>")
        || tr.starts_with("<Parameter ")
        || tr.starts_with("<Parameter>")
        || tr.starts_with("<WatchedResource")
        || tr.starts_with("<JarScanner")
        || tr.starts_with("<JarScanFilter")
        || tr.starts_with("<CookieProcessor")
        || tr.starts_with("<Resources ")
        || tr.starts_with("<Resources>")
        || tr.starts_with("<PreResources")
        || tr.starts_with("<PostResources")
        || tr.starts_with("<Transaction ")
        || tr.starts_with("<Transaction>")
        || tr.starts_with("<InstanceListener")
        || tr.starts_with("<WrapperListener")
        || tr.starts_with("<NamingResources")
        || tr.starts_with("<NamingResources ")
        || tr.starts_with("<TestCredential")
        || tr.starts_with("<Store ")
        || tr.starts_with("<Store>")
        || tr.starts_with("<Cluster ")
        || tr.starts_with("<Cluster>")
        || tr.starts_with("<Host")
        || tr.starts_with("<Ejb ")
        || tr.starts_with("<Ejb>")
        || tr.starts_with("<LocalEjb")
}

fn ctx_root(tr: &str) -> bool {
    tr.starts_with("<Context ") || tr.starts_with("<Context>")
}

/// Detect a Tomcat context.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if ctx_root(tr) {
            root = true;
            continue;
        }
        if tr.starts_with("<!--") {
            continue;
        }
        if element(tr) {
            elems += 1;
        }
    }
    (root && elems >= 1) || elems >= 3
}

impl ContextXml {
    /// Count categories. Returns `None` when the input does not look like
    /// a context.xml descriptor.
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
            if ctx_root(tr) {
                c.has_root = true;
            } else if tr.starts_with("<!--") {
                c.comments += 1;
            } else if element(tr) {
                c.elements += 1;
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`ContextXml::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<ContextXml> {
    ContextXml::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<?xml version="1.0"?>
<Context path="/shop" docBase="shop.war" reloadable="false" crossContext="true">
  <!-- datasources -->
  <Resource name="jdbc/shop" auth="Container" type="javax.sql.DataSource"
            url="jdbc:postgresql://db/shop" maxTotal="20"/>
  <ResourceLink name="jdbc/global" global="jdbc/shared" type="javax.sql.DataSource"/>
  <Environment name="mode" value="prod" type="java.lang.String"/>
  <Parameter name="company" value="acme" override="false"/>
  <Valve className="org.apache.catalina.valves.AccessLogValve" directory="logs"/>
  <Realm className="org.apache.catalina.realm.JNDIRealm" connectionURL="ldap://x"/>
  <Loader delegate="true"/>
  <Manager className="org.apache.catalina.session.PersistentManager">
    <Store className="org.apache.catalina.session.FileStore" directory="/tmp/sessions"/>
  </Manager>
  <WatchedResource>WEB-INF/web.xml</WatchedResource>
  <WatchedResource>WEB-INF/tomcat-web.xml</WatchedResource>
  <JarScanner scanAllDirectories="true"/>
  <CookieProcessor sameSiteCookies="lax"/>
  <Transaction factory="com.atomikos.AtomikosDataSourceBean"/>
</Context>"#;
        assert!(detect(b));
        let c = ContextXml::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 14);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_generic_xml() {
        assert!(!detect(b"<context><resource name=\"x\"/></context>"));
        assert!(!detect(b"<settings><item>1</item></settings>"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"<!-- <Resource name=\"jdbc/x\"/> -->\n<!-- <Valve className=\"y\"/> -->"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(ContextXml::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"<Context>");
        assert!(!detect(&b));
    }
}
