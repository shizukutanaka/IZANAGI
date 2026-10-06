//! Kubeflow Pipelines compiled manifest detection and census.
//!
//! Detects KFP v2 IR (`tekton.dev` apiVersion + `PipelineRun`/`Pipeline`
//! kinds, `pipelines.kubeflow.org`/`pipeline.kubeflow.org` annotations, or
//! the KFP IR `components:`/`deploymentSpec:`/`schemaVersion:`/`sdkVersion:`
//! structure) and counts pipeline keys (`apiVersion`/`kind`/`metadata`/
//! `spec`/`status`), spec keys (`pipelineRef`/`pipelineSpec`/`params`/
//! `paramSpecs`/`serviceAccountName`/`serviceAccountNames`/`taskRunTemplate`/
//! `podTemplate`/`workspaces`/`timeouts`/`results`/`taskRunSpecs`/
//! `finally`/`status`/`displayName`), task keys (`name:`/`taskRef`/`taskSpec`/
//! `params`/`runAfter`/`when`/`retries`/`timeout`/`matrix`/`computeResources`/
//! `pipelineTask`/`onError`/`taskRunMetadata`/`customTask`/`annotations`/
//! `labels`/`displayName`), IR keys (`components`/`deploymentSpec`/
//! `executors`/`root`/`pipelineInfo`/`sdkVersion`/`schemaVersion`/
//! `defaultPipelineRoot`/`inputArtifacts`/`outputArtifacts`/`inputParameters`/
//! `outputParameters`/`executorLabel`/`dag`/`tasks`/`componentRef`/
//! `taskInfos`/`inputDefinitions`/`outputDefinitions`/`executorInput`),
//! `pipelines.kubeflow.org`/`tekton.dev`/`kubernetes.io`/`sidecar.istio.io`
//! annotations, and `#` comment lines.
//!
//! ```
//! let b = b"apiVersion: tekton.dev/v1beta1\nkind: PipelineRun\nmetadata:\n  annotations:\n    pipelines.kubeflow.org/pipeline_spec: '{}'\nspec:\n  pipelineSpec:\n    tasks:\n    - name: preprocess\n      taskSpec:\n        apiVersion: custom.tekton.dev\n      runAfter: []\n";
//! assert!(izanagi_kit::kubeflow::detect(b));
//! let c = izanagi_kit::kubeflow::Kubeflow::parse(b).unwrap();
//! assert_eq!(c.pipeline_keys, 1);
//! assert!(c.task_keys >= 2);
//! ```

/// Parsed Kubeflow Pipelines manifest summary.
#[derive(Debug, Clone)]
pub struct Kubeflow {
    /// `kind:` Pipeline/PipelineRun/Run/Task/TaskRun occurrences.
    pub pipeline_keys: usize,
    /// spec keys (`pipelineRef`/`pipelineSpec`/`params`/`podTemplate`/…).
    pub spec_keys: usize,
    /// task-level keys (`taskRef`/`taskSpec`/`runAfter`/`when`/…).
    pub task_keys: usize,
    /// KFP IR keys (`components`/`deploymentSpec`/`executors`/`dag`/…).
    pub ir_keys: usize,
    /// kubeflow/tekton/kubernetes annotation occurrences.
    pub annotations: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const PIPELINE_KINDS: &[&str] = &[
    "kind: PipelineRun",
    "kind: Pipeline",
    "kind: Run",
    "kind: TaskRun",
    "kind: Task",
    "kind: CustomRun",
];

const SPEC_KEYS: &[&str] = &[
    "pipelineRef:",
    "pipelineSpec:",
    "params:",
    "paramSpecs:",
    "serviceAccountName:",
    "serviceAccountNames:",
    "taskRunTemplate:",
    "podTemplate:",
    "workspaces:",
    "timeouts:",
    "results:",
    "taskRunSpecs:",
    "finally:",
    "status:",
    "displayName:",
    "description:",
];

const TASK_KEYS: &[&str] = &[
    "taskRef:",
    "taskSpec:",
    "params:",
    "runAfter:",
    "when:",
    "retries:",
    "timeout:",
    "matrix:",
    "computeResources:",
    "pipelineTask:",
    "onError:",
    "taskRunMetadata:",
    "customTask:",
    "annotations:",
    "labels:",
    "displayName:",
];

const IR_KEYS: &[&str] = &[
    "components:",
    "deploymentSpec:",
    "executors:",
    "root:",
    "pipelineInfo:",
    "sdkVersion:",
    "schemaVersion:",
    "defaultPipelineRoot:",
    "inputArtifacts:",
    "outputArtifacts:",
    "inputParameters:",
    "outputParameters:",
    "executorLabel:",
    "dag:",
    "tasks:",
    "componentRef:",
    "taskInfos:",
    "inputDefinitions:",
    "outputDefinitions:",
    "executorInput:",
    "platformSpec:",
    "deploymentConfig:",
];

const ANNOTATION_KEYS: &[&str] = &[
    "pipelines.kubeflow.org",
    "pipeline.kubeflow.org",
    "tekton.dev",
    "kubernetes.io",
    "sidecar.istio.io",
    "kfp-pipeline",
];
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| is_key(l.trim_start(), key))
}
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

