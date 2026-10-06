//! ArgoCD `Application`/`AppProject`/`ApplicationSet` manifest detection
//! and census.
//!
//! Detects `apiVersion: argoproj.io/…` with `kind: Application`/
//! `AppProject`/`ApplicationSet` and counts the three kinds, spec keys
//! (`source`/`sources`/`destination`/`project`/`syncPolicy`/
//! `ignoreDifferences`/`info`/`revisionHistoryLimit`), source keys
//! (`repoURL`/`chart`/`targetRevision`/`path`/`helm`/`kustomize`/`plugin`/
//! `directory`/`jsonnet`/`ref`/`recursive`/`valueFiles`/`valuesObject`/
//! `parameters`/`releaseName`/`name`/`helmValues`/`version`/`passCredentials`/
//! `ignoreMissingValueFiles`/`skipCrds`/`kubeVersion`/`apiVersions`/
//! `namespace`/`commonLabels`/`commonAnnotations`/`forceString`/`fileParameters`/
//! `ignoreHelmSelectors`/`includeCRDs`/`skipSchemaValidation`/
//! `commonAnnotationsEnvsubst`/`values`), sync keys (`automated`/`prune`/
//! `selfHeal`/`syncOptions`/`retry`/`policy`/`allowEmpty`/`pruneLast`/
//! `managedNamespaceMetadata`/`prunePropagationPolicy`/`limit`/`backoff`/
//! `duration`/`factor`/`maxDuration`), AppProject policy keys (`sourceRepos`/
//! `destinations`/`clusterResourceWhitelist`/`namespaceResourceBlacklist`/
//! `namespaceResourceWhitelist`/`clusterResourceBlacklist`/`roles`/`policies`/
//! `scope`/`sourceNamespaces`/`permitsOnly`/`orphanedResources`/
//! `signatureKeys`/`syncWindows`/`description`), ApplicationSet generator
//! keys (`generators`/`list`/`git`/`clusters`/`scmProvider`/`pullRequest`/
//! `matrix`/`merge`/`clusterDecisionResource`/`duck`/`plugin`/`files`/
//! `selector`/`matchExpressions`/`goTemplate`/`template`/`directories`/
//! `requeueAfterSeconds`/`refreshInterval`/`configMapRef`/`postBuildSelectors`),
//! and `#` comment lines.
//!
//! ```
//! let b = b"apiVersion: argoproj.io/v1alpha1\nkind: Application\nmetadata:\n  name: guestbook\nspec:\n  project: default\n  source:\n    repoURL: https://example.com/repo.git\n    targetRevision: HEAD\n    path: guestbook\n  destination:\n    server: https://kubernetes.default.svc\n    namespace: guestbook\n  syncPolicy:\n    automated:\n      prune: true\n      selfHeal: true\n";
//! assert!(izanagi_kit::argocd::detect(b));
//! let c = izanagi_kit::argocd::Argocd::parse(b).unwrap();
//! assert_eq!(c.apps, 1);
//! assert!(c.sync_keys >= 3);
//! ```

