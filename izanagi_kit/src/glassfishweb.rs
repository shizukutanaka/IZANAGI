//! GlassFish/Payara `glassfish-web.xml`（旧 `sun-web.xml`）記述子の
//! 検出と構造カウント。
//!
//! `<glassfish-web-app>`/`<sun-web-app>` ルート + `<context-root>`/
//! `<session-config>`/`<jsp-config>`/`<class-loader>`/
//! `<security-role-mapping>`/`<locale-charset-info>`/
//! `<parameter-encoding>` 等の要素を識別する。
//!
//! ```
//! let b = br#"<glassfish-web-app>
//!   <context-root>/shop</context-root>
//!   <session-config>
//!     <session-manager persistence-type="memory"/>
//!   </session-config>
//!   <jsp-config>
//!     <property name="keepgenerated" value="true"/>
//!   </jsp-config>
//! </glassfish-web-app>"#;
//! assert!(izanagi_kit::glassfishweb::detect(b));
//! let c = izanagi_kit::glassfishweb::GlassfishWeb::parse(b).unwrap();
//! assert_eq!(c.elements, 5);
//! ```

/// Parsed glassfish-web.xml summary.
#[derive(Debug, Clone)]
pub struct GlassfishWeb {
    /// `<glassfish-web-app>`/`<sun-web-app>` root present.
    pub has_root: bool,
    /// Recognized descriptor elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<context-root")
        || tr.starts_with("<session-config")
        || tr.starts_with("<session-manager")
        || tr.starts_with("<session-properties")
        || tr.starts_with("<cookie-properties")
        || tr.starts_with("<manager-properties")
        || tr.starts_with("<store-properties")
        || tr.starts_with("<jsp-config")
        || tr.starts_with("<property ")
        || tr.starts_with("<property>")
        || tr.starts_with("<class-loader")
        || tr.starts_with("<security-role-mapping")
        || tr.starts_with("<role-name")
        || tr.starts_with("<principal-name")
        || tr.starts_with("<group-name")
        || tr.starts_with("<locale-charset-info")
        || tr.starts_with("<locale-charset-map")
        || tr.starts_with("<parameter-encoding")
        || tr.starts_with("<form-hint-field")
        || tr.starts_with("<charset")
        || tr.starts_with("<locale")
        || tr.starts_with("<agent")
        || tr.starts_with("<description")
        || tr.starts_with("<resource-ref")
        || tr.starts_with("<resource-env-ref")
        || tr.starts_with("<ejb-ref")
        || tr.starts_with("<message-destination-ref")
        || tr.starts_with("<message-destination")
        || tr.starts_with("<webservice-endpoint")
        || tr.starts_with("<service-ref")
        || tr.starts_with("<port-component-name")
        || tr.starts_with("<endpoint-address-uri")
        || tr.starts_with("<wsdl-override")
        || tr.starts_with("<service-qname")
        || tr.starts_with("<res-ref-name")
        || tr.starts_with("<jndi-name")
        || tr.starts_with("<default-resource-principal")
        || tr.starts_with("<name>")
        || tr.starts_with("<password")
        || tr.starts_with("<ejb-ref-name")
        || tr.starts_with("<ejb-link")
        || tr.starts_with("<service-ref-name")
        || tr.starts_with("<message-destination-ref-name")
        || tr.starts_with("<linked-name")
        || tr.starts_with("<valve")
        || tr.starts_with("<idempotent-url-pattern")
        || tr.starts_with("<idempotent-url-pattern ")
        || tr.starts_with("<url-pattern")
        || tr.starts_with("<num-of-retries")
        || tr.starts_with("<interval")
        || tr.starts_with("<keep-state")
        || tr.starts_with("<cache")
        || tr.starts_with("<cache-mapping")
        || tr.starts_with("<dispatcher-name")
        || tr.starts_with("<timeout")
        || tr.starts_with("<http-field")
        || tr.starts_with("<key-field")
        || tr.starts_with("<cache-field")
        || tr.starts_with("<constraint-field")
        || tr.starts_with("<check-if-exists")
        || tr.starts_with("<check-if-modified")
        || tr.starts_with("<helper")
        || tr.starts_with("<property name=")
        || tr.starts_with("<servlet-name")
        || tr.starts_with("<access-log")
        || tr.starts_with("<format")
        || tr.starts_with("<encoding")
}

fn gf_root(tr: &str) -> bool {
    tr.starts_with("<glassfish-web-app ") || tr.starts_with("<glassfish-web-app>")
}

fn sun_root(tr: &str) -> bool {
    tr.starts_with("<sun-web-app ") || tr.starts_with("<sun-web-app>")
}

/// Detect a GlassFish glassfish-web.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if gf_root(tr) || sun_root(tr) {
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
    (root && elems >= 1) || elems >= 4
}

impl GlassfishWeb {
    /// Count categories. Returns `None` when the input does not look like
    /// a glassfish-web.xml descriptor.
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
            if gf_root(tr) || sun_root(tr) {
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

/// Convenience wrapper around [`GlassfishWeb::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<GlassfishWeb> {
    GlassfishWeb::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<?xml version="1.0"?>
<glassfish-web-app>
  <context-root>/shop</context-root>
  <session-config>
    <session-manager persistence-type="memory">
      <manager-properties>
        <property name="maxSessions" value="1000"/>
        <property name="sessionFilename" value="sessions"/>
      </manager-properties>
      <store-properties>
        <property name="reapIntervalSeconds" value="60"/>
      </store-properties>
    </session-manager>
    <cookie-properties>
      <property name="cookieMaxAgeSeconds" value="3600"/>
    </cookie-properties>
  </session-config>
  <jsp-config>
    <property name="keepgenerated" value="true"/>
    <property name="scratchdir" value="/tmp/jsp"/>
  </jsp-config>
  <class-loader delegate="true" extra-class-path="/opt/lib"/>
  <security-role-mapping>
    <role-name>admin</role-name>
    <principal-name>admin-user</principal-name>
    <group-name>admins</group-name>
  </security-role-mapping>
  <locale-charset-info default-locale="">
    <locale-charset-map locale="ja" charset="UTF-8"/>
    <parameter-encoding default-charset="UTF-8" form-hint-field="charset"/>
  </locale-charset-info>
  <resource-ref>
    <res-ref-name>jdbc/shop</res-ref-name>
    <jndi-name>jdbc/shop-pool</jndi-name>
    <default-resource-principal>
      <name>app</name>
      <password>secret</password>
    </default-resource-principal>
  </resource-ref>
  <idempotent-url-pattern url-pattern="/order/*" num-of-retries="3"/>
  <valve name="audit"/>
</glassfish-web-app>"#;
        assert!(detect(b));
        let c = GlassfishWeb::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 22);
    }

    #[test]
    fn detects_sun_web_app() {
        assert!(detect(
            b"<sun-web-app>\n<context-root>/x</context-root>\n</sun-web-app>"
        ));
    }

    #[test]
    fn rejects_generic_xml() {
        assert!(!detect(b"<glassfish><app/></glassfish>"));
        assert!(!detect(b"<web-app><servlet/></web-app>"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(GlassfishWeb::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"<glassfish-web-app>");
        assert!(!detect(&b));
    }
}
