//! Parser for devcontainer.json files (`.devcontainer/devcontainer.json`, JSONC).
//!
//! Counts `name`/`image`/`dockerFile`/`build`, `features` entries,
//! `forwardPorts`/`portsAttributes`, lifecycle hooks (`postCreateCommand`
//! `postStartCommand` `onCreateCommand` `initializeCommand` `updateContentCommand`
//! `postAttachCommand` …), `customizations`, `remoteUser`/`mounts`/
//! `containerEnv`/`remoteEnv`, and comments.
//!
//! ```
//! let b = b"{\n  \"name\": \"dev\",\n  \"image\": \"mcr.microsoft.com/x\",\n  \"forwardPorts\": [3000]\n}\n";
//! assert!(izanagi_kit::devcontainer::detect(b));
//! let c = izanagi_kit::devcontainer::Devcontainer::parse(b).unwrap();
//! assert_eq!(c.named, 1);
//! assert_eq!(c.forward_ports, 1);
//! ```

/// Parsed devcontainer.json summary.
#[derive(Debug, Clone)]
pub struct Devcontainer {
    /// `"name": "…"` declaration.
    pub named: usize,
    /// `"image"` or `"dockerFile"`/`"build"` build source.
    pub build_source: usize,
    /// `"dockerComposeFile"` compose integration.
    pub compose: usize,
    /// `"features": {` entries (`"ghcr.io/…"` keys counted separately).
    pub features: usize,
    /// `"forwardPorts"`/`"portsAttributes"` keys.
    pub forward_ports: usize,
    /// Lifecycle hook keys (post*/onCreate/initialize/updateContent/waitFor).
    pub lifecycle_hooks: usize,
    /// `"customizations"`/`"vscode"` keys.
    pub customizations: usize,
    /// `"remoteUser"`/`"containerUser"`/`"remoteEnv"`/`"containerEnv"`/`"workspaceFolder"`/`"workspaceMount"`/`"mounts"` keys.
    pub environment_keys: usize,
    /// `"runArgs"`/`"shutdownAction"`/`"overrideCommand"` keys.
    pub runtime_keys: usize,
    /// `//` and `/*` comment lines (JSONC).
    pub comments: usize,
}

const LIFECYCLE: &[&str] = &[
    "onCreateCommand",
    "initializeCommand",
    "updateContentCommand",
    "postCreateCommand",
    "postStartCommand",
    "postAttachCommand",
    "waitFor",
];

const ENV_KEYS: &[&str] = &[
    "remoteUser",
    "containerUser",
    "updateRemoteUserUID",
    "remoteEnv",
    "containerEnv",
    "workspaceFolder",
    "workspaceMount",
    "mounts",
    "userEnvProbe",
];

const RUN_KEYS: &[&str] = &[
    "runArgs",
    "shutdownAction",
    "overrideCommand",
    "privileged",
    "capAdd",
    "securityOpt",
    "hostRequirements",
];

fn key(l: &str) -> Option<&str> {
    let tr = l.trim();
    let q = tr.strip_prefix('"')?;
    let end = q.find('"')?;
    if q[end + 1..].trim_start().starts_with(':') {
        Some(&q[..end])
    } else {
        None
    }
}

/// Returns `true` when the bytes look like a devcontainer.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let keys = [
        "\"devcontainerId\"",
        "\"customizations\"",
        "\"remoteUser\"",
        "\"forwardPorts\"",
        "\"postCreateCommand\"",
        "\"containerEnv\"",
        "\"remoteEnv\"",
        "\"features\"",
        "\"workspaceFolder\"",
    ];
    let hits = keys.iter().filter(|k| t.contains(**k)).count();
    hits >= 1 || (t.contains("\"image\"") && t.contains("\"name\""))
}

impl Devcontainer {
    /// Parses a devcontainer.json, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            named: 0,
            build_source: 0,
            compose: 0,
            features: 0,
            forward_ports: 0,
            lifecycle_hooks: 0,
            customizations: 0,
            environment_keys: 0,
            runtime_keys: 0,
            comments: 0,
        };
        let mut in_features = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("//") || tr.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            if in_features {
                if tr.starts_with('}') {
                    in_features = false;
                    continue;
                }
                if tr.starts_with('"') && tr.contains(':') {
                    c.features += 1;
                }
                continue;
            }
            if let Some(k) = key(l) {
                match k {
                    "name" => c.named += 1,
                    "image" | "dockerFile" | "dockerfile" | "build" => c.build_source += 1,
                    "dockerComposeFile" | "docker_compose_file" => c.compose += 1,
                    "features" => {
                        in_features = true;
                    }
                    "forwardPorts" | "portsAttributes" | "appPort" => c.forward_ports += 1,
                    "customizations" | "vscode" => c.customizations += 1,
                    _ => {
                        if LIFECYCLE.contains(&k) {
                            c.lifecycle_hooks += 1;
                        } else if ENV_KEYS.contains(&k) {
                            c.environment_keys += 1;
                        } else if RUN_KEYS.contains(&k) {
                            c.runtime_keys += 1;
                        }
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

    const CONF: &[u8] = b"// dev\n{\n  \"name\": \"rust-dev\",\n  \"image\": \"mcr.microsoft.com/devcontainers/rust\",\n  \"features\": {\n    \"ghcr.io/f1\": {},\n    \"ghcr.io/f2\": {}\n  },\n  \"forwardPorts\": [3000, 8080],\n  \"postCreateCommand\": \"cargo build\",\n  \"customizations\": {\n    \"vscode\": { \"extensions\": [] }\n  },\n  \"remoteUser\": \"vscode\",\n  \"mounts\": []\n}\n";

    #[test]
    fn parses_devcontainer() {
        let c = Devcontainer::parse(CONF).unwrap();
        assert_eq!(c.named, 1);
        assert_eq!(c.build_source, 1);
        assert_eq!(c.features, 2);
        assert_eq!(c.forward_ports, 1);
        assert_eq!(c.lifecycle_hooks, 1);
        assert_eq!(c.customizations, 2);
        assert_eq!(c.environment_keys, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_devcontainer() {
        assert!(!detect(b"{\"a\": 1}"));
        assert!(Devcontainer::parse(b"x").is_none());
    }
}
