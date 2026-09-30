//! xacro — the ROS XML-macro preprocessor language layered on URDF.
//!
//! ```
//! let d = br#"<robot xmlns:xacro="http://ros.org/wiki/xacro" name="x">
//!   <xacro:property name="len" value="1"/>
//!   <xacro:include filename="arm.urdf.xacro"/>
//!   <xacro:macro name="wheel" params="p"/></robot>"#;
//! let x = izanagi_kit::xacro::parse(d).unwrap();
//! assert_eq!(x.macros, vec!["wheel".to_string()]);
//! assert_eq!(x.properties, 1);
//! assert_eq!(x.includes, 1);
//! ```

/// Parsed xacro document summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xacro {
    /// Names of every `<xacro:macro name="…">`, in document order.
    pub macros: Vec<String>,
    /// Number of `<xacro:property>` declarations.
    pub properties: usize,
    /// Number of `<xacro:include>` elements.
    pub includes: usize,
    /// Number of `${…}` substitution expressions.
    pub substitutions: usize,
    /// `xmlns:xacro` namespace URI as declared.
    pub namespace: String,
}

fn attr(tag: &str, key: &str) -> Option<String> {
    for q in ['"', '\''] {
        let pat = format!("{key}={q}");
        if let Some(pos) = tag.find(&pat) {
            let rest = &tag[pos + pat.len()..];
            let end = rest.find(q)?;
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Parse an xacro document; `None` without an `xmlns:xacro` declaration.
pub fn parse(d: &[u8]) -> Option<Xacro> {
    let s = std::str::from_utf8(d).ok()?;
    let npos = s.find("xmlns:xacro")?;
    let nend = s[npos..].find(['"', '\''])? + npos;
    let q = s.as_bytes()[nend];
    let vend = s[nend + 1..].find(q as char)? + nend + 1;
    let namespace = s[nend + 1..vend].to_string();

    let mut macros = Vec::new();
    let pat = "<xacro:macro";
    for (i, _) in s.match_indices(pat) {
        if let Some(off) = s[i..].find('>') {
            if let Some(n) = attr(&s[i..i + off], "name") {
                macros.push(n);
            }
        }
    }
    let properties = s.matches("<xacro:property").count();
    let includes = s.matches("<xacro:include").count();
    let substitutions = s.matches("${").count();
    Some(Xacro {
        macros,
        properties,
        includes,
        substitutions,
        namespace,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = br#"<robot xmlns:xacro='http://ros.org/wiki/xacro' name='r'>
          <xacro:property name='a' value='1'/>
          <xacro:property name='b' value='2'/>
          <xacro:include filename='f.xacro'/>
          <xacro:macro name='m1' params='x'/><xacro:macro name='m2'/>
          <link name='${a}'/></robot>"#;
        let x = parse(d).unwrap();
        assert_eq!(x.macros, vec!["m1", "m2"]);
        assert_eq!(x.properties, 2);
        assert_eq!(x.includes, 1);
        assert_eq!(x.substitutions, 1);
        assert_eq!(x.namespace, "http://ros.org/wiki/xacro");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<robot name='x'/>").is_none()); // no xmlns:xacro
    }
}
