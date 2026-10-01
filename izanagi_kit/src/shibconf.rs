//! Shibboleth IdP/SP XML config census.
//!
//! Covers `attribute-filter.xml` (`<AttributeFilterPolicy>` +
//! `<AttributeRule>` + `<PermitValueRule>`/`<DenyValueRule>`),
//! `attribute-resolver.xml` (`<AttributeDefinition>` + `<DataConnector>` +
//! `<InputAttributeDefinition>`), `relying-party.xml`
//! (`<RelyingParty>` + `<MetadataProvider>` + `<ProfileRequest>`),
//! `shibboleth2.xml` (`<ApplicationDefaults>` + `<SSO>` + `<SessionInitiator>`).
//!
//! ```rust
//! let x = r#"<afp:AttributeFilterPolicyGroup><AttributeFilterPolicy id="a"><AttributeRule attributeID="e"><PermitValueRule xsi:type="ANY"/></AttributeRule></AttributeFilterPolicy></afp:AttributeFilterPolicyGroup>"#;
//! let c = izanagi_kit::shibconf::Shibconf::parse(x.as_bytes()).unwrap();
//! assert_eq!(c.policies, 1);
//! assert_eq!(c.rules, 1);
//! ```

/// Shibboleth config census.
#[derive(Debug, Clone)]
pub struct Shibconf {
    /// `<AttributeFilterPolicy>` entries.
    pub policies: usize,
    /// `<AttributeRule>` entries.
    pub rules: usize,
    /// `<AttributeDefinition>` entries.
    pub definitions: usize,
    /// `<DataConnector>` entries.
    pub connectors: usize,
    /// `<MetadataProvider>`/`<RelyingParty>`/`ProfileRequest` entries.
    pub relying: usize,
    /// `<PermitValueRule>`/`<PermitOR` entries.
    pub permits: usize,
    /// `<DenyValueRule>`/`<DenyRule` entries.
    pub deny: usize,
}

/// Whether the buffer looks like Shibboleth XML config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("AttributeFilterPolicyGroup")
        || t.contains("AttributeFilterPolicy")
        || t.contains("AttributeResolver")
        || t.contains("RelyingPartyGroup")
        || t.contains("ApplicationDefaults")
        || t.contains("shibboleth2")
        || t.contains("SessionInitiator")
        || t.contains("DataConnector")
}

impl Shibconf {
    /// Parse Shibboleth XML into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let m = |name: &str| t.matches(&["<", name].concat()).count();
        Some(Self {
            policies: m("AttributeFilterPolicy") - m("AttributeFilterPolicyGroup"),
            rules: m("AttributeRule"),
            definitions: m("AttributeDefinition"),
            connectors: m("DataConnector"),
            relying: m("MetadataProvider")
                + m("RelyingParty")
                + m("ProfileRequest")
                + m("ApplicationDefaults")
                + m("SessionInitiator"),
            permits: m("PermitValueRule") + m("PermitOR"),
            deny: m("DenyValueRule") + m("DenyRule"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_filter() {
        let b = concat!(
            "<afp:AttributeFilterPolicyGroup xmlns:afp=\"urn:mace:shibboleth:2.0:afp\">\n",
            "  <AttributeFilterPolicy id=\"release\">\n",
            "    <AttributeRule attributeID=\"eduPersonPrincipalName\">\n",
            "      <PermitValueRule xsi:type=\"ANY\"/>\n",
            "    </AttributeRule>\n",
            "    <AttributeRule attributeID=\"mail\">\n",
            "      <PermitValueRule xsi:type=\"AttributeInMetadata\"/>\n",
            "    </AttributeRule>\n",
            "  </AttributeFilterPolicy>\n",
            "  <AttributeFilterPolicy id=\"deny\">\n",
            "    <AttributeRule attributeID=\"password\">\n",
            "      <DenyValueRule xsi:type=\"ANY\"/>\n",
            "    </AttributeRule>\n",
            "  </AttributeFilterPolicy>\n",
            "</afp:AttributeFilterPolicyGroup>\n",
        );
        let c = Shibconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.policies, 2);
        assert_eq!(c.rules, 3);
        assert_eq!(c.permits, 2);
        assert_eq!(c.deny, 1);
    }

    #[test]
    fn parses_resolver() {
        let b = concat!(
            "<resolver:AttributeResolver xmlns:resolver=\"urn:mace:shibboleth:2.0:resolver\">\n",
            "  <AttributeDefinition id=\"uid\" xsi:type=\"Simple\">\n",
            "    <InputDataConnector ref=\"myLDAP\"/>\n",
            "  </AttributeDefinition>\n",
            "  <DataConnector id=\"myLDAP\" xsi:type=\"LDAPDirectory\"/>\n",
            "  <AttributeDefinition id=\"cn\" xsi:type=\"Simple\"/>\n",
            "</resolver:AttributeResolver>\n",
        );
        let c = Shibconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.definitions, 2);
        assert_eq!(c.connectors, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Shibconf::parse(b"<html>").is_none());
    }
}
