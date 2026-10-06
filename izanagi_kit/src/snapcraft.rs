//! Snapcraft `snapcraft.yaml` config census.
//!
//! Snapcraft top-level keys: `name`, `version`, `summary`, `description`,
//! `base`, `confinement`, `grade`, `type`, `apps`, `parts`, `plugs`,
//! `slots`, `hooks`, `architectures`, `assumes`, `epoch`, `adopt-info`,
//! `compression`, `layout`, `system-usernames`, `passthrough`.
//!
//! ```rust
//! let k = b"name: myapp\nbase: core22\nversion: '1.0'\nsummary: hi\nconfinement: strict\ngrade: stable\nparts:\n  mypart:\n    plugin: nil\napps:\n  myapp:\n    command: bin/x\n";
//! assert!(izanagi_kit::snapcraft::detect(k));
//! ```

/// Snapcraft config census.
#[derive(Debug, Clone)]
pub struct Snapcraft {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "confinement",
    "grade",
    "parts",
    "plugs",
    "slots",
    "hooks",
    "architectures",
    "assumes",
    "epoch",
    "adopt-info",
    "compression",
    "layout",
    "system-usernames",
    "passthrough",
    "apps",
];

const WEAK: &[&str] = &[
    "name",
    "version",
    "summary",
    "description",
    "type",
    "icon",
    "license",
    "contact",
    "donation",
    "issues",
    "source-code",
    "website",
    "title",
    "base",
    "provenance",
    "links",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"').trim_matches('\'');
            if k.is_empty() {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a Snapcraft `snapcraft.yaml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // vendor-exclusive keys carry the weight; shared keys only count
    // once a strong anchor is present.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Snapcraft {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
                    }
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
    fn detects() {
        let b = b"name: myapp\nbase: core22\nversion: '1.0'\nsummary: hi\nconfinement: strict\ngrade: stable\nparts:\n  mypart:\n    plugin: nil\napps:\n  myapp:\n    command: bin/x\n";
        assert!(detect(b));
        let c = Snapcraft::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"name: x\nversion: '1'\nsummary: s\n"));
        assert!(!detect(b"# confinement: strict\n# parts:\nname: x\n"));
    }
}
