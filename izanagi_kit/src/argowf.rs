//! Argo Workflows manifest detection and census.
//!
//! Detects `apiVersion: argoproj.io/…` with a workflow `kind:` (Workflow,
//! CronWorkflow, WorkflowTemplate, ClusterWorkflowTemplate,
//! WorkflowEventBinding, WorkflowTaskSet, WorkflowArtifactGCTask,
//! WorkflowTaskResult) and counts spec keys (`entrypoint`/`templates`/
//! `serviceAccountName`/`volumes`/`volumeClaimTemplates`/`arguments`/
//! `ttlStrategy`/`podGC`/`activeDeadlineSeconds`/`workflowTemplateRef`/
//! `workflowMetadata`/`onExit`/`suspend`/`shutdown`/`metrics`/`parallelism`/
//! `priorityClassName`/`priority`/`securityContext`/`imagePullSecrets`/
//! `hostAliases`/`dnsConfig`/`dnsPolicy`/`schedulerName`/
//! `artifactRepositoryRef`/`artifactGC`/`synchronization`/`execution`/
//! `hooks`/`automountServiceAccountToken`/`archiveLogs`/`podMetadata`/
//! `podDisruptionBudget`/`podPriority`/`podPriorityClassName`/
//! `retryStrategy`/`nodeSelector`/`affinity`/`tolerations`), template keys
//! (`name`/`inputs`/`outputs`/`container`/`script`/`resource`/`http`/
//! `suspend`/`dag`/`steps`/`retryStrategy`/`metadata`/`memoize`/`plugin`/
//! `executor`/`containerSet`/`sidecars`/`initContainer`/`failFast`/`timeout`/
//! `activeDeadlineSeconds`/`archiveLocation`/`serviceAccountName`/
//! `automountServiceAccountToken`), I/O keys (`inputs`/`outputs`/
//! `parameters`/`artifacts`/`from`/`fromExpression`/`path`/`s3`/`gcs`/`git`/
//! `oss`/`raw`/`configMap`/`artifactory`/`hdfs`/`azure`/`globalName`/`value`/
//! `valueFrom`/`default`/`jqFilter`/`mode`/`archive`/`archiveLocation`),
//! and `#` comment lines.
//!
//! ```
//! let b = b"apiVersion: argoproj.io/v1alpha1\nkind: Workflow\nmetadata:\n  generateName: hello-\nspec:\n  entrypoint: whalesay\n  templates:\n  - name: whalesay\n    container:\n      image: docker/whalesay:latest\n";
//! assert!(izanagi_kit::argowf::detect(b));
//! let c = izanagi_kit::argowf::Argowf::parse(b).unwrap();
//! assert_eq!(c.kinds, 1);
//! assert!(c.template_keys >= 2);
//! ```

