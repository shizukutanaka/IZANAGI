//! Parser for Jenkinsfiles (`Jenkinsfile`, Groovy declarative/scripted).
//!
//! Counts `stage(`/`steps {`/`post {`/`agent`/`environment`/`parameters`/
//! `triggers`/`tools`/`node {`/`parallel`/`sh`/`echo` usages and comments.
//!
//! ```
//! let b = b"pipeline {\n    agent any\n    stages {\n        stage('build') {\n            steps {\n                sh 'make'\n            }\n        }\n    }\n}\n";
//! assert!(izanagi_kit::jenkinsfile::detect(b));
//! let j = izanagi_kit::jenkinsfile::Jenkinsfile::parse(b).unwrap();
//! assert!(j.declarative);
//! assert_eq!(j.stages, 1);
//! assert_eq!(j.sh_calls, 1);
//! ```

/// Parsed Jenkinsfile summary.
#[derive(Debug, Clone)]
pub struct Jenkinsfile {
    /// Declarative syntax (`pipeline {`) present.
    pub declarative: bool,
    /// `stage(`/`stage '…'`/`stage "…"` blocks.
    pub stages: usize,
    /// `steps {`/`script {`/`stage('x') {` step containers.
    pub step_blocks: usize,
    /// `post {`/`always {`/`success {`/`failure {`/`cleanup {` blocks.
    pub post_blocks: usize,
    /// `agent any`/`agent none`/`agent {`/`agent label`/`agent docker` lines.
    pub agents: usize,
    /// `KEY = value` entries inside `environment {` blocks.
    pub env_entries: usize,
    /// `parameters {`/`string(`/`choice(`/`booleanParam(`/`text(`/`password(` usages.
    pub parameters: usize,
    /// `triggers {`/`cron(`/`pollSCM(`/`upstream(` usages.
    pub triggers: usize,
    /// `tools {`/`tool ` usages.
    pub tools: usize,
    /// `node {`/`node('…')`/`node("…")` blocks (scripted pipelines).
    pub nodes: usize,
    /// `parallel {`/`parallel(`/`matrix {` blocks.
    pub parallel_blocks: usize,
    /// `sh `/`bat `/`powershell ` step calls.
    pub sh_calls: usize,
    /// `echo `/`error(`/`error ` calls.
    pub echoes: usize,
    /// `timeout(`/`retry(`/`input(`/`withEnv(`/`withCredentials(` wrappers.
    pub wrappers: usize,
    /// `//` and `/*` comment lines.
    pub comments: usize,
}

fn count(t: &str, needles: &[&str]) -> usize {
    needles.iter().map(|n| t.matches(n).count()).sum()
}

/// `NAME = "v"` assignments inside `environment {` blocks.
fn env_entries(t: &str) -> usize {
    let mut n = 0;
    let mut depth: Option<usize> = None;
    let mut brace_depth = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.contains("environment") && tr.contains('{') && depth.is_none() {
            depth = Some(brace_depth + tr.matches('{').count() - tr.matches('}').count());
            brace_depth += tr.matches('{').count() - tr.matches('}').count();
            continue;
        }
        if let Some(d) = depth {
            brace_depth += tr.matches('{').count();
            if tr.contains('=') && !tr.starts_with("//") {
                n += 1;
            }
            brace_depth -= tr.matches('}').count();
            if brace_depth < d {
                depth = None;
            }
        } else {
            brace_depth += tr.matches('{').count();
            brace_depth -= tr.matches('}').count();
        }
    }
    n
}

/// Returns `true` when `b` looks like a Jenkinsfile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.contains("pipeline {")
        || (t.contains("node") && t.contains("stage"))
        || t.contains("stages {") && t.contains("agent")
}

impl Jenkinsfile {
    /// Parses `b` as a Jenkinsfile.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let comments = t
            .lines()
            .filter(|l| {
                let tr = l.trim_start();
                tr.starts_with("//") || tr.starts_with("/*")
            })
            .count();
        Some(Self {
            declarative: t.contains("pipeline {"),
            stages: count(t, &["stage(", "stage '", "stage \"", "stage{"]),
            step_blocks: count(t, &["steps {", "script {"]),
            post_blocks: count(
                t,
                &["post {", "always {", "success {", "failure {", "cleanup {"],
            ),
            agents: count(
                t,
                &[
                    "agent any",
                    "agent none",
                    "agent {",
                    "agent label",
                    "agent docker",
                    "agent kubernetes",
                ],
            ),
            env_entries: env_entries(t),
            parameters: count(
                t,
                &[
                    "parameters {",
                    "string(",
                    "choice(",
                    "booleanParam(",
                    "text(",
                    "password(",
                ],
            ),
            triggers: count(t, &["triggers {", "cron(", "pollSCM(", "upstream("]),
            tools: count(t, &["tools {", "tool '", "tool \""]),
            nodes: count(t, &["node {", "node('", "node(\""]),
            parallel_blocks: count(t, &["parallel {", "parallel(", "matrix {"]),
            sh_calls: count(t, &["sh ", "sh(", "bat ", "powershell "]),
            echoes: count(t, &["echo ", "echo(", "error(", "error '"]),
            wrappers: count(
                t,
                &[
                    "timeout(",
                    "retry(",
                    "input(",
                    "withEnv(",
                    "withCredentials(",
                ],
            ),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"// build
pipeline {
    agent any
    environment {
        CC = 'clang'
        MODE = 'fast'
    }
    parameters {
        string(name: 'TAG')
    }
    triggers {
        cron('H * * * *')
    }
    stages {
        stage('build') {
            steps {
                sh 'make'
                sh 'make test'
            }
        }
        stage('deploy') {
            steps {
                echo 'deploying'
            }
        }
    }
    post {
        always {
            echo 'done'
        }
    }
}
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"println 'hi'"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let j = Jenkinsfile::parse(SRC).unwrap();
        assert!(j.declarative);
        assert_eq!(j.stages, 2);
        assert_eq!(j.step_blocks, 2);
        assert_eq!(j.agents, 1);
        assert_eq!(j.env_entries, 2);
        assert_eq!(j.parameters, 2);
        assert_eq!(j.triggers, 2);
        assert_eq!(j.post_blocks, 2);
        assert_eq!(j.sh_calls, 2);
        assert_eq!(j.echoes, 2);
        assert_eq!(j.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Jenkinsfile::parse(b"\x01\x02").is_none());
    }
}
