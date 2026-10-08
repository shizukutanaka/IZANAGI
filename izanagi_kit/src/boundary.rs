//! Boundary `boundary.hcl` config census.
//!
//! Boundary HCL blocks: `listener "tcp" { }`, `controller { }`,
//! `worker { }`, `kms "<type>" { }` (`aead`/`awskms`/`gcpckms`/
//! `azurekeyvault`/`alicloudkms`/`ociwkms`/`transit`/`pkcs11`/
//! `static`), `purpose = "…"` on listeners, `name =`, `description =`,
//! `disable_mlock`, `initial_upstreams`, `auth_token`, `controllers`.
//!
//! ```rust
//! let k = b"listener \"tcp\" {\n  purpose = \"api\"\n}\ncontroller {\n  name = \"ctl\"\n}\n";
//! assert!(izanagi_kit::boundary::detect(k));
//! ```

/// Boundary config census.
#[derive(Debug, Clone)]
pub struct Boundary {
    /// `key =` assignment lines.
    pub assignments: usize,
    /// `name {`/`name "label" {` block openings.
    pub blocks: usize,
    /// `listener`/`controller`/`worker`/`kms` blocks.
    pub boundary_blocks: usize,
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
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect a Boundary `boundary.hcl`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    // Vault also opens `listener "tcp" {`; require a boundary-only
    // companion block (`controller`/`worker`/`kms "`/`purpose =`).
    let mut has_listener = false;
    let mut has_boundary = false;
    for line in t.lines() {
        let s = code_line(line);
        if s.is_empty() {
            continue;
        }
        if s.starts_with("listener \"") && s.ends_with('{') {
            has_listener = true;
            continue;
        }
        if (s.starts_with("controller") || s.starts_with("worker") || s.starts_with("kms \""))
            && s.ends_with('{')
        {
            has_boundary = true;
        }
        if s.starts_with("purpose =") || s.starts_with("purpose=") {
            has_boundary = true;
        }
    }
    has_listener && has_boundary
}

impl Boundary {
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
            boundary_blocks: 0,
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
            if s.ends_with('{') && !s.starts_with('}') {
                c.blocks += 1;
                let head = s.split_whitespace().next().unwrap_or("");
                if matches!(
                    head,
                    "listener" | "controller" | "worker" | "kms" | "event" | "events"
                ) {
                    c.boundary_blocks += 1;
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
        let b = b"listener \"tcp\" {\n  purpose = \"api\"\n}\ncontroller {\n  name = \"ctl\"\n}\n";
        assert!(detect(b));
        let c = Boundary::parse(b).unwrap();
        assert_eq!(c.boundary_blocks, 2);
        assert_eq!(c.assignments, 2);
    }

    #[test]
    fn detects_worker() {
        assert!(detect(
            b"worker {\n  name = \"w\"\n}\nlistener \"tcp\" {\n  purpose = \"proxy\"\n}\n"
        ));
    }

    #[test]
    fn rejects_others() {
        // Vault-style: listener + storage/seal only.
        assert!(!detect(
            b"listener \"tcp\" {\n  address = \"0.0.0.0:8200\"\n}\nstorage \"raft\" {\n}\n"
        ));
        assert!(!detect(b"# listener \"tcp\" {\n# }\ncontroller {\n}\n"));
        assert!(!detect(b"controller {\n  name = \"x\"\n}\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
