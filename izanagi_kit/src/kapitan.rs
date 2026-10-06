//! Kapitan inventory class census.
//!
//! Kapitan class files (`inventory/classes/*.yml`) pair a `classes:`
//! list of parent classes with a `parameters:` mapping; target files
//! add `kapitan:`/`compile:` sections.
//!
//! ```rust
//! let k = b"classes:\n  - common\nparameters:\n  cluster: prod\n  kapitan:\n    compile:\n    - output_path: manifests\n";
//! assert!(izanagi_kit::kapitan::detect(k));
//! ```

/// Kapitan class census.
#[derive(Debug, Clone)]
pub struct Kapitan {
    /// `classes:` entries.
    pub classes: usize,
    /// `parameters:` mapping keys (top level only).
    pub parameters: usize,
    /// `kapitan.compile` entries.
    pub compile: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => Some(s[..i].trim()),
        None => None,
    }
}

/// Detect Kapitan class/target content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut has_classes = false;
    let mut has_params = false;
    let mut has_kapitan = false;
    let mut has_compile = false;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            match k {
                "classes" => has_classes = true,
                "parameters" => has_params = true,
                _ => {}
            }
        }
        // `kapitan:`/`compile:` nest under `parameters:` — match any indent.
        let s = line.trim();
        if s == "kapitan:" {
            has_kapitan = true;
        } else if s == "compile:" {
            has_compile = true;
        }
    }
    (has_classes && has_params) || (has_params && has_kapitan && has_compile)
}

impl Kapitan {
    /// Census a Kapitan buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            classes: 0,
            parameters: 0,
            compile: 0,
            settings: 0,
            comments: 0,
        };
        let mut section = "";
        let mut in_compile = false;
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = top_key(line) {
                section = k;
                in_compile = false;
            }
            if s.starts_with("- ") {
                if section == "classes" {
                    c.classes += 1;
                } else if in_compile {
                    c.compile += 1;
                }
                continue;
            }
            if s.contains(':') {
                let k = s[..s.find(':').unwrap_or(s.len())].trim();
                if s == "kapitan:" {
                    section = "kapitan";
                }
                if s == "compile:" && section == "kapitan" {
                    in_compile = true;
                }
                if section == "parameters" && line.starts_with("  ") && !line.starts_with("   ") {
                    c.parameters += 1;
                }
                if !k.is_empty() {
                    c.settings += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_class() {
        let b = b"classes:\n  - common\n  - component.nginx\nparameters:\n  replicas: 2\n  image: nginx\n";
        assert!(detect(b));
        let c = Kapitan::parse(b).unwrap();
        assert_eq!(c.classes, 2);
        assert_eq!(c.parameters, 2);
    }

    #[test]
    fn detects_target() {
        let b = b"parameters:\n  kapitan:\n    compile:\n    - output_path: manifests\n      input_type: jinja2\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"classes:\n  - a\nfoo: bar\n"));
        assert!(!detect(b"parameters:\n  x: 1\n"));
        assert!(!detect(b"kind: Deployment\n"));
    }
}
