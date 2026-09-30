//! Skaffold `skaffold.yaml` census.
//!
//! `apiVersion: skaffold/v4betaN` + `kind: Config` envelope
//! plus sections: `build:` (`artifacts` `- image:` entries,
//! `local`/`cluster`/`googleCloudBuild`/`kaniko`/`custom`
//! builders, `tagPolicy`), `test:` (`- image:`/`structureTests:`),
//! `deploy:` (`kubectl`/`kustomize`/`helm`/`kpt`/`rawYaml`/`render`),
//! `portForward:`, `profiles:` (`- name:` activations),
//! `resourceSelector:`, `sync:`, `requires:`,
//! `statusCheckDeadlineSeconds`, `manifests:`,
//! `deploy:`/`logTail`, `platforms`, `version`, `metadata`.
//!
//! ```rust
//! let s = "apiVersion: skaffold/v4beta7\nkind: Config\nbuild:\n  artifacts:\n  - image: app\n    docker:\n      dockerfile: Dockerfile\ndeploy:\n  kubectl:\n    manifests:\n    - k8s/*.yaml\n";
//! let c = izanagi_kit::skaffold::Skaffold::parse(s.as_bytes()).unwrap();
//! assert_eq!(c.artifacts, 1);
//! ```

/// skaffold.yaml census.
#[derive(Debug, Clone)]
pub struct Skaffold {
    /// Recognised section keys (top-level or nested).
    pub sections: usize,
    /// `- image:`/`artifacts` entries.
    pub artifacts: usize,
    /// `profiles:` `- name:` entries.
    pub profiles: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items (all).
    pub items: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SECTION_KEYS: &[&str] = &[
    "apiVersion",
    "kind",
    "metadata",
    "build",
    "test",
    "deploy",
    "render",
    "portForward",
    "profiles",
    "resourceSelector",
    "sync",
    "requires",
    "manifests",
    "artifacts",
    "local",
    "cluster",
    "googleCloudBuild",
    "kaniko",
    "custom",
    "tagPolicy",
    "gitCommit",
    "dateTime",
    "sha256",
    "envTemplate",
    "inputDigest",
    "customTemplate",
    "kubectl",
    "kustomize",
    "helm",
    "kpt",
    "rawYaml",
    "statusCheckDeadlineSeconds",
    "structureTests",
    "version",
    "labels",
    "dockerfile",
    "context",
    "target",
    "buildArgs",
    "cacheFrom",
    "docker",
    "bazel",
    "jib",
    "buildpacks",
    "ko",
    "pack",
    "deployer",
    "hooks",
    "cloudBuild",
    "executionEnvironment",
    "typecheck",
    "platforms",
    "namespace",
    "logTail",
];

/// Detect skaffold.yaml content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    t.contains("skaffold/v") || (t.contains("kind: Config") && t.contains("artifacts"))
}

impl Skaffold {
    /// Census a skaffold.yaml buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            artifacts: 0,
            profiles: 0,
            settings: 0,
            items: 0,
            comments: 0,
        };
        let mut in_profiles = false;
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(rest) = s.strip_prefix("- ") {
                c.items += 1;
                let rest = rest.trim();
                if rest.starts_with("image:") {
                    c.artifacts += 1;
                }
                if in_profiles && rest.starts_with("name:") {
                    c.profiles += 1;
                }
                continue;
            }
            if let Some(colon) = s.find(':') {
                let key = s[..colon].trim();
                if !line.starts_with(' ') && !line.starts_with('\t') {
                    in_profiles = key == "profiles";
                }
                if !key.is_empty() {
                    c.settings += 1;
                    if SECTION_KEYS.contains(&key) {
                        c.sections += 1;
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
    fn detects_skaffold() {
        let b = b"apiVersion: skaffold/v4beta7\nkind: Config\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_skaffold() {
        let b = concat!(
            "# skaffold\n",
            "apiVersion: skaffold/v4beta7\n",
            "kind: Config\n",
            "metadata:\n",
            "  name: my-app\n",
            "build:\n",
            "  tagPolicy:\n",
            "    gitCommit: {}\n",
            "  artifacts:\n",
            "  - image: app\n",
            "    context: .\n",
            "    docker:\n",
            "      dockerfile: Dockerfile\n",
            "      target: prod\n",
            "      buildArgs:\n",
            "        DEBUG: \"false\"\n",
            "  - image: worker\n",
            "    context: worker\n",
            "    custom:\n",
            "      buildCommand: ./build.sh\n",
            "  local:\n",
            "    push: false\n",
            "test:\n",
            "- image: app\n",
            "  structureTests:\n",
            "  - test/structure.yaml\n",
            "deploy:\n",
            "  kubectl:\n",
            "    manifests:\n",
            "    - k8s/*.yaml\n",
            "  statusCheckDeadlineSeconds: 300\n",
            "profiles:\n",
            "- name: dev\n",
            "  activation:\n",
            "  - env: DEV=true\n",
            "  patches:\n",
            "  - op: replace\n",
            "    path: /build/local/push\n",
            "    value: true\n",
            "- name: prod\n",
        );
        let c = Skaffold::parse(b.as_bytes()).unwrap();
        assert_eq!(c.artifacts, 3);
        assert_eq!(c.profiles, 2);
        assert!(c.sections >= 12);
        assert!(c.items >= 6);
        assert_eq!(c.comments, 1);
    }
}