/// Detects Kubeflow Pipelines manifests and IR.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    let kfp_ir =
        has_key(&t, "schemaVersion") && has_key(&t, "sdkVersion") && has_key(&t, "components");
    let kfp_ann = code_has(&t, "pipelines.kubeflow.org") || code_has(&t, "pipeline.kubeflow.org");
    let tekton = code_has(&t, "tekton.dev/")
        && (has_kv(&t, "kind", "PipelineRun") || has_kv(&t, "kind", "Pipeline"));
    kfp_ir || kfp_ann || tekton
}

impl Kubeflow {
    /// Parses a Kubeflow Pipelines manifest, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            pipeline_keys: t
                .lines()
                .filter(|l| PIPELINE_KINDS.iter().any(|k| l.trim() == *k))
                .count(),
            spec_keys: 0,
            task_keys: 0,
            ir_keys: 0,
            annotations: ANNOTATION_KEYS.iter().map(|k| t.matches(k).count()).sum(),
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
            if TASK_KEYS.iter().any(|k| tr.starts_with(k)) || tr.starts_with("name:") {
                c.task_keys += 1;
            }
            if IR_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.ir_keys += 1;
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
        assert!(!detect(
            b"# schemaVersion: 1\n# sdkVersion: 2\n# components: {}\n"
        ));
        assert!(!detect(b"# pipelines.kubeflow.org/x: y\n"));
        assert!(!detect(b"# tekton.dev/v1\n# kind: PipelineRun\n"));
    }

    #[test]
    fn detects_tekton_pipeline_run() {
        let b = b"apiVersion: tekton.dev/v1beta1\nkind: PipelineRun\nmetadata:\n  annotations:\n    pipelines.kubeflow.org/pipeline_spec: '{}'\nspec:\n  pipelineSpec:\n    tasks:\n    - name: preprocess\n      taskSpec:\n        apiVersion: custom.tekton.dev\n      runAfter: []\n";
        let c = Kubeflow::parse(b).unwrap();
        assert_eq!(c.pipeline_keys, 1);
        assert!(c.spec_keys >= 1);
        assert!(c.task_keys >= 2);
        assert_eq!(c.annotations, 3);
    }

    #[test]
    fn detects_ir() {
        let b = b"components:\n  comp-train:\n    executorLabel: exec-train\ndeploymentSpec:\n  executors: {}\nroot:\n  dag:\n    tasks: {}\nschemaVersion: 2\nsdkVersion: kfp-2\npipelineInfo:\n  name: p\n";
        let c = Kubeflow::parse(b).unwrap();
        assert!(c.ir_keys >= 5);
    }

    #[test]
    fn rejects_plain_yaml() {
        assert!(Kubeflow::parse(b"key: value\n").is_none());
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }
}
