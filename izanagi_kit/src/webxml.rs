//! Jakarta EE `web.xml` デプロイメント記述子の検出と構造カウント。
//!
//! `<web-app>` ルート要素 + `<servlet>`/`<servlet-mapping>`/`<filter>`/
//! `<listener>`/`<welcome-file-list>`/`<session-config>`/`<context-param>`/
//! `<error-page>`/`<security-constraint>`/`<login-config>` 等の要素を
//! 識別する。
//!
//! ```
//! let b = br#"<?xml version="1.0"?>
//! <web-app xmlns="https://jakarta.ee/xml/ns/jakartaee" version="6.0">
//!   <display-name>app</display-name>
//!   <servlet>
//!     <servlet-name>hello</servlet-name>
//!     <servlet-class>x.Hello</servlet-class>
//!   </servlet>
//!   <servlet-mapping>
//!     <servlet-name>hello</servlet-name>
//!     <url-pattern>/*</url-pattern>
//!   </servlet-mapping>
//!   <welcome-file-list>
//!     <welcome-file>index.html</welcome-file>
//!   </welcome-file-list>
//! </web-app>"#;
//! assert!(izanagi_kit::webxml::detect(b));
//! let c = izanagi_kit::webxml::WebXml::parse(b).unwrap();
//! assert_eq!(c.elements, 9);
//! ```

/// Parsed web.xml summary.
#[derive(Debug, Clone)]
pub struct WebXml {
    /// `<web-app>` root present.
    pub has_root: bool,
    /// Recognized descriptor elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<display-name")
        || tr.starts_with("<servlet>")
        || tr.starts_with("<servlet ")
        || tr.starts_with("<servlet-name")
        || tr.starts_with("<servlet-class")
        || tr.starts_with("<servlet-mapping")
        || tr.starts_with("<jsp-file")
        || tr.starts_with("<init-param")
        || tr.starts_with("<param-name")
        || tr.starts_with("<param-value")
        || tr.starts_with("<url-pattern")
        || tr.starts_with("<filter>")
        || tr.starts_with("<filter ")
        || tr.starts_with("<filter-name")
        || tr.starts_with("<filter-class")
        || tr.starts_with("<filter-mapping")
        || tr.starts_with("<listener>")
        || tr.starts_with("<listener ")
        || tr.starts_with("<listener-class")
        || tr.starts_with("<welcome-file-list")
        || tr.starts_with("<welcome-file")
        || tr.starts_with("<session-config")
        || tr.starts_with("<session-timeout")
        || tr.starts_with("<cookie-config")
        || tr.starts_with("<tracking-mode")
        || tr.starts_with("<context-param")
        || tr.starts_with("<error-page")
        || tr.starts_with("<error-code")
        || tr.starts_with("<exception-type")
        || tr.starts_with("<location")
        || tr.starts_with("<security-constraint")
        || tr.starts_with("<web-resource-collection")
        || tr.starts_with("<web-resource-name")
        || tr.starts_with("<http-method")
        || tr.starts_with("<auth-constraint")
        || tr.starts_with("<role-name")
        || tr.starts_with("<user-data-constraint")
        || tr.starts_with("<transport-guarantee")
        || tr.starts_with("<security-role")
        || tr.starts_with("<security-role-ref")
        || tr.starts_with("<role-link")
        || tr.starts_with("<login-config")
        || tr.starts_with("<auth-method")
        || tr.starts_with("<realm-name")
        || tr.starts_with("<form-login-config")
        || tr.starts_with("<form-login-page")
        || tr.starts_with("<form-error-page")
        || tr.starts_with("<mime-mapping")
        || tr.starts_with("<extension")
        || tr.starts_with("<mime-type")
        || tr.starts_with("<env-entry")
        || tr.starts_with("<env-entry-name")
        || tr.starts_with("<env-entry-value")
        || tr.starts_with("<env-entry-type")
        || tr.starts_with("<ejb-ref")
        || tr.starts_with("<ejb-local-ref")
        || tr.starts_with("<ejb-ref-name")
        || tr.starts_with("<ejb-ref-type")
        || tr.starts_with("<ejb-link")
        || tr.starts_with("<home")
        || tr.starts_with("<remote")
        || tr.starts_with("<local")
        || tr.starts_with("<local-home")
        || tr.starts_with("<resource-ref")
        || tr.starts_with("<resource-env-ref")
        || tr.starts_with("<res-ref-name")
        || tr.starts_with("<res-type")
        || tr.starts_with("<res-auth")
        || tr.starts_with("<res-sharing-scope")
        || tr.starts_with("<message-destination-ref")
        || tr.starts_with("<message-destination")
        || tr.starts_with("<message-destination-link")
        || tr.starts_with("<message-destination-name")
        || tr.starts_with("<jsp-config")
        || tr.starts_with("<jsp-property-group")
        || tr.starts_with("<taglib")
        || tr.starts_with("<taglib-uri")
        || tr.starts_with("<taglib-location")
        || tr.starts_with("<include-prelude")
        || tr.starts_with("<include-coda")
        || tr.starts_with("<el-ignored")
        || tr.starts_with("<page-encoding")
        || tr.starts_with("<scripting-invalid")
        || tr.starts_with("<is-xml")
        || tr.starts_with("<deferred-syntax-allowed-as-literal")
        || tr.starts_with("<trim-directive-whitespaces")
        || tr.starts_with("<default-content-type")
        || tr.starts_with("<buffer")
        || tr.starts_with("<deny-uncovered-http-methods")
        || tr.starts_with("<request-character-encoding")
        || tr.starts_with("<response-character-encoding")
        || tr.starts_with("<locale-encoding-mapping-list")
        || tr.starts_with("<locale-encoding-mapping")
        || tr.starts_with("<locale")
        || tr.starts_with("<encoding")
        || tr.starts_with("<post-construct")
        || tr.starts_with("<pre-destroy")
        || tr.starts_with("<multipart-config")
        || tr.starts_with("<max-file-size")
        || tr.starts_with("<max-request-size")
        || tr.starts_with("<file-size-threshold")
        || tr.starts_with("<distributable")
        || tr.starts_with("<absolute-ordering")
        || tr.starts_with("<name>")
        || tr.starts_with("<icon")
        || tr.starts_with("<small-icon")
        || tr.starts_with("<large-icon")
        || tr.starts_with("<description")
        || tr.starts_with("<async-supported")
        || tr.starts_with("<load-on-startup")
        || tr.starts_with("<run-as")
        || tr.starts_with("<enabled")
        || tr.starts_with("<ordering")
        || tr.starts_with("<before")
        || tr.starts_with("<after")
        || tr.starts_with("<others")
}

