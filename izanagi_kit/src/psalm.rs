//! Psalm `psalm.xml` census.
//!
//! `<psalm>` root + `<projectFiles>`/`<file>`/`<directory>`/
//! `<ignoreFiles>`/`<stubs>`/`<file name>`/`<issueHandlers>`/
//! `<enableExtensions>`/`<plugins>`/`<pluginClass>`/`<*Plugin>` and
//! `<MissingImmutableAnnotation>`/`<UndefinedMethod>`-style issue names
//! as `<errorLevel>` children.
//!
//! ```rust
//! let s = br#"<psalm errorLevel="1"><projectFiles><directory name="src"/><ignoreFiles><directory name="vendor"/></ignoreFiles></projectFiles></psalm>"#;
//! assert!(izanagi_kit::psalm::detect(s));
//! ```

/// psalm.xml census.
#[derive(Debug, Clone)]
pub struct Psalm {
    /// `<psalm` root seen.
    pub has_root: bool,
    /// Config element lines.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<projectFiles")
        || tr.starts_with("<ignoreFiles")
        || tr.starts_with("<stubs")
        || tr.starts_with("<file ")
        || tr.starts_with("<file>")
        || tr.starts_with("<directory")
        || tr.starts_with("<issueHandlers")
        || tr.starts_with("<enableExtensions")
        || tr.starts_with("<extensions")
        || tr.starts_with("<plugins")
        || tr.starts_with("<pluginClass")
        || tr.starts_with("<plugin")
        || tr.starts_with("<forbiddenFunctions")
        || tr.starts_with("<globalVariable")
        || tr.starts_with("<property")
        || tr.starts_with("<errorLevel")
        || tr.starts_with("<findUnusedCode")
        || tr.starts_with("<referencedFile")
        || tr.starts_with("<referencedSymbol")
        || tr.starts_with("<basedir")
        || tr.starts_with("<sealAllMethods")
        || tr.starts_with("<overrideMethodReturnType")
        || tr.starts_with("<ignoreExceptions")
        || tr.starts_with("<ignoreExceptionsAndGlobals")
        || tr.starts_with("<function")
        || tr.starts_with("<method")
        || tr.starts_with("<class")
        || tr.starts_with("<interface")
        || tr.starts_with("<trait")
        || tr.starts_with("<constant")
        || tr.starts_with("<phpVar")
        || tr.starts_with("<predefinedClassConstants")
}

/// Detect a `psalm.xml` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<psalm") {
            root = true;
            continue;
        }
        if element(tr) {
            elems += 1;
        }
    }
    root || (elems >= 3 && t.contains("</psalm>"))
}

impl Psalm {
    /// Count elements. Returns `None` when the input does not look like a
    /// `psalm.xml`.
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
            if tr.starts_with("<psalm") {
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
<psalm errorLevel="1" resolveFromConfigFile="false" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <projectFiles>
        <directory name="src" />
        <ignoreFiles>
            <directory name="vendor" />
            <file name="src/Generated/Lexer.php" />
        </ignoreFiles>
    </projectFiles>
    <issueHandlers>
        <MissingImmutableAnnotation>
            <errorLevel type="suppress"><directory name="src/Legacy"/></errorLevel>
        </MissingImmutableAnnotation>
    </issueHandlers>
    <plugins>
        <pluginClass class="Vendor\Psalm\Plugin"/>
    </plugins>
</psalm>
"#;
        assert!(detect(b));
        let c = Psalm::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 8);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"<ruleset></ruleset>"));
        assert!(!detect(b"key=value\n"));
        assert!(Psalm::parse(b"").is_none());
    }
}
