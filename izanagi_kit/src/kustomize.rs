//! Kustomize `kustomization.yaml`/`kustomization.yml` census.
//!
//! `apiVersion: kustomize.config.k8s.io/v1beta1` + `kind:
//! Kustomization` envelope, then top-level keys: `resources`,
//! `bases`, `components`, `namespace`, `namePrefix`,
//! `nameSuffix`, `commonLabels`, `commonAnnotations`, `images`,
//! `patches`, `patchesStrategicMerge`, `patchesJson6902`,
//! `configMapGenerator`, `secretGenerator`, `generatorOptions`,
//! `vars`, `replacements`, `replicas`, `crds`, `helmCharts`,
//! `helmGlobals`, `transformers`, `validators`, `labels`,
//! `sortOptions`, `buildMetadata`, `openapi`, `configurations`,
//! `generatorConfigs`, `dependencies`, `completions`.
//!
//! ```rust
//! let k = "apiVersion: kustomize.config.k8s.io/v1beta1\nkind: Kustomization\nresources:\n- dep.yaml\n- svc.yaml\nnamePrefix: prod-\n";
//! let c = izanagi_kit::kustomize::Kustomize::parse(k.as_bytes()).unwrap();
//! assert_eq!(c.resources, 2);
//! ```

/// kustomization census.
#[derive(Debug, Clone)]
pub struct Kustomize {
    /// Recognised top-level keys.
    pub sections: usize,
    /// `- ` items under `resources`/`bases`/`components`/`crds`.
    pub resources: usize,
    /// `- ` items under `*Generator`.
    pub generators: usize,
    /// `- ` items under `patches*`/`replacements`/`images`/`replicas`/`vars`.
    pub patches: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items (all).
    pub items: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const TOP_KEYS: &[&str] = &[
    "apiVersion",
    "kind",
    "resources",
    "bases",
    "components",
    "namespace",
    "namePrefix",
    "nameSuffix",
    "commonLabels",
    "commonAnnotations",
    "labels",
    "images",
    "patches",
    "patchesStrategicMerge",
    "patchesJson6902",
    "configMapGenerator",
    "secretGenerator",
    "generatorOptions",
    "vars",
    "replacements",
    "replicas",
    "crds",
    "helmCharts",
    "helmGlobals",
    "transformers",
    "validators",
    "sortOptions",
    "buildMetadata",
    "openapi",
    "configurations",
    "generatorConfigs",
    "dependencies",
    "completions",
];

const RESOURCE_KEYS: &[&str] = &["resources", "bases", "components", "crds"];

const GENERATOR_KEYS: &[&str] = &["configMapGenerator", "secretGenerator", "generatorConfigs"];

const PATCH_KEYS: &[&str] = &[
    "patches",
    "patchesStrategicMerge",
    "patchesJson6902",
    "replacements",
    "images",
    "replicas",
    "vars",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect kustomization.yaml content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut kind_ok = false;
    let mut keys = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') || s.starts_with('-') {
            continue;
        }
        if let Some(colon) = s.find(':') {
            let key = &s[..colon];
            if key == "kind" && s.contains("Kustomization") {
                kind_ok = true;
            }
            if TOP_KEYS.contains(&key) {
                keys += 1;
            }
        }
    }
    (kind_ok && keys >= 2) || keys >= 4
}

impl Kustomize {
    /// Census a kustomization buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sections: 0,
            resources: 0,
            generators: 0,
            patches: 0,
            settings: 0,
            items: 0,
            comments: 0,
        };
        let mut cur = "";
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") || s.starts_with("-") && s.len() > 1 && s.as_bytes()[1] == b' ' {
                c.items += 1;
                if RESOURCE_KEYS.contains(&cur) {
                    c.resources += 1;
                } else if GENERATOR_KEYS.contains(&cur) {
                    c.generators += 1;
                } else if PATCH_KEYS.contains(&cur) {
                    c.patches += 1;
                }
                continue;
            }
            if let Some(colon) = s.find(':') {
                let key = s[..colon].trim();
                if !line.starts_with(' ') && !line.starts_with('\t') {
                    cur = "";
                    if TOP_KEYS.contains(&key) {
                        c.sections += 1;
                        cur = match key {
                            "resources" | "bases" | "components" | "crds" => key,
                            "configMapGenerator" | "secretGenerator" | "generatorConfigs" => key,
                            "patches"
                            | "patchesStrategicMerge"
                            | "patchesJson6902"
                            | "replacements"
                            | "images"
                            | "replicas"
                            | "vars" => key,
                            _ => "",
                        };
                    }
                }
                if !key.is_empty() {
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
    fn detects_kust() {
        let b = b"kind: Kustomization\nresources:\n- a.yaml\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_kust() {
        let b = concat!(
            "# kust\n",
            "apiVersion: kustomize.config.k8s.io/v1beta1\n",
            "kind: Kustomization\n",
            "namespace: prod\n",
            "namePrefix: prod-\n",
            "commonLabels:\n",
            "  app: web\n",
            "resources:\n",
            "- deployment.yaml\n",
            "- service.yaml\n",
            "- ingress.yaml\n",
            "configMapGenerator:\n",
            "- name: app-config\n",
            "  literals:\n",
            "  - KEY=val\n",
            "- name: env-config\n",
            "  envs:\n",
            "  - app.env\n",
            "secretGenerator:\n",
            "- name: creds\n",
            "  files:\n",
            "  - creds.txt\n",
            "images:\n",
            "- name: nginx\n",
            "  newTag: 1.25\n",
            "patchesStrategicMerge:\n",
            "- patch1.yaml\n",
            "- patch2.yaml\n",
            "replicas:\n",
            "- name: web\n",
            "  count: 3\n",
            "helmCharts:\n",
            "- name: redis\n",
            "  releaseName: cache\n",
        );
        let c = Kustomize::parse(b.as_bytes()).unwrap();
        assert!(c.sections >= 10);
        assert_eq!(c.resources, 3);
        assert_eq!(c.generators, 6);
        assert_eq!(c.patches, 4);
        assert!(c.items >= 10);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
