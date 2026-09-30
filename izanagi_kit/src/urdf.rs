//! URDF (Unified Robot Description Format) — the ROS robot model XML.
//!
//! ```
//! let d = b"<robot name=\"r2\"><link name=\"base\"/><link name=\"arm\"/><joint name=\"j\" type=\"fixed\"/></robot>";
//! let u = izanagi_kit::urdf::parse(d).unwrap();
//! assert_eq!(u.name, "r2");
//! assert_eq!(u.links, 2);
//! assert_eq!(u.joints, 1);
//! assert_eq!(u.joint_types, vec!["fixed".to_string()]);
//! ```

/// Parsed URDF document summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Urdf {
    /// `name` attribute of the `<robot>` root element.
    pub name: String,
    /// Number of `<link>` elements.
    pub links: usize,
    /// Number of `<joint>` elements.
    pub joints: usize,
    /// `type` of each `<joint>` (fixed / revolute / continuous / prismatic / floating / planar), in document order.
    pub joint_types: Vec<String>,
    /// Number of `<gazebo>` extension elements.
    pub gazebo: usize,
}

/// `key="value"` or `key='value'` inside a tag string.
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

/// True when the byte after a `<name` match is a real tag boundary.
fn boundary(rest: &str) -> bool {
    matches!(
        rest.chars().next(),
        Some(' ' | '\t' | '\r' | '\n' | '>' | '/' | '\0') | None
    )
}

fn count_tag(s: &str, name: &str) -> Vec<usize> {
    let pat = format!("<{name}");
    s.match_indices(&pat)
        .filter(|(i, _)| boundary(&s[i + pat.len()..]))
        .map(|(i, _)| i)
        .collect()
}

/// Parse a URDF document; `None` without a `<robot>` root.
pub fn parse(d: &[u8]) -> Option<Urdf> {
    let s = std::str::from_utf8(d).ok()?;
    let rpos = *count_tag(s, "robot").first()?;
    let rend = s[rpos..].find('>')? + rpos;
    let name = attr(&s[rpos..rend], "name")?;

    let links = count_tag(s, "link").len();
    let gazebo = count_tag(s, "gazebo").len();
    let mut joint_types = Vec::new();
    for i in count_tag(s, "joint") {
        if let Some(off) = s[i..].find('>') {
            joint_types.push(attr(&s[i..i + off], "type").unwrap_or_default());
        }
    }
    Some(Urdf {
        name,
        links,
        joints: joint_types.len(),
        joint_types,
        gazebo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"<robot name='bot'><link name='a'/><link name='b'/><joint name='j1' type='revolute'/><joint name='j2' type='prismatic'/><gazebo reference='a'/></robot>";
        let u = parse(d).unwrap();
        assert_eq!(u.name, "bot");
        assert_eq!(u.links, 2);
        assert_eq!(u.joints, 2);
        assert_eq!(u.joint_types, vec!["revolute", "prismatic"]);
        assert_eq!(u.gazebo, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<link name='a'/>").is_none()); // no <robot>
        assert!(parse(b"<robot><link/></robot>").is_none()); // no name attr
                                                             // <linkage> must not count as <link>
        assert_eq!(
            parse(b"<robot name='x'><linkage/></robot>").unwrap().links,
            0
        );
    }
}
