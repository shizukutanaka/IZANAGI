//! CDI `beans.xml` の検出と構造カウント。
//!
//! `<beans>` ルート（`bean-discovery-mode` 属性を含む）+ `<alternatives>`/
//! `<decorators>`/`<interceptors>`/`<scan>`/`<exclude>`/`<class>`/
//! `<stereotype>` 等の要素を識別する。
//!
//! ```
//! let b = br#"<beans xmlns="https://jakarta.ee/xml/ns/jakartaee"
//!        bean-discovery-mode="annotated" version="4.0">
//!   <alternatives>
//!     <class>x.FastImpl</class>
//!   </alternatives>
//!   <interceptors>
//!     <class>x.LoggingInterceptor</class>
//!   </interceptors>
//! </beans>"#;
//! assert!(izanagi_kit::beansxml::detect(b));
//! let c = izanagi_kit::beansxml::BeansXml::parse(b).unwrap();
//! assert_eq!(c.elements, 4);
//! ```

/// Parsed beans.xml summary.
#[derive(Debug, Clone)]
pub struct BeansXml {
    /// `<beans>` root present.
    pub has_root: bool,
    /// Recognized descriptor elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<alternatives")
        || tr.starts_with("<decorators")
        || tr.starts_with("<interceptors")
        || tr.starts_with("<scan")
        || tr.starts_with("<exclude")
        || tr.starts_with("<class>")
        || tr.starts_with("<class ")
        || tr.starts_with("<stereotype")
        || tr.starts_with("<priority")
        || tr.starts_with("<bean-discovery-mode")
}

fn beans_root(tr: &str) -> bool {
    tr.starts_with("<beans ") || tr.starts_with("<beans>")
}

/// Detect a CDI beans.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    let mut mode = false;
    for l in t.lines() {
        let tr = l.trim();
        if beans_root(tr) {
            root = true;
            if tr.contains("bean-discovery-mode") {
                mode = true;
            }
            continue;
        }
        if tr.starts_with("<!--") {
            continue;
        }
        if tr.contains("bean-discovery-mode") {
            mode = true;
        }
        if element(tr) {
            elems += 1;
        }
    }
    (root && (mode || elems >= 1)) || (mode && elems >= 1) || elems >= 4
}

impl BeansXml {
    /// Count categories. Returns `None` when the input does not look like
    /// a beans.xml descriptor.
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
            if beans_root(tr) {
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

/// Convenience wrapper around [`BeansXml::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<BeansXml> {
    BeansXml::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<?xml version="1.0"?>
<beans xmlns="https://jakarta.ee/xml/ns/jakartaee"
       bean-discovery-mode="annotated" version="4.0">
  <scan>
    <exclude name="x.legacy.**">
      <if-class-available name="x.Optional"/>
    </exclude>
  </scan>
  <alternatives>
    <class>x.FastImpl</class>
    <stereotype>x.HighPerf</stereotype>
  </alternatives>
  <decorators>
    <class>x.CachingDecorator</class>
  </decorators>
  <interceptors>
    <class>x.LoggingInterceptor</class>
    <class>x.TxInterceptor</class>
  </interceptors>
</beans>"#;
        assert!(detect(b));
        let c = BeansXml::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 10);
    }

    #[test]
    fn detects_empty_marker_beans() {
        assert!(detect(b"<beans bean-discovery-mode=\"all\"/>"));
    }

    #[test]
    fn rejects_generic_xml() {
        assert!(!detect(
            b"<configuration><beans><bean id=\"x\"/></beans></configuration>"
        ));
        assert!(!detect(b"<beans><bean id=\"x\" class=\"y.Bean\"/></beans>"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(BeansXml::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"<beans>");
        assert!(!detect(&b));
    }
}
