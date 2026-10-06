//! JBoss/WildFly `jboss-web.xml` 記述子の検出と構造カウント。
//!
//! `<jboss-web>` ルート + `<context-root>`/`<security-domain>`/
//! `<virtual-host>`/`<valve>`/`<resource-ref>`/`<ejb-ref>`/
//! `<servlet>`/`<replication-config>`/`<max-active-sessions>` 等の
//! 要素を識別する。
//!
//! ```
//! let b = br#"<jboss-web>
//!   <context-root>/shop</context-root>
//!   <security-domain>java:/jaas/shop</security-domain>
//!   <virtual-host>shop.local</virtual-host>
//! </jboss-web>"#;
//! assert!(izanagi_kit::jbossweb::detect(b));
//! let c = izanagi_kit::jbossweb::JbossWeb::parse(b).unwrap();
//! assert_eq!(c.elements, 3);
//! ```

/// Parsed jboss-web.xml summary.
#[derive(Debug, Clone)]
pub struct JbossWeb {
    /// `<jboss-web>` root present.
    pub has_root: bool,
    /// Recognized descriptor elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<context-root")
        || tr.starts_with("<security-domain")
        || tr.starts_with("<virtual-host")
        || tr.starts_with("<valve")
        || tr.starts_with("<class-name")
        || tr.starts_with("<param")
        || tr.starts_with("<param-name")
        || tr.starts_with("<param-value")
        || tr.starts_with("<resource-ref")
        || tr.starts_with("<resource-env-ref")
        || tr.starts_with("<res-ref-name")
        || tr.starts_with("<jndi-name")
        || tr.starts_with("<ejb-ref")
        || tr.starts_with("<ejb-local-ref")
        || tr.starts_with("<ejb-ref-name")
        || tr.starts_with("<ejb-link")
        || tr.starts_with("<servlet>")
        || tr.starts_with("<servlet ")
        || tr.starts_with("<servlet-name")
        || tr.starts_with("<run-as-principal")
        || tr.starts_with("<security-role")
        || tr.starts_with("<role-name")
        || tr.starts_with("<principal-name")
        || tr.starts_with("<security-role-ref")
        || tr.starts_with("<role-link")
        || tr.starts_with("<replication-config")
        || tr.starts_with("<replication-trigger")
        || tr.starts_with("<replication-granularity")
        || tr.starts_with("<replication-field-batch-mode")
        || tr.starts_with("<snapshot-mode")
        || tr.starts_with("<snapshot-interval")
        || tr.starts_with("<session-notification-policy")
        || tr.starts_with("<max-active-sessions")
        || tr.starts_with("<passivation-config")
        || tr.starts_with("<use-session-passivation")
        || tr.starts_with("<passivation-min-idle-time")
        || tr.starts_with("<passivation-max-idle-time")
        || tr.starts_with("<distinct-name")
        || tr.starts_with("<depends")
        || tr.starts_with("<jboss-web-ejb-ref")
        || tr.starts_with("<description")
        || tr.starts_with("<disable-audit")
        || tr.starts_with("<webservice-description")
        || tr.starts_with("<webservice-description-name")
        || tr.starts_with("<wsdl-publish-location")
        || tr.starts_with("<port-component")
        || tr.starts_with("<port-component-name")
        || tr.starts_with("<port-component-uri")
        || tr.starts_with("<auth-method")
        || tr.starts_with("<transport-guarantee")
        || tr.starts_with("<secure-wsdl-access")
        || tr.starts_with("<server-identity")
        || tr.starts_with("<mime-mapping")
        || tr.starts_with("<extension")
        || tr.starts_with("<mime-type")
        || tr.starts_with("<annotation")
        || tr.starts_with("<class")
        || tr.starts_with("<method")
        || tr.starts_with("<method-name")
        || tr.starts_with("<method-params")
        || tr.starts_with("<method-param")
        || tr.starts_with("<exception")
        || tr.starts_with("<exception-class")
        || tr.starts_with("<ejb-name")
        || tr.starts_with("<jndi-binding")
        || tr.starts_with("<local-jndi-name")
        || tr.starts_with("<audit")
        || tr.starts_with("<shared-session-config")
        || tr.starts_with("<version")
        || tr.starts_with("<distributable")
        || tr.starts_with("<overlay")
        || tr.starts_with("<symbolic-linking")
        || tr.starts_with("<disable-cross-context")
        || tr.starts_with("<enable-websockets")
        || tr.starts_with("<http-method")
        || tr.starts_with("<deny-uncovered-http-methods")
        || tr.starts_with("<disable-proxy-caching")
        || tr.starts_with("<default-encodable-path-elements")
        || tr.starts_with("<encoded-context-path")
        || tr.starts_with("<clustering")
        || tr.starts_with("<replication-granularity")
}

