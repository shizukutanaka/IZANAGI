//! PHP_CodeSniffer `phpcs.xml` / `ruleset.xml` census.
//!
//! `<ruleset name="…">` root + `<rule ref="…">` + `<exclude>`/
//! `<exclude-pattern>`/`<arg name="…" value="…"/>`/`<severity>`/
//! `<type>`/`<message>`/`<property name="…">` elements and
//! `<file>`/`<extensions>`/`<ini>`/`<autoload>` config children.
//!
//! ```rust
//! let r = br#"<ruleset name="MyStandard"><description>Std</description><rule ref="PSR12"/><arg name="tab-width" value="4"/><exclude-pattern>*/vendor/*</exclude-pattern></ruleset>"#;
//! assert!(izanagi_kit::phpcs::detect(r));
//! ```

/// phpcs ruleset census.
#[derive(Debug, Clone)]
pub struct Phpcs {
    /// `<ruleset` root seen.
    pub has_root: bool,
    /// Rule/config element lines.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<rule ")
        || tr.starts_with("<rule>")
        || tr.starts_with("<exclude ")
        || tr.starts_with("<exclude-pattern")
        || tr.starts_with("<excludePattern")
        || tr.starts_with("<arg ")
        || tr.starts_with("<severity")
        || tr.starts_with("<type")
        || tr.starts_with("<message")
        || tr.starts_with("<property ")
        || tr.starts_with("<properties")
        || tr.starts_with("<element")
        || tr.starts_with("<one-true-brace")
        || tr.starts_with("<file>")
        || tr.starts_with("<file ")
        || tr.starts_with("<extensions")
        || tr.starts_with("<ini ")
        || tr.starts_with("<autoload")
        || tr.starts_with("<config ")
        || tr.starts_with("<php")
}

/// Detect a `phpcs.xml`/`ruleset.xml` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut rules = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<ruleset") {
            root = true;
            continue;
        }
        if tr.starts_with("<rule ") || tr.starts_with("<rule>") {
            rules += 1;
        }
    }
    root || rules >= 2
}

impl Phpcs {
    /// Count elements. Returns `None` when the input does not look like a
    /// ruleset file.
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
            if tr.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("<ruleset") {
                c.has_root = true;
                continue;
            }
            if element(tr) {
                c.elements += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<?xml version="1.0"?>
<ruleset name="MyStandard">
    <description>Project standard</description>
    <arg name="tab-width" value="4"/>
    <arg name="encoding" value="utf-8"/>
    <file>src</file>
    <file>tests</file>
    <exclude-pattern>*/vendor/*</exclude-pattern>
    <exclude-pattern>*/cache/*</exclude-pattern>
    <rule ref="PSR12">
        <exclude name="Generic.WhiteSpace.DisallowTabIndent"/>
    </rule>
    <rule ref="Generic.Files.LineLength">
        <properties>
            <property name="lineLimit" value="120"/>
        </properties>
    </rule>
    <rule ref="Squiz.Commenting.FunctionComment">
        <severity>5</severity>
    </rule>
</ruleset>
"#;
        assert!(detect(b));
        let c = Phpcs::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 10);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"<phpunit></phpunit>"));
        assert!(!detect(b"foo=bar\n"));
        assert!(Phpcs::parse(b"").is_none());
    }
}
