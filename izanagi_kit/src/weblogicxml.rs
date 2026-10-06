//! WebLogic `weblogic.xml` デプロイメント記述子の検出と構造カウント。
//!
//! `<weblogic-web-app>` ルート + `<context-root>`/
//! `<virtual-directory-mapping>`/`<session-descriptor>`/
//! `<jsp-descriptor>`/`<container-descriptor>`/
//! `<security-role-assignment>`/`<resource-description>`/
//! `<work-manager>` 等の要素を識別する。
//!
//! ```
//! let b = br#"<weblogic-web-app xmlns="http://xmlns.oracle.com/weblogic/weblogic-web-app">
//!   <context-root>/shop</context-root>
//!   <session-descriptor>
//!     <timeout-secs>3600</timeout-secs>
//!   </session-descriptor>
//!   <jsp-descriptor>
//!     <keepgenerated>true</keepgenerated>
//!   </jsp-descriptor>
//! </weblogic-web-app>"#;
//! assert!(izanagi_kit::weblogicxml::detect(b));
//! let c = izanagi_kit::weblogicxml::WeblogicXml::parse(b).unwrap();
//! assert_eq!(c.elements, 5);
//! ```

/// Parsed weblogic.xml summary.
#[derive(Debug, Clone)]
pub struct WeblogicXml {
    /// `<weblogic-web-app>` root present.
    pub has_root: bool,
    /// Recognized descriptor elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<context-root")
        || tr.starts_with("<virtual-directory-mapping")
        || tr.starts_with("<local-path")
        || tr.starts_with("<url-pattern")
        || tr.starts_with("<session-descriptor")
        || tr.starts_with("<timeout-secs")
        || tr.starts_with("<invalidation-interval-secs")
        || tr.starts_with("<cookie-name")
        || tr.starts_with("<cookie-path")
        || tr.starts_with("<cookie-domain")
        || tr.starts_with("<cookie-secure")
        || tr.starts_with("<cookie-max-age-secs")
        || tr.starts_with("<cookies-enabled")
        || tr.starts_with("<url-rewriting-enabled")
        || tr.starts_with("<persistent-store-type")
        || tr.starts_with("<persistent-store-pool")
        || tr.starts_with("<persistent-store-table")
        || tr.starts_with("<persistent-store-cookie-name")
        || tr.starts_with("<persistent-store-dir")
        || tr.starts_with("<persistent-session-logging-enabled")
        || tr.starts_with("<sharing-enabled")
        || tr.starts_with("<http-proxy-caching-of-cookies")
        || tr.starts_with("<encode-session-id-in-query-params")
        || tr.starts_with("<monitoring-attribute-name")
        || tr.starts_with("<session-param")
        || tr.starts_with("<jsp-descriptor")
        || tr.starts_with("<keepgenerated")
        || tr.starts_with("<pagecheckseconds")
        || tr.starts_with("<verbose")
        || tr.starts_with("<workingdir")
        || tr.starts_with("<printnulls")
        || tr.starts_with("<backwardcompatible")
        || tr.starts_with("<encoding")
        || tr.starts_with("<exactmapping")
        || tr.starts_with("<defaultfilename")
        || tr.starts_with("<rtexprvalue")
        || tr.starts_with("<compileall")
        || tr.starts_with("<jspparam")
        || tr.starts_with("<container-descriptor")
        || tr.starts_with("<prefer-web-inf-classes")
        || tr.starts_with("<prefer-application-packages")
        || tr.starts_with("<prefer-application-resources")
        || tr.starts_with("<package-name")
        || tr.starts_with("<resource-name")
        || tr.starts_with("<prefer-application")
        || tr.starts_with("<show-archived-real-path-enabled")
        || tr.starts_with("<redirect-content")
        || tr.starts_with("<redirect-with-absolute-url")
        || tr.starts_with("<index-directory-enabled")
        || tr.starts_with("<index-directory-sort-by")
        || tr.starts_with("<servlet-reload-check-secs")
        || tr.starts_with("<resource-reload-check-secs")
        || tr.starts_with("<min-native-file-size")
        || tr.starts_with("<disable-implicit-servlet-mappings")
        || tr.starts_with("<tempdir")
        || tr.starts_with("<optimistic-serialization")
        || tr.starts_with("<require-admin-traffic")
        || tr.starts_with("<access-logging-disabled")
        || tr.starts_with("<security-role-assignment")
        || tr.starts_with("<role-name")
        || tr.starts_with("<principal-name")
        || tr.starts_with("<externally-defined")
        || tr.starts_with("<global-role")
        || tr.starts_with("<resource-description")
        || tr.starts_with("<res-ref-name")
        || tr.starts_with("<jndi-name")
        || tr.starts_with("<resource-env-description")
        || tr.starts_with("<resource-env-ref-name")
        || tr.starts_with("<ejb-reference-description")
        || tr.starts_with("<ejb-ref-name")
        || tr.starts_with("<service-reference-description")
        || tr.starts_with("<service-ref-name")
        || tr.starts_with("<wsdl-url")
        || tr.starts_with("<message-destination-descriptor")
        || tr.starts_with("<message-destination-name")
        || tr.starts_with("<work-manager")
        || tr.starts_with("<name>")
        || tr.starts_with("<response-time-request-class")
        || tr.starts_with("<fair-share-request-class")
        || tr.starts_with("<context-request-class")
        || tr.starts_with("<request-class-name")
        || tr.starts_with("<min-threads-constraint")
        || tr.starts_with("<max-threads-constraint")
        || tr.starts_with("<capacity")
        || tr.starts_with("<count")
        || tr.starts_with("<run-as-principal-name")
        || tr.starts_with("<run-as-role-assignment")
        || tr.starts_with("<ready-app-registration")
        || tr.starts_with("<ready-registration-delay-secs")
        || tr.starts_with("<charset-params")
        || tr.starts_with("<input-charset")
        || tr.starts_with("<resource-path")
        || tr.starts_with("<charset")
        || tr.starts_with("<virtual-host-name")
        || tr.starts_with("<session-id-length")
        || tr.starts_with("<auth-filter")
        || tr.starts_with("<login")
        || tr.starts_with("<auth-cookie")
        || tr.starts_with("<async-descriptor")
        || tr.starts_with("<async-work-manager")
        || tr.starts_with("<async-timeout-secs")
        || tr.starts_with("<dispatch-policy")
        || tr.starts_with("<logging")
        || tr.starts_with("<log-filename")
        || tr.starts_with("<library-ref")
        || tr.starts_with("<library-name")
        || tr.starts_with("<specification-version")
        || tr.starts_with("<implementation-version")
        || tr.starts_with("<exact-match")
        || tr.starts_with("<description")
}