/// Parsed Argo Workflows manifest summary.
#[derive(Debug, Clone)]
pub struct Argowf {
    /// workflow `kind:` occurrences (Workflow/CronWorkflow/…).
    pub kinds: usize,
    /// workflow spec keys (`entrypoint`/`templates`/`ttlStrategy`/…).
    pub spec_keys: usize,
    /// template-body keys (`container`/`script`/`resource`/`dag`/`steps`/…).
    pub template_keys: usize,
    /// inputs/outputs/artifact keys.
    pub io_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const WF_KINDS: &[&str] = &[
    "kind: Workflow",
    "kind: CronWorkflow",
    "kind: WorkflowTemplate",
    "kind: ClusterWorkflowTemplate",
    "kind: WorkflowEventBinding",
    "kind: WorkflowTaskSet",
    "kind: WorkflowArtifactGCTask",
    "kind: WorkflowTaskResult",
];

const SPEC_KEYS: &[&str] = &[
    "entrypoint:",
    "templates:",
    "serviceAccountName:",
    "volumes:",
    "volumeClaimTemplates:",
    "arguments:",
    "ttlStrategy:",
    "podGC:",
    "activeDeadlineSeconds:",
    "workflowTemplateRef:",
    "workflowMetadata:",
    "onExit:",
    "suspend:",
    "shutdown:",
    "metrics:",
    "prometheus:",
    "parallelism:",
    "priorityClassName:",
    "priority:",
    "securityContext:",
    "imagePullSecrets:",
    "hostAliases:",
    "dnsConfig:",
    "dnsPolicy:",
    "schedulerName:",
    "artifactRepositoryRef:",
    "artifactGC:",
    "synchronization:",
    "mutex:",
    "semaphore:",
    "execution:",
    "hooks:",
    "automountServiceAccountToken:",
    "archiveLogs:",
    "podMetadata:",
    "podDisruptionBudget:",
    "podPriority:",
    "retryStrategy:",
    "nodeSelector:",
    "affinity:",
    "tolerations:",
    "ttlSecondsAfterFinished:",
    "secondsAfterCompletion:",
    "secondsAfterSuccess:",
    "secondsAfterFailure:",
];

const TEMPLATE_KEYS: &[&str] = &[
    "container:",
    "containerSet:",
    "script:",
    "resource:",
    "http:",
    "suspend:",
    "dag:",
    "steps:",
    "memoize:",
    "plugin:",
    "executor:",
    "initContainer:",
    "sidecars:",
    "failFast:",
    "timeout:",
    "archiveLocation:",
    "when:",
    "withParam:",
    "withSequence:",
    "withItems:",
    "dependencies:",
    "depends:",
    "continueOn:",
    "recursion:",
    "template:",
    "templateRef:",
    "inline:",
    "manifest:",
    "manifestFrom:",
    "selector:",
    "strategy:",
    "action:",
    "successCondition:",
    "failureCondition:",
    "estimatedDuration:",
    "driver:",
    "expressions:",
    "outputs:",
    "inputs:",
];

const IO_KEYS: &[&str] = &[
    "parameters:",
    "artifacts:",
    "from:",
    "fromExpression:",
    "path:",
    "s3:",
    "gcs:",
    "git:",
    "oss:",
    "raw:",
    "configMap:",
    "artifactory:",
    "hdfs:",
    "azure:",
    "globalName:",
    "value:",
    "valueFrom:",
    "default:",
    "jqFilter:",
    "mode:",
    "archive:",
    "subPath:",
    "optional:",
    "globalScope:",
];

/// Detects Argo Workflows manifests.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    t.contains("argoproj.io/") && WF_KINDS.iter().any(|k| t.contains(k))
}

impl Argowf {
    /// Parses an Argo Workflows manifest, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            kinds: t
                .lines()
                .filter(|l| WF_KINDS.iter().any(|k| l.trim() == *k))
                .count(),
            spec_keys: 0,
            template_keys: 0,
            io_keys: 0,
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
            if TEMPLATE_KEYS.iter().any(|k| tr.starts_with(k)) || tr.starts_with("name:") {
                c.template_keys += 1;
            }
            if IO_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.io_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"apiVersion: argoproj.io/v1alpha1\nkind: Workflow\nmetadata:\n  generateName: hello-\nspec:\n  entrypoint: whalesay\n  templates:\n  - name: whalesay\n    container:\n      image: docker/whalesay:latest\n      command: [cowsay]\n      args: [\"hello world\"]\n";
        let c = Argowf::parse(b).unwrap();
        assert_eq!(c.kinds, 1);
        assert!(c.spec_keys >= 2);
        assert!(c.template_keys >= 2);
    }

    #[test]
    fn detects_cron_workflow() {
        let b = b"apiVersion: argoproj.io/v1alpha1\nkind: CronWorkflow\nmetadata:\n  name: daily\nspec:\n  schedule: \"0 6 * * *\"\n  workflowSpec:\n    entrypoint: main\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other_crds() {
        assert!(Argowf::parse(b"apiVersion: v1\nkind: Pod\n").is_none());
        assert!(!detect(
            b"apiVersion: argoproj.io/v1alpha1\nkind: Application\n"
        ));
    }
}
