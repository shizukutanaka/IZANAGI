//! Waypoint `waypoint.hcl` config census.
//!
//! waypoint.hcl top-level blocks: `app "name" { build/deploy/release }`,
//! `runner { }`, `project = "name"`, `configsource "type" { }`,
//! `pipeline "name" { }`, `variable "name" { }`, `project { }` (runner).
//!
//! ```rust
//! let k = b"app \"web\" {\n  build {\n    use \"pack\" {\n    }\n  }\n  deploy {\n  }\n}\n";
//! assert!(izanagi_kit::waypoint::detect(k));
//! ```

/// Waypoint config census.
#[derive(Debug, Clone)]
pub struct Waypoint {
    /// `key =` assignment lines.
    pub assignments: usize,
    /// `name {`/`name "label" {` block openings.
    pub blocks: usize,
    /// `app "`/`pipeline "`/`variable "`/`configsource "` blocks.
    pub labelled: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

fn code_line(s: &str) -> &str {
    let s = s.trim();
    if s.starts_with('#') || s.starts_with("//") {
        return "";
    }
    s
}

fn is_block(s: &str) -> bool {
    let s = s.trim_end();
    s.ends_with('{') && !s.starts_with('}')
}

fn is_labelled(s: &str) -> bool {
    for p in ["app \"", "pipeline \"", "variable \"", "configsource \""] {
        if s.starts_with(p) && s.ends_with('{') {
            return true;
        }
    }
    false
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect a waypoint.hcl config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut has_app = false;
    let mut has_inner = false;
    for line in t.lines() {
        let s = code_line(line);
        if s.is_empty() {
            continue;
        }
        if s.starts_with("app \"") && s.ends_with('{') {
            has_app = true;
            continue;
        }
        // build/deploy/release/registry/runner blocks or `project =` are
        // only evidence together with an `app "` block.
        if is_block(s)
            && matches!(
                s.split_whitespace().next().unwrap_or(""),
                "build" | "deploy" | "release" | "registry" | "runner" | "pipeline" | "workspace"
            )
        {
            has_inner = true;
        }
        if s.starts_with("project =") || s.starts_with("project=") {
            has_inner = true;
        }
    }
    has_app && has_inner
}

impl Waypoint {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            assignments: 0,
            blocks: 0,
            labelled: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if is_block(s) {
                c.blocks += 1;
                if is_labelled(s) {
                    c.labelled += 1;
                }
            } else if s.contains('=') && !s.starts_with('}') {
                c.assignments += 1;
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
        let b = b"app \"web\" {\n  build {\n    use \"pack\" {\n    }\n  }\n  deploy {\n  }\n}\n";
        assert!(detect(b));
        let c = Waypoint::parse(b).unwrap();
        assert_eq!(c.labelled, 1);
        assert!(c.blocks >= 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"app \"web\" {\n}\n"));
        assert!(!detect(b"# app \"web\" {\n#   build {\n#   }\n# }\n"));
        assert!(!detect(b"resource \"x\" {\n  build {\n  }\n}\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