/// Parsed ArgoCD manifest summary.
#[derive(Debug, Clone)]
pub struct Argocd {
    /// `kind: Application` occurrences.
    pub apps: usize,
    /// `kind: AppProject` occurrences.
    pub projects: usize,
    /// `kind: ApplicationSet` occurrences.
    pub appsets: usize,
    /// spec keys (`source`/`destination`/`project`/`syncPolicy`/…).
    pub spec_keys: usize,
    /// source keys (`repoURL`/`targetRevision`/`helm`/`kustomize`/…).
    pub source_keys: usize,
    /// syncPolicy keys (`automated`/`prune`/`selfHeal`/`syncOptions`/…).
    pub sync_keys: usize,
    /// AppProject policy/generator keys.
    pub policy_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SPEC_KEYS: &[&str] = &[
    "source:",
    "sources:",
    "destination:",
    "project:",
    "syncPolicy:",
    "ignoreDifferences:",
    "info:",
    "revisionHistoryLimit:",
];

const SOURCE_KEYS: &[&str] = &[
    "repoURL:",
    "chart:",
    "targetRevision:",
    "path:",
    "helm:",
    "kustomize:",
    "plugin:",
    "directory:",
    "jsonnet:",
    "ref:",
    "recursive:",
    "valueFiles:",
    "valuesObject:",
    "parameters:",
    "releaseName:",
    "name:",
    "version:",
    "passCredentials:",
    "ignoreMissingValueFiles:",
    "skipCrds:",
    "kubeVersion:",
    "apiVersions:",
    "namespace:",
    "commonLabels:",
    "commonAnnotations:",
    "forceString:",
    "fileParameters:",
    "ignoreHelmSelectors:",
    "includeCRDs:",
    "skipSchemaValidation:",
    "commonAnnotationsEnvsubst:",
    "values:",
    "server:",
    "serverName:",
];

const SYNC_KEYS: &[&str] = &[
    "automated:",
    "prune:",
    "selfHeal:",
    "syncOptions:",
    "retry:",
    "policy:",
    "allowEmpty:",
    "pruneLast:",
    "managedNamespaceMetadata:",
    "prunePropagationPolicy:",
    "limit:",
    "backoff:",
    "duration:",
    "factor:",
    "maxDuration:",
];

const POLICY_KEYS: &[&str] = &[
    "sourceRepos:",
    "destinations:",
    "clusterResourceWhitelist:",
    "namespaceResourceBlacklist:",
    "namespaceResourceWhitelist:",
    "clusterResourceBlacklist:",
    "roles:",
    "policies:",
    "scope:",
    "sourceNamespaces:",
    "permitsOnly:",
    "orphanedResources:",
    "signatureKeys:",
    "syncWindows:",
    "description:",
    "generators:",
    "list:",
    "git:",
    "clusters:",
    "scmProvider:",
    "pullRequest:",
    "matrix:",
    "merge:",
    "clusterDecisionResource:",
    "duck:",
    "files:",
    "selector:",
    "matchExpressions:",
    "goTemplate:",
    "template:",
    "directories:",
    "requeueAfterSeconds:",
    "refreshInterval:",
    "configMapRef:",
    "postBuildSelectors:",
];
fn code_has(t: &str, needle: &str) -> bool {
    // `#` コメント行内の言及は証拠にしない。
    t.lines()
        .any(|l| !l.trim_start().starts_with('#') && l.contains(needle))
}
fn yaml_val<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let l = line.trim_start_matches(['"', '\'']);
    let r = l
        .strip_prefix(key)?
        .trim_start_matches(['"', '\''])
        .trim_start();
    r.strip_prefix(':')
        .map(|v| v.trim().trim_matches('"').trim_matches('\''))
}

fn has_kv(t: &str, key: &str, val: &str) -> bool {
    t.lines().any(|l| yaml_val(l.trim(), key) == Some(val))
}

/// Detects ArgoCD manifests.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    code_has(&t, "argoproj.io/")
        && (has_kv(&t, "kind", "Application")
            || has_kv(&t, "kind", "AppProject")
            || has_kv(&t, "kind", "ApplicationSet"))
}

impl Argocd {
    /// Parses an ArgoCD manifest, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            apps: t
                .lines()
                .filter(|l| l.trim() == "kind: Application")
                .count(),
            projects: t.lines().filter(|l| l.trim() == "kind: AppProject").count(),
            appsets: t
                .lines()
                .filter(|l| l.trim() == "kind: ApplicationSet")
                .count(),
            spec_keys: 0,
            source_keys: 0,
            sync_keys: 0,
            policy_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let tr = tr.trim_start_matches("- ").trim_start();
            if SPEC_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.spec_keys += 1;
            }
            if SOURCE_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.source_keys += 1;
            }
            if SYNC_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.sync_keys += 1;
            }
            if POLICY_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.policy_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"# argoproj.io/v1alpha1\n# kind: Application\n"));
    }

    #[test]
    fn detects_and_counts() {
        let b = b"apiVersion: argoproj.io/v1alpha1\nkind: Application\nmetadata:\n  name: guestbook\nspec:\n  project: default\n  source:\n    repoURL: https://example.com/repo.git\n    targetRevision: HEAD\n    path: guestbook\n  destination:\n    server: https://kubernetes.default.svc\n    namespace: guestbook\n  syncPolicy:\n    automated:\n      prune: true\n      selfHeal: true\n";
        let c = Argocd::parse(b).unwrap();
        assert_eq!(c.apps, 1);
        assert!(c.spec_keys >= 4);
        assert!(c.source_keys >= 3);
        assert!(c.sync_keys >= 3);
    }

    #[test]
    fn detects_appproject() {
        let b = b"apiVersion: argoproj.io/v1alpha1\nkind: AppProject\nmetadata:\n  name: p\nspec:\n  sourceRepos: ['*']\n";
        let c = Argocd::parse(b).unwrap();
        assert_eq!(c.projects, 1);
    }

    #[test]
    fn rejects_workflow() {
        assert!(Argocd::parse(b"apiVersion: argoproj.io/v1alpha1\nkind: Workflow\n").is_none());
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }
}