/// Detect a Jakarta EE web.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<web-app ") || tr.starts_with("<web-app>") {
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
    (root && elems >= 2) || elems >= 5
}

impl WebXml {
    /// Count categories. Returns `None` when the input does not look like
    /// a web.xml descriptor.
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
            if tr.starts_with("<web-app ") || tr.starts_with("<web-app>") {
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

/// Convenience wrapper around [`WebXml::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<WebXml> {
    WebXml::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<?xml version="1.0" encoding="UTF-8"?>
<!-- descriptor -->
<web-app xmlns="https://jakarta.ee/xml/ns/jakartaee" version="6.0">
  <display-name>my-app</display-name>
  <context-param>
    <param-name>env</param-name>
    <param-value>prod</param-value>
  </context-param>
  <filter>
    <filter-name>auth</filter-name>
    <filter-class>x.Auth</filter-class>
  </filter>
  <filter-mapping>
    <filter-name>auth</filter-name>
    <url-pattern>/*</url-pattern>
  </filter-mapping>
  <listener>
    <listener-class>x.Boot</listener-class>
  </listener>
  <servlet>
    <servlet-name>api</servlet-name>
    <servlet-class>x.Api</servlet-class>
    <load-on-startup>1</load-on-startup>
  </servlet>
  <servlet-mapping>
    <servlet-name>api</servlet-name>
    <url-pattern>/api/*</url-pattern>
  </servlet-mapping>
  <session-config>
    <session-timeout>30</session-timeout>
  </session-config>
  <welcome-file-list>
    <welcome-file>index.jsp</welcome-file>
  </welcome-file-list>
  <error-page>
    <error-code>404</error-code>
    <location>/404.html</location>
  </error-page>
  <security-constraint>
    <web-resource-collection>
      <web-resource-name>all</web-resource-name>
      <url-pattern>/*</url-pattern>
    </web-resource-collection>
    <auth-constraint>
      <role-name>admin</role-name>
    </auth-constraint>
  </security-constraint>
  <login-config>
    <auth-method>FORM</auth-method>
    <form-login-config>
      <form-login-page>/login.jsp</form-login-page>
      <form-error-page>/error.jsp</form-error-page>
    </form-login-config>
  </login-config>
  <mime-mapping>
    <extension>json</extension>
    <mime-type>application/json</mime-type>
  </mime-mapping>
</web-app>"#;
        assert!(detect(b));
        let c = WebXml::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 35);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_plain_html() {
        assert!(!detect(b"<html><body><div>hello</div></body></html>"));
        assert!(!detect(b"<webconfig><x/></webconfig>"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"<!-- <servlet-name>x</servlet-name> -->\n<!-- <url-pattern>/x</url-pattern> -->"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(WebXml::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"<web-app>");
        assert!(!detect(&b));
    }
}
