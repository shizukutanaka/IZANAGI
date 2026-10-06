//! JSF `faces-config.xml` の検出と構造カウント。
//!
//! `<faces-config>` ルート + `<managed-bean>`/`<navigation-rule>`/
//! `<converter>`/`<validator>`/`<render-kit>`/`<application>`/
//! `<lifecycle>`/`<factory>`/`<component>` 等の要素を識別する。
//!
//! ```
//! let b = br#"<faces-config xmlns="https://jakarta.ee/xml/ns/jakartaee" version="4.0">
//!   <managed-bean>
//!     <managed-bean-name>user</managed-bean-name>
//!     <managed-bean-class>x.UserBean</managed-bean-class>
//!     <managed-bean-scope>session</managed-bean-scope>
//!   </managed-bean>
//!   <navigation-rule>
//!     <from-view-id>/login.xhtml</from-view-id>
//!     <navigation-case>
//!       <from-outcome>ok</from-outcome>
//!       <to-view-id>/home.xhtml</to-view-id>
//!     </navigation-case>
//!   </navigation-rule>
//! </faces-config>"#;
//! assert!(izanagi_kit::facesconfig::detect(b));
//! let c = izanagi_kit::facesconfig::FacesConfig::parse(b).unwrap();
//! assert_eq!(c.elements, 9);
//! ```

/// Parsed faces-config.xml summary.
#[derive(Debug, Clone)]
pub struct FacesConfig {
    /// `<faces-config>` root present.
    pub has_root: bool,
    /// Recognized descriptor elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<managed-bean>")
        || tr.starts_with("<managed-bean ")
        || tr.starts_with("<managed-bean-name")
        || tr.starts_with("<managed-bean-class")
        || tr.starts_with("<managed-bean-scope")
        || tr.starts_with("<managed-property")
        || tr.starts_with("<property-name")
        || tr.starts_with("<property-class")
        || tr.starts_with("<list-entries")
        || tr.starts_with("<map-entries")
        || tr.starts_with("<key-class")
        || tr.starts_with("<value-class")
        || tr.starts_with("<map-entry")
        || tr.starts_with("<key>")
        || tr.starts_with("<value>")
        || tr.starts_with("<null-value")
        || tr.starts_with("<navigation-rule")
        || tr.starts_with("<navigation-case")
        || tr.starts_with("<from-view-id")
        || tr.starts_with("<to-view-id")
        || tr.starts_with("<from-action")
        || tr.starts_with("<from-outcome")
        || tr.starts_with("<redirect")
        || tr.starts_with("<view-param")
        || tr.starts_with("<if")
        || tr.starts_with("<converter>")
        || tr.starts_with("<converter ")
        || tr.starts_with("<converter-id")
        || tr.starts_with("<converter-class")
        || tr.starts_with("<converter-for-class")
        || tr.starts_with("<attribute")
        || tr.starts_with("<attribute-name")
        || tr.starts_with("<attribute-class")
        || tr.starts_with("<property")
        || tr.starts_with("<validator>")
        || tr.starts_with("<validator ")
        || tr.starts_with("<validator-id")
        || tr.starts_with("<validator-class")
        || tr.starts_with("<render-kit")
        || tr.starts_with("<renderer")
        || tr.starts_with("<renderer-type")
        || tr.starts_with("<renderer-class")
        || tr.starts_with("<component-family")
        || tr.starts_with("<component>")
        || tr.starts_with("<component ")
        || tr.starts_with("<component-type")
        || tr.starts_with("<component-class")
        || tr.starts_with("<application>")
        || tr.starts_with("<application ")
        || tr.starts_with("<action-listener")
        || tr.starts_with("<navigation-handler")
        || tr.starts_with("<view-handler")
        || tr.starts_with("<state-manager")
        || tr.starts_with("<el-resolver")
        || tr.starts_with("<expression-factory")
        || tr.starts_with("<resource-handler")
        || tr.starts_with("<locale-config")
        || tr.starts_with("<default-locale")
        || tr.starts_with("<supported-locale")
        || tr.starts_with("<resource-bundle")
        || tr.starts_with("<base-name")
        || tr.starts_with("<var")
        || tr.starts_with("<lifecycle>")
        || tr.starts_with("<lifecycle ")
        || tr.starts_with("<phase-listener")
        || tr.starts_with("<factory>")
        || tr.starts_with("<factory ")
        || tr.starts_with("<faces-config-factory")
        || tr.starts_with("<application-factory")
        || tr.starts_with("<faces-context-factory")
        || tr.starts_with("<lifecycle-factory")
        || tr.starts_with("<render-kit-factory")
        || tr.starts_with("<exception-handler-factory")
        || tr.starts_with("<external-context-factory")
        || tr.starts_with("<partial-view-context-factory")
        || tr.starts_with("<tag-handler-delegate-factory")
        || tr.starts_with("<visit-context-factory")
        || tr.starts_with("<flow-handler-factory")
        || tr.starts_with("<client-window-factory")
        || tr.starts_with("<flash-factory")
        || tr.starts_with("<search-expression-context-factory")
        || tr.starts_with("<referenced-bean")
        || tr.starts_with("<referenced-bean-name")
        || tr.starts_with("<referenced-bean-class")
        || tr.starts_with("<ordering")
        || tr.starts_with("<before")
        || tr.starts_with("<after")
        || tr.starts_with("<others")
        || tr.starts_with("<name>")
        || tr.starts_with("<absolute-ordering")
        || tr.starts_with("<flow-definition")
        || tr.starts_with("<flow-return")
        || tr.starts_with("<from-outcome")
        || tr.starts_with("<protected-views")
        || tr.starts_with("<url-pattern")
        || tr.starts_with("<system-event-listener")
        || tr.starts_with("<system-event-listener-class")
        || tr.starts_with("<system-event-class")
        || tr.starts_with("<source-class")
        || tr.starts_with("<behavior")
        || tr.starts_with("<behavior-id")
        || tr.starts_with("<behavior-class")
        || tr.starts_with("<contract-mapping")
        || tr.starts_with("<url-pattern")
        || tr.starts_with("<contracts")
}