/// Detect a JBoss jboss-web.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<jboss-web ") || tr.starts_with("<jboss-web>") {
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

impl JbossWeb {
    /// Count categories. Returns `None` when the input does not look like
    /// a jboss-web.xml descriptor.
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
            if tr.starts_with("<jboss-web ") || tr.starts_with("<jboss-web>") {
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

/// Convenience wrapper around [`JbossWeb::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<JbossWeb> {
    JbossWeb::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<?xml version="1.0"?>
<!-- jboss web descriptor -->
<jboss-web>
  <context-root>/shop</context-root>
  <security-domain>java:/jaas/shop</security-domain>
  <virtual-host>shop.local</virtual-host>
  <distinct-name>shop</distinct-name>
  <valve>
    <class-name>org.jboss.resteasy.plugins.server.servlet.ResteasyBootstrap</class-name>
    <param>
      <param-name>resteasy.scan</param-name>
      <param-value>true</param-value>
    </param>
  </valve>
  <resource-ref>
    <res-ref-name>jdbc/shop</res-ref-name>
    <jndi-name>java:jboss/datasources/ShopDS</jndi-name>
  </resource-ref>
  <ejb-ref>
    <ejb-ref-name>ejb/cart</ejb-ref-name>
    <jndi-name>java:app/cart/CartBean</jndi-name>
  </ejb-ref>
  <servlet>
    <servlet-name>api</servlet-name>
    <run-as-principal>svc</run-as-principal>
  </servlet>
  <security-role>
    <role-name>admin</role-name>
    <principal-name>admins</principal-name>
  </security-role>
  <replication-config>
    <replication-trigger>SET_AND_NON_PRIMITIVE_GET</replication-trigger>
    <replication-granularity>SESSION</replication-granularity>
    <snapshot-mode>INSTANT</snapshot-mode>
    <snapshot-interval>500</snapshot-interval>
  </replication-config>
  <max-active-sessions>10000</max-active-sessions>
  <passivation-config>
    <use-session-passivation>true</use-session-passivation>
    <passivation-min-idle-time>60</passivation-min-idle-time>
    <passivation-max-idle-time>600</passivation-max-idle-time>
  </passivation-config>
  <webservice-description>
    <webservice-description-name>ShopWS</webservice-description-name>
    <wsdl-publish-location>/wsdl/shop.wsdl</wsdl-publish-location>
    <port-component>
      <port-component-name>ShopPort</port-component-name>
      <port-component-uri>/shop</port-component-uri>
      <auth-method>BASIC</auth-method>
      <transport-guarantee>CONFIDENTIAL</transport-guarantee>
      <secure-wsdl-access>true</secure-wsdl-access>
    </port-component>
  </webservice-description>
  <depends>jboss.web.deployment:war=/base</depends>
</jboss-web>"#;
        assert!(detect(b));
        let c = JbossWeb::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 28);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_generic_xml() {
        assert!(!detect(b"<jboss><web/></jboss>"));
        assert!(!detect(b"<web-app><servlet/></web-app>"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"<!-- <context-root>/x</context-root> -->\n<!-- <security-domain>y</security-domain> -->"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(JbossWeb::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"<jboss-web>");
        assert!(!detect(&b));
    }
}
