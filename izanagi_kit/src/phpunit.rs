//! PHPUnit `phpunit.xml` / `phpunit.xml.dist` census.
//!
//! `<phpunit bootstrap=… colors=…>` root + `<testsuites>`/`<testsuite>`/
//! `<directory>`/`<file>`/`<exclude>`, `<coverage>`/`<source>`/
//! `<include>`/`<report>`, `<php>`/`<env>`/`<ini>`/`<const>`/`<server>`,
//! `<extensions>`, `<groups>`/`<listeners>` elements.
//!
//! ```rust
//! let x = br#"<phpunit bootstrap="vendor/autoload.php" colors="true">
//! <testsuites><testsuite name="unit"><directory>tests/Unit</directory></testsuite></testsuites>
//! </phpunit>"#;
//! assert!(izanagi_kit::phpunit::detect(x));
//! ```

/// phpunit.xml census.
#[derive(Debug, Clone)]
pub struct Phpunit {
    /// `<phpunit` root element seen.
    pub has_root: bool,
    /// Test-suite/coverage/php/extension element lines.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<testsuites")
        || tr.starts_with("<testsuite")
        || tr.starts_with("<coverage")
        || tr.starts_with("<source")
        || tr.starts_with("<include")
        || tr.starts_with("<exclude")
        || tr.starts_with("<report")
        || tr.starts_with("<php>")
        || tr.starts_with("<php ")
        || tr.starts_with("<env")
        || tr.starts_with("<ini")
        || tr.starts_with("<const")
        || tr.starts_with("<server")
        || tr.starts_with("<get")
        || tr.starts_with("<post")
        || tr.starts_with("<cookie")
        || tr.starts_with("<extensions")
        || tr.starts_with("<bootstrap")
        || tr.starts_with("<groups")
        || tr.starts_with("<group")
        || tr.starts_with("<listeners")
        || tr.starts_with("<directory")
        || tr.starts_with("<file")
        || tr.starts_with("<ini ")
        || tr.starts_with("<logging")
        || tr.starts_with("<junit")
        || tr.starts_with("<teamcity")
        || tr.starts_with("<testdoxHtml")
        || tr.starts_with("<testdoxText")
        || tr.starts_with("<testdoxXml")
        || tr.starts_with("<text")
        || tr.starts_with("<clover")
        || tr.starts_with("<cobertura")
        || tr.starts_with("<crap4j")
        || tr.starts_with("<html")
        || tr.starts_with("<php ")
        || tr.starts_with("<require")
        || tr.starts_with("<suffix")
}

/// Detect a `phpunit.xml`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<phpunit") {
            root = true;
            continue;
        }
        if element(tr) {
            elems += 1;
        }
    }
    root || elems >= 3
}

impl Phpunit {
    /// Count elements. Returns `None` when the input does not look like a
    /// `phpunit.xml`.
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
            if tr.starts_with("<phpunit") {
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
<phpunit xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:noNamespaceSchemaLocation="vendor/phpunit/phpunit/phpunit.xsd"
         bootstrap="vendor/autoload.php"
         cacheDirectory=".phpunit.cache"
         colors="true">
    <testsuites>
        <testsuite name="unit">
            <directory>tests/Unit</directory>
        </testsuite>
        <testsuite name="feature">
            <directory>tests/Feature</directory>
        </testsuite>
    </testsuites>
    <coverage>
        <report><html outputDirectory="coverage"/></report>
    </coverage>
    <php>
        <env name="APP_ENV" value="testing"/>
        <ini name="memory_limit" value="512M"/>
    </php>
</phpunit>
"#;
        assert!(detect(b));
        let c = Phpunit::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 10);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"<html><body>x</body></html>"));
        assert!(!detect(b"key=value\n"));
        assert!(Phpunit::parse(b"").is_none());
    }
}
