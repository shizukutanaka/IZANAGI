//! Census of an XACML policy document (XML).
//!
//! XACML `<Policy>`/`<PolicySet>` documents: `<Policy PolicyId="…" Version="…"
//! RuleCombiningAlgId="…">` with `<Target>` and `<Rule RuleId="…" Effect="Permit">`
//! elements, `<AttributeDesignator>`/`<AttributeValue>`/`<Apply>`/`Match`
//! functions (`urn:oasis:names:tc:xacml:*:function:*`), `Permit`/`Deny` effects,
//! `Obligation`/`ObligationExpression`, `AttributeAssignmentExpression`,
//! `PolicyIdReference`/`PolicySetIdReference`, `VariableDefinition`,
//! `Description`, `<!-- -->` comments.
//!
//! ```rust
//! let c = izanagi_kit::xacml::Xacml::parse(
//!     b"<Policy PolicyId=\"p\" RuleCombiningAlgId=\"a\"><Rule RuleId=\"r\" Effect=\"Permit\"/></Policy>\n",
//! ).unwrap();
//! assert_eq!(c.policies, 1);
//! ```
#![forbid(unsafe_code)]

/// XACML policy census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xacml {
    /// `<Policy` elements.
    pub policies: usize,
    /// `<PolicySet` elements.
    pub policy_sets: usize,
    /// `<Rule` elements.
    pub rules: usize,
    /// `<Target>`/`<AttributeDesignator>`/`<AttributeValue>`/`<Apply`/`Match`/`Obligation*`/`VariableDefinition`/`PolicyIdReference`/`PolicySetIdReference` elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

/// Element tags counted in `elements`.
const TAGS: &[&str] = &[
    "<Target",
    "<AttributeDesignator",
    "<AttributeValue",
    "<Apply",
    "<Match",
    "<Obligation",
    "<VariableDefinition",
    "<PolicyIdReference",
    "<PolicySetIdReference",
    "<Description",
];

/// True if `b` looks like XACML.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("<Policy") || t.contains("xacml"))
        && (t.contains("<Rule") || t.contains("<Target") || t.contains("xacml"))
}

impl Xacml {
    /// Parse an XACML document into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            policies: 0,
            policy_sets: 0,
            rules: 0,
            elements: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            if l.contains("<PolicySet") {
                c.policy_sets += l.matches("<PolicySet").count();
            } else if l.contains("<Policy") {
                c.policies += l.matches("<Policy").count();
            }
            c.rules += l.matches("<Rule").count();
            for tag in TAGS {
                c.elements += l.matches(tag).count();
            }
        }
        if c.policies + c.policy_sets == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "<!-- xacml -->\n",
            "<Policy PolicyId=\"p1\" Version=\"1\" RuleCombiningAlgId=\"urn:oasis:names:tc:xacml:3.0:rule-combining-algorithm:permit-overrides\">\n",
            "  <Target/>\n",
            "  <Rule RuleId=\"r1\" Effect=\"Permit\">\n",
            "    <Target>\n",
            "      <Match MatchId=\"urn:oasis:names:tc:xacml:1.0:function:string-equal\">\n",
            "        <AttributeValue DataType=\"http://www.w3.org/2001/XMLSchema#string\">admin</AttributeValue>\n",
            "        <AttributeDesignator AttributeId=\"role\" DataType=\"string\" Category=\"subject\"/>\n",
            "      </Match>\n",
            "    </Target>\n",
            "  </Rule>\n",
            "  <ObligationExpressions/>\n",
            "</Policy>\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Xacml::parse(b.as_bytes()).unwrap();
        assert_eq!(c.policies, 1);
        assert_eq!(c.rules, 1);
        assert!(c.elements >= 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"<html></html>\n"));
        assert!(Xacml::parse(b"<!-- none -->\n").is_none());
    }
}