/// Detect a WebLogic weblogic.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<weblogic-web-app ") || tr.starts_with("<weblogic-web-app>") {
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

impl WeblogicXml {
    /// Count categories. Returns `None` when the input does not look like
    /// a weblogic.xml descriptor.
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
            if tr.starts_with("<weblogic-web-app ") || tr.starts_with("<weblogic-web-app>") {
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

/// Convenience wrapper around [`WeblogicXml::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<WeblogicXml> {
    WeblogicXml::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<?xml version="1.0"?>
<weblogic-web-app xmlns="http://xmlns.oracle.com/weblogic/weblogic-web-app">
  <context-root>/shop</context-root>
  <virtual-directory-mapping>
    <local-path>/var/static</local-path>
    <url-pattern>/static/*</url-pattern>
  </virtual-directory-mapping>
  <session-descriptor>
    <timeout-secs>3600</timeout-secs>
    <cookie-name>JSESSION</cookie-name>
    <cookies-enabled>true</cookies-enabled>
    <url-rewriting-enabled>false</url-rewriting-enabled>
    <persistent-store-type>jdbc</persistent-store-type>
    <persistent-store-pool>pool1</persistent-store-pool>
  </session-descriptor>
  <jsp-descriptor>
    <keepgenerated>true</keepgenerated>
    <pagecheckseconds>60</pagecheckseconds>
    <verbose>true</verbose>
    <encoding>UTF-8</encoding>
  </jsp-descriptor>
  <container-descriptor>
    <prefer-web-inf-classes>false</prefer-web-inf-classes>
    <prefer-application-packages>
      <package-name>com.fasterxml.*</package-name>
    </prefer-application-packages>
    <servlet-reload-check-secs>-1</servlet-reload-check-secs>
  </container-descriptor>
  <security-role-assignment>
    <role-name>admin</role-name>
    <principal-name>admins</principal-name>
    <externally-defined/>
  </security-role-assignment>
  <resource-description>
    <res-ref-name>jdbc/shop</res-ref-name>
    <jndi-name>jdbc.shop</jndi-name>
  </resource-description>
  <work-manager>
    <name>fast</name>
    <fair-share-request-class>
      <name>fs</name>
      <fair-share>50</fair-share>
    </fair-share-request-class>
    <min-threads-constraint>
      <name>min</name>
      <count>4</count>
    </min-threads-constraint>
  </work-manager>
  <library-ref>
    <library-name>common-lib</library-name>
    <exact-match>true</exact-match>
  </library-ref>
</weblogic-web-app>"#;
        assert!(detect(b));
        let c = WeblogicXml::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 28);
    }

    #[test]
    fn rejects_generic_xml() {
        assert!(!detect(b"<weblogic><app/></weblogic>"));
        assert!(!detect(b"<web-app><servlet/></web-app>"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"<!-- <context-root>/x</context-root> -->\n<!-- <timeout-secs>1</timeout-secs> -->"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(WeblogicXml::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"<weblogic-web-app>");
        assert!(!detect(&b));
    }
}
