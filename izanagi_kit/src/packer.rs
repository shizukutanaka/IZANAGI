//! Packer template format (JSON or HCL).
//!
//! Packer templates describe image builds: JSON `{"builders":[…],
//! "provisioners":[…], "post-processors":[…], "variables":{…}}` or
//! HCL `source "type" "name"` + `build {…}` + `variable "x"` blocks.
//!
//! ```
//! let b = br#"{
//!   "variables": { "region": "us-east-1" },
//!   "builders": [
//!     { "type": "amazon-ebs", "ami_name": "img" }
//!   ],
//!   "provisioners": [ { "type": "shell", "inline": ["echo hi"] } ],
//!   "post-processors": [ { "type": "vagrant" } ]
//! }"#;
//! assert!(izanagi_kit::packer::detect(b));
//! let c = izanagi_kit::packer::Packer::parse(b).unwrap();
//! assert_eq!(c.builders, 1);
//! assert_eq!(c.provisioners, 1);
//! ```

use crate::textutil::strip_bom;
/// Parsed Packer template summary.
#[derive(Debug, Clone)]
pub struct Packer {
    /// Builders (JSON objects in `builders` or `source` blocks).
    pub builders: usize,
    /// Provisioners (JSON objects or `provisioner "type"` blocks).
    pub provisioners: usize,
    /// Post-processors.
    pub post_processors: usize,
    /// `variables` object keys or `variable "name"` blocks.
    pub variables: usize,
    /// `build {}` blocks (HCL) or template-level `source` refs.
    pub builds: usize,
    /// `locals` blocks.
    pub locals: usize,
    /// Comment lines (`#`, `//`).
    pub comments: usize,
}

fn count_kv_entries(t: &str, key: &str) -> usize {
    // JSON: count `"key": { ... }` top-level entries via `"x":` inside
    // the block after `"key":`; approximate by counting `"` at indent.
    // Simpler: occurrences of `"<k>":` with `"k"` under variables key —
    // count `"<word>":` occurrences within the variables object.
    // Fallback: count `":` occurrences after key until next top key.
    if let Some(pos) = t.find(&format!("\"{key}\"",)) {
        t[pos..]
            .split(['{', '}'])
            .nth(1)
            .map_or(0, |seg| seg.matches("\":").count())
    } else {
        0
    }
}

/// Whether the buffer looks like a Packer template.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    (t.contains("\"builders\"") || t.contains("\"provisioners\""))
        && t.trim_start().starts_with('{')
        || t.contains("source \"")
        || (t.contains("build {") && t.contains("source"))
}

impl Packer {
    /// Parses a Packer template summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            builders: 0,
            provisioners: 0,
            post_processors: 0,
            variables: 0,
            builds: 0,
            locals: 0,
            comments: 0,
        };
        if t.trim_start().starts_with('{') {
            // JSON form: count objects inside each top-level array.
            if let Some(pos) = t.find("\"builders\"") {
                c.builders += t[pos..].split('{').count().saturating_sub(1).min(
                    t[pos..]
                        .split(']')
                        .next()
                        .map_or(0, |seg| seg.matches('{').count()),
                );
            }
            if let Some(pos) = t.find("\"provisioners\"") {
                c.provisioners += t[pos..]
                    .split(']')
                    .next()
                    .map_or(0, |seg| seg.matches('{').count());
            }
            if let Some(pos) = t.find("\"post-processors\"") {
                c.post_processors += t[pos..]
                    .split(']')
                    .next()
                    .map_or(0, |seg| seg.matches('{').count());
            }
            c.variables = count_kv_entries(t, "variables");
        } else {
            // HCL form.
            for l in t.lines() {
                let tr = l.trim();
                if tr.starts_with('#') || tr.starts_with("//") {
                    c.comments += 1;
                    continue;
                }
                if tr.starts_with("source ") || tr.starts_with("source \"") {
                    c.builders += 1;
                } else if tr.starts_with("provisioner ") {
                    c.provisioners += 1;
                } else if tr.starts_with("post-processor ") || tr.starts_with("post_processor ") {
                    c.post_processors += 1;
                } else if tr.starts_with("variable ") {
                    c.variables += 1;
                } else if tr.starts_with("build") && tr.contains('{') {
                    c.builds += 1;
                } else if tr.starts_with("locals") && tr.contains('{') {
                    c.locals += 1;
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
    fn parses_packer_json() {
        let b = br#"{
  "variables": { "region": "us-east-1", "ami": "base" },
  "builders": [
    { "type": "amazon-ebs", "ami_name": "img" },
    { "type": "docker", "image": "busybox" }
  ],
  "provisioners": [ { "type": "shell", "inline": ["echo hi"] } ],
  "post-processors": [ { "type": "vagrant" } ]
}"#;
        assert!(detect(b));
        let c = Packer::parse(b).unwrap();
        assert_eq!(c.builders, 2);
        assert_eq!(c.provisioners, 1);
        assert_eq!(c.post_processors, 1);
        assert_eq!(c.variables, 2);
    }

    #[test]
    fn parses_packer_hcl() {
        let b = br#"
variable "region" {
  default = "us-east-1"
}
source "amazon-ebs" "img" {
  ami_name = "img"
}
build {
  sources = ["source.amazon-ebs.img"]
  provisioner "shell" {
    inline = ["echo hi"]
  }
  post-processor "vagrant" {}
}
locals {
  x = 1
}
"#;
        assert!(detect(b));
        let c = Packer::parse(b).unwrap();
        assert_eq!(c.builders, 1);
        assert_eq!(c.provisioners, 1);
        assert_eq!(c.post_processors, 1);
        assert_eq!(c.variables, 1);
        assert_eq!(c.builds, 1);
        assert_eq!(c.locals, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{}"));
        assert!(Packer::parse(b"x").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
