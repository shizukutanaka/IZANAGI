//! Parser for Devfile files (`devfile.yaml`, devfile.io schema).
//!
//! Counts `schemaVersion`, `metadata.name`/`projects`, `components` entries
//! (container/kubernetes/openshift/image/volume types), `commands` entries
//! (exec/apply/composite), `events` hooks, `variables`, `starterProjects`,
//! and `attributes`.
//!
//! ```
//! let b = b"schemaVersion: 2\nmetadata:\n  name: dev\ncomponents:\n  - name: tools\n    container:\n      image: quay.io/x\n";
//! assert!(izanagi_kit::devfile::detect(b));
//! let c = izanagi_kit::devfile::Devfile::parse(b).unwrap();
//! assert_eq!(c.schema_version, 1);
//! assert_eq!(c.components, 1);
//! ```

/// Parsed devfile.yaml summary.
#[derive(Debug, Clone)]
pub struct Devfile {
    /// `schemaVersion:` declaration.
    pub schema_version: usize,
    /// `metadata:` block keys (name/version/displayName/description).
    pub metadata_keys: usize,
    /// `projects:`/`starterProjects:` entries.
    pub projects: usize,
    /// `components:` list entries.
    pub components: usize,
    /// `container:`/`kubernetes:`/`openshift:`/`image:`/`volume:` typed components.
    pub component_types: usize,
    /// `commands:` list entries.
    pub commands: usize,
    /// `exec:`/`apply:`/`composite:`/`vscodeTask`/`vscodeLaunch` command types.
    pub command_types: usize,
    /// `events:` hooks (preStart/postStart/preStop/postStop).
    pub events: usize,
    /// `variables:`/`attributes:` entries.
    pub variables: usize,
    /// `env:`/`endpoints:`/`resources:`/`volumes:` entries inside components.
    pub component_children: usize,
    /// `parents:`/`parent:` references.
    pub parents: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const COMP_TYPES: &[&str] = &[
    "container",
    "kubernetes",
    "openshift",
    "image",
    "volume",
    "plugin",
];
const CMD_TYPES: &[&str] = &["exec", "apply", "composite", "vscodeTask", "vscodeLaunch"];
const CHILDREN: &[&str] = &["env", "endpoints", "resources", "volumes", "volumeMounts"];

/// Returns `true` when the bytes look like a devfile.yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("schemaVersion")
        && (t.contains("components:") || t.contains("commands:") || t.contains("metadata:"))
}

impl Devfile {
    /// Parses a devfile.yaml, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            schema_version: 0,
            metadata_keys: 0,
            projects: 0,
            components: 0,
            component_types: 0,
            commands: 0,
            command_types: 0,
            events: 0,
            variables: 0,
            component_children: 0,
            parents: 0,
            comments: 0,
        };
        let mut ctx = "";
        let mut ctx_indent = 0usize;
        let mut comp_item_indent: Option<usize> = None;
        let mut cmd_item_indent: Option<usize> = None;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let i = l.len() - l.trim_start().len();
            if i == 0 {
                let key = tr.split(':').next().unwrap_or("");
                ctx = match key {
                    "metadata" | "components" | "commands" | "events" | "variables"
                    | "attributes" | "projects" | "starterProjects" | "parent" | "parents" => key,
                    _ => "",
                };
                ctx_indent = i;
                if key == "schemaVersion" {
                    c.schema_version += 1;
                }
                if key == "parent" || key == "parents" {
                    c.parents += 1;
                }
                continue;
            }
            if i <= ctx_indent {
                ctx = "";
                comp_item_indent = None;
                cmd_item_indent = None;
            }
            let key = tr
                .trim_start_matches('-')
                .trim()
                .split(':')
                .next()
                .unwrap_or("");
            match ctx {
                "metadata" => c.metadata_keys += 1,
                "components" => {
                    if tr.starts_with("- ") {
                        if let Some(ci) = comp_item_indent {
                            if i == ci && key == "name" {
                                c.components += 1;
                            }
                        } else {
                            comp_item_indent = Some(i);
                            if key == "name" {
                                c.components += 1;
                            }
                        }
                    }
                    if COMP_TYPES.contains(&key) {
                        c.component_types += 1;
                    }
                    if CHILDREN.contains(&key) {
                        c.component_children += 1;
                    }
                }
                "commands" => {
                    if tr.starts_with("- ") {
                        if let Some(ci) = cmd_item_indent {
                            if i == ci && key == "id" {
                                c.commands += 1;
                            }
                        } else {
                            cmd_item_indent = Some(i);
                            if key == "id" {
                                c.commands += 1;
                            }
                        }
                    }
                    if CMD_TYPES.contains(&key) {
                        c.command_types += 1;
                    }
                }
                "events" => c.events += 1,
                "variables" | "attributes" => c.variables += 1,
                "projects" | "starterProjects" if tr.starts_with("- ") && key == "name" => {
                    c.projects += 1;
                }
                _ => {}
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"schemaVersion: 2\nmetadata:\n  name: mydev\n  version: 1\ncomponents:\n  - name: tools\n    container:\n      image: quay.io/x\n      env:\n        - name: A\n          value: b\n  - name: vol\n    volume:\n      size: 1Gi\ncommands:\n  - id: build\n    exec:\n      component: tools\n      commandLine: make\nevents:\n  postStart:\n    - build\nvariables:\n  A: b\n";

    #[test]
    fn parses_devfile() {
        let c = Devfile::parse(CONF).unwrap();
        assert_eq!(c.schema_version, 1);
        assert_eq!(c.metadata_keys, 2);
        assert_eq!(c.components, 2);
        assert_eq!(c.component_types, 3);
        assert_eq!(c.component_children, 1);
        assert_eq!(c.commands, 1);
        assert_eq!(c.command_types, 1);
        assert_eq!(c.events, 2);
        assert_eq!(c.variables, 1);
    }

    #[test]
    fn rejects_non_devfile() {
        assert!(!detect(b"version: 3\nservices:"));
        assert!(Devfile::parse(b"x").is_none());
    }
}