/// Detect a JSF faces-config.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<faces-config ") || tr.starts_with("<faces-config>") {
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
    (root && elems >= 1) || elems >= 5
}

impl FacesConfig {
    /// Count categories. Returns `None` when the input does not look like
    /// a faces-config.xml descriptor.
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
            if tr.starts_with("<faces-config ") || tr.starts_with("<faces-config>") {
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

/// Convenience wrapper around [`FacesConfig::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<FacesConfig> {
    FacesConfig::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<?xml version="1.0"?>
<faces-config xmlns="https://jakarta.ee/xml/ns/jakartaee" version="4.0">
  <application>
    <locale-config>
      <default-locale>en</default-locale>
      <supported-locale>ja</supported-locale>
    </locale-config>
    <resource-bundle>
      <base-name>x.messages</base-name>
      <var>msg</var>
    </resource-bundle>
  </application>
  <managed-bean>
    <managed-bean-name>cart</managed-bean-name>
    <managed-bean-class>x.CartBean</managed-bean-class>
    <managed-bean-scope>session</managed-bean-scope>
    <managed-property>
      <property-name>catalog</property-name>
      <value>#{catalog}</value>
    </managed-property>
  </managed-bean>
  <navigation-rule>
    <from-view-id>/list.xhtml</from-view-id>
    <navigation-case>
      <from-action>#{cart.add}</from-action>
      <from-outcome>ok</from-outcome>
      <to-view-id>/confirm.xhtml</to-view-id>
      <redirect/>
    </navigation-case>
  </navigation-rule>
  <converter>
    <converter-id>phone</converter-id>
    <converter-class>x.PhoneConverter</converter-class>
  </converter>
  <validator>
    <validator-id>email</validator-id>
    <validator-class>x.EmailValidator</validator-class>
  </validator>
  <lifecycle>
    <phase-listener>x.AuditPhaseListener</phase-listener>
  </lifecycle>
</faces-config>"#;
        assert!(detect(b));
        let c = FacesConfig::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 20);
    }

    #[test]
    fn rejects_generic_xml() {
        assert!(!detect(b"<faces><config><x/></config></faces>"));
        assert!(!detect(b"<config><value>1</value></config>"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"<!-- <managed-bean-name>x</managed-bean-name> -->\n<!-- <to-view-id>/x</to-view-id> -->"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(FacesConfig::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"<faces-config>");
        assert!(!detect(&b));
    }
}
