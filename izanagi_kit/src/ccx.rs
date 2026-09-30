//! CalculiX `.inp` keyword deck — the Abaqus-like text input for
//! CalculiX CrunchiX. `*NODE`, `*ELEMENT`, `*MATERIAL`, `*STEP`,
//! `*STATIC`, `*BOUNDARY`, `*CLOAD`/`DLOAD`, `*EL FILE`/`NODE FILE`
//! records; `**` comments.
//!
//! Distinct from `inp`: `parse` requires a `*STEP` marker (CalculiX
//! decks always carry steps) and counts the CalculiX-flavoured
//! keyword families.
//!
//! ```
//! let f = b"** ccx deck\n*NODE\n1, 0., 0.\n*ELEMENT\n1, 1\n*STEP\n*STATIC\n*NODE FILE\nU\n*END STEP\n";
//! let c = izanagi_kit::ccx::parse(f).unwrap();
//! assert!(c.has_node && c.has_element);
//! assert_eq!(c.steps, 1);
//! ```

/// Parsed CalculiX deck summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Ccx {
    /// `*KEYWORD` records seen.
    pub keywords: usize,
    /// `*STEP` records.
    pub steps: usize,
    /// `*NODE` present.
    pub has_node: bool,
    /// `*ELEMENT` present.
    pub has_element: bool,
    /// `*END STEP` records.
    pub end_steps: usize,
}

/// Parse a CalculiX deck; `None` without `*NODE`/`*ELEMENT` + `*STEP`.
pub fn parse(d: &[u8]) -> Option<Ccx> {
    let s = std::str::from_utf8(d).ok()?;
    let mut c = Ccx {
        keywords: 0,
        steps: 0,
        has_node: false,
        has_element: false,
        end_steps: 0,
    };
    for line in s.lines() {
        let l = line.trim_start();
        if !l.starts_with('*') || l.starts_with("**") {
            continue;
        }
        c.keywords += 1;
        let up = l.to_uppercase();
        if up.starts_with("*END STEP") {
            c.end_steps += 1;
        } else if up.starts_with("*STEP") {
            c.steps += 1;
        }
        if up.starts_with("*NODE") {
            c.has_node = true;
        }
        if up.starts_with("*ELEMENT") {
            c.has_element = true;
        }
    }
    if c.steps == 0 || !(c.has_node || c.has_element) {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"*NODE\n*ELEMENT\n*STEP\n*STATIC\n*END STEP\n";
        let c = parse(f).unwrap();
        assert_eq!(c.steps, 1);
        assert_eq!(c.end_steps, 1);
        assert_eq!(c.keywords, 5);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"*NODE\n*ELEMENT\n").is_none()); // no step
        assert!(parse(b"*STEP\n*END STEP\n").is_none()); // no node/element
    }
}
