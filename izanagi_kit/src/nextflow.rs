//! `nextflow.config` 検出モジュール。
//!
//! Nextflow ワークフローの設定は Groovy/HOCON 風で、`params.*`、
//! `process { }`、`executor = `、`profiles { }`、`docker { }`、
//! `manifest { }`、`workDir`、`timeline`/`report`/`trace`/`dag`
//! ブロックが特徴。
//!
//! ```
//! let b = br#"params.reads = "data/*_{1,2}.fq.gz"
//! params.outdir = "results"
//! process.executor = "slurm"
//! process.cpus = 4
//! docker.enabled = true
//! profiles {
//!     standard {
//!         process.executor = "local"
//!     }
//! }
//! "#;
//! let c = izanagi_kit::nextflow::parse(b);
//! assert!(izanagi_kit::nextflow::detect(b));
//! assert!(c.dotted_keys >= 4);
//! ```

const BLOCKS: &[&str] = &[
    "aws",
    "azure",
    "cloud",
    "conda",
    "dag",
    "docker",
    "executor",
    "google",
    "k8s",
    "mail",
    "manifest",
    "notification",
    "params",
    "podman",
    "process",
    "profiles",
    "report",
    "singularity",
    "timeline",
    "tower",
    "trace",
    "wave",
    "workDir",
];

fn is_comment(t: &str) -> bool {
    t.starts_with("//") || t.starts_with('#')
}

fn dotted_key(t: &str) -> bool {
    // `params.x = ` or `process.foo = ` or `docker.enabled = `
    let Some(eq) = t.find('=') else {
        return false;
    };
    let key = t[..eq].trim();
    BLOCKS.iter().any(|b| key.starts_with(&format!("{b}.")))
}

fn block_line(t: &str) -> bool {
    // `process {` or `profiles {`
    BLOCKS
        .iter()
        .any(|b| t == *b || t.starts_with(&format!("{b} ")) && t.contains('{'))
}

/// `b` が nextflow.config に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut dotted = 0usize;
    let mut blocks = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if dotted_key(tr) {
            dotted += 1;
        } else if block_line(tr) {
            blocks += 1;
        }
    }
    dotted >= 2 || (blocks >= 1 && dotted >= 1) || blocks >= 2
}

/// nextflow.config の統計。
#[derive(Debug, Default, Clone)]
pub struct NextflowConf {
    /// `params.*`/`process.*`/`docker.*` 等ドットキー行数。
    pub dotted_keys: usize,
    /// `process {`/`profiles {` 等ブロック行数。
    pub blocks: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を nextflow.config として統計する。
pub fn parse(b: &[u8]) -> NextflowConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = NextflowConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if dotted_key(tr) {
            c.dotted_keys += 1;
        } else if block_line(tr) {
            c.blocks += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"params.reads = "data/*"
params.outdir = "results"
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.dotted_keys, 2);
    }

    #[test]
    fn detects_blocks() {
        let b = br#"process {
    executor = "slurm"
}
profiles {
    standard { }
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.blocks, 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"params.reads = \"x\"\n"));
        assert!(!detect(b"foo = 1\nbar = 2\nbaz = 3\n"));
        assert!(!detect(b"process.x = 1\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.dotted_keys, 0);
    }
}
