//! Garden.io `garden.yml`/`project.garden.yml` census.
//!
//! `apiVersion: garden.io/v1` (or legacy `garden.io/v0`) envelope
//! plus a `kind` in `Project`, `Module`, `Build`, `Deploy`, `Test`,
//! `Run`, `Workflow`, `ConfigTemplate`, `RenderTemplate`,
//! `Command`, `Secret`, `Provider`.
//!
//! ```rust
//! let k = b"apiVersion: garden.io/v1\nkind: Deploy\nname: api\nspec:\n  command: [npm, start]\n";
//! assert!(izanagi_kit::garden::detect(k));
//! ```

/// Garden.io config census.
#[derive(Debug, Clone)]
pub struct Garden {
    /// `kind` value.
    pub kind: Option<String>,
    /// `apiVersion`/`kind`/`name`/`spec`/`metadata` envelope lines.
    pub envelope: usize,
    /// `- ` list items.
    pub items: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KINDS: &[&str] = &[
    "Project",
    "Module",
    "Build",
    "Deploy",
    "Test",
    "Run",
    "Workflow",
    "ConfigTemplate",
    "RenderTemplate",
    "Command",
    "Secret",
    "Provider",
];

fn value_of(s: &str) -> &str {
    match s.find(':') {
        Some(i) => s[i + 1..].trim().trim_matches('"').trim_matches('\''),
        None => "",
    }
}

fn api_ok(t: &str) -> bool {
    t.lines().any(|l| {
        let s = l.trim();
        !s.starts_with('#') && s.starts_with("apiVersion") && value_of(s).starts_with("garden.io/")
    })
}

fn kind_val(t: &str) -> Option<String> {
    t.lines().find_map(|l| {
        let s = l.trim();
        if s.starts_with('#') || !s.starts_with("kind") {
            return None;
        }
        let v = value_of(s);
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    })
}

/// Detect a Garden.io config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    if !api_ok(t) {
        return false;
    }
    match kind_val(t) {
        Some(k) => KINDS.contains(&k.as_str()),
        None => false,
    }
}

impl Garden {
    /// Census a Garden buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            kind: kind_val(t),
            envelope: 0,
            items: 0,
            settings: 0,
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
                let k = s[..s.find(':').unwrap_or(s.len())].trim();
                if matches!(
                    k,
                    "apiVersion" | "kind" | "name" | "spec" | "metadata" | "type"
                ) {
                    c.envelope += 1;
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
    fn detects_deploy() {
        let b =
            b"apiVersion: garden.io/v1\nkind: Deploy\nname: api\nspec:\n  command: [npm, start]\n";
        assert!(detect(b));
        let c = Garden::parse(b).unwrap();
        assert_eq!(c.kind.as_deref(), Some("Deploy"));
    }

    #[test]
    fn detects_project() {
        let b = b"apiVersion: garden.io/v0\nkind: Project\nname: demo\nenvironments:\n  - name: local\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"apiVersion: v1\nkind: Deploy\n"));
        assert!(!detect(b"apiVersion: garden.io/v1\nkind: StatefulSet\n"));
        assert!(!detect(b"# apiVersion: garden.io/v1\nkind: Deploy\n"));
    }
}
