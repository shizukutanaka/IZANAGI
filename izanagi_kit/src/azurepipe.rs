//! Parser for Azure Pipelines files (`azure-pipelines.yml`).
//!
//! Counts `trigger:` branches, `pool:`/`vmImage:`/`jobs:`/`stages:`/
//! `steps:` items, `- task:`/`- script:`/`- bash:`/`- pwsh:`/`- checkout:`
//! usages, `variables:`/`parameters:` entries, and comments.
//!
//! ```
//! let b = b"trigger:\n  - main\npool:\n  vmImage: ubuntu-latest\nsteps:\n  - script: make\n";
//! assert!(izanagi_kit::azurepipe::detect(b));
//! let a = izanagi_kit::azurepipe::Azurepipe::parse(b).unwrap();
//! assert_eq!(a.trigger_branches, 1);
//! assert_eq!(a.vm_image, "ubuntu-latest");
//! assert_eq!(a.scripts, 1);
//! ```

/// Parsed `azure-pipelines.yml` summary.
#[derive(Debug, Clone)]
pub struct Azurepipe {
    /// `- ` items under `trigger:` (branch names).
    pub trigger_branches: usize,
    /// `pool:` blocks.
    pub pools: usize,
    /// First `vmImage:` value.
    pub vm_image: String,
    /// Entries under `jobs:`.
    pub jobs: usize,
    /// Entries under `stages:`.
    pub stages: usize,
    /// `- ` items inside `steps:` blocks.
    pub steps: usize,
    /// `- task:` usages.
    pub tasks: usize,
    /// `- script:` usages.
    pub scripts: usize,
    /// `- bash:` usages.
    pub bash: usize,
    /// `- powershell:`/`- pwsh:` usages.
    pub powershell: usize,
    /// `- checkout:` usages.
    pub checkouts: usize,
    /// Entries under `variables:`.
    pub variables: usize,
    /// Entries under `parameters:`.
    pub parameters: usize,
    /// `resources:` section present.
    pub resources: bool,
    /// `#` comment lines.
    pub comments: usize,
}

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key).is_some_and(|r| {
        // `key :`/`"key" :`(コロン前の空白と閉じ引用符)も YAML では合法。
        r.trim_start()
            .trim_start_matches(['"', '\''])
            .trim_start()
            .starts_with(':')
    })
}

fn blocks<'a>(t: &'a str, key: &str) -> Vec<Vec<&'a str>> {
    let mut out: Vec<Vec<&'a str>> = Vec::new();
    let mut cur: Option<(usize, Vec<&'a str>)> = None;
    for l in t.lines() {
        let tr = l.trim_start();
        let i = l.len() - tr.len();
        if let Some((d, v)) = cur.as_mut() {
            if !tr.is_empty() && i <= *d {
                out.push(core::mem::take(v));
                cur = None;
            } else {
                v.push(l);
                continue;
            }
        }
        if cur.is_none() && !tr.is_empty() && is_key(tr, key) {
            cur = Some((i, Vec::new()));
        }
    }
    if let Some((_, v)) = cur.take() {
        out.push(v);
    }
    out
}

fn child_items(t: &str, key: &str) -> usize {
    blocks(t, key)
        .iter()
        .map(|b| {
            let at = b
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.len() - l.trim_start().len())
                .min()
                .unwrap_or(0);
            b.iter()
                .filter(|l| !l.trim().is_empty() && l.len() - l.trim_start().len() == at)
                .count()
        })
        .sum()
}

fn dash_items(t: &str, key: &str) -> usize {
    blocks(t, key)
        .iter()
        .map(|b| {
            let at = b
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.len() - l.trim_start().len())
                .min()
                .unwrap_or(0);
            b.iter()
                .filter(|l| {
                    let tr = l.trim_start();
                    l.len() - tr.len() == at && tr.starts_with('-')
                })
                .count()
        })
        .sum()
}

fn dash_prefixed(t: &str, prefix: &str) -> usize {
    t.lines()
        .filter(|l| l.trim_start().starts_with(prefix))
        .count()
}

fn val_after<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    for l in t.lines() {
        let tr = l.trim();
        if let Some(rest) = tr.trim_start_matches(['"', '\'']).strip_prefix(key) {
            let rest = rest.trim_start_matches(['"', '\'']).trim_start();
            if let Some(v) = rest.strip_prefix(':') {
                let v = v.trim().trim_matches(['"', '\'']).trim_end_matches(',');
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Returns `true` when `b` looks like an `azure-pipelines.yml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let has = |k: &str| t.lines().any(|l| is_key(l.trim_start(), k));
    (has("trigger") || has("pr")) && (has("pool") || has("jobs") || has("stages") || has("steps"))
        || t.contains("vmImage")
}

impl Azurepipe {
    /// Parses `b` as an `azure-pipelines.yml`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let comments = t
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count();
        Some(Self {
            trigger_branches: dash_items(t, "trigger") + dash_items(t, "branches"),
            pools: blocks(t, "pool").len(),
            vm_image: val_after(t, "vmImage").unwrap_or("").to_string(),
            jobs: child_items(t, "jobs"),
            stages: child_items(t, "stages"),
            steps: dash_items(t, "steps"),
            tasks: dash_prefixed(t, "- task:"),
            scripts: dash_prefixed(t, "- script:"),
            bash: dash_prefixed(t, "- bash:"),
            powershell: dash_prefixed(t, "- powershell:") + dash_prefixed(t, "- pwsh:"),
            checkouts: dash_prefixed(t, "- checkout:"),
            variables: child_items(t, "variables"),
            parameters: child_items(t, "parameters"),
            resources: !blocks(t, "resources").is_empty(),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_space_before_colon() {
        // YAML では `key :` も合法。
        let v = String::from_utf8_lossy(SRC).replace("trigger:", "trigger :");
        assert!(detect(v.as_bytes()));
    }

    const SRC: &[u8] = b"# az
trigger:
  - main
  - dev
pool:
  vmImage: ubuntu-latest
variables:
  CARGO_TERM_COLOR: always
stages:
  - stage: build
    jobs:
      - job: build
        steps:
          - task: Cargo@1
          - script: make test
          - bash: ./deploy.sh
          - checkout: self
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"foo: bar"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let a = Azurepipe::parse(SRC).unwrap();
        assert_eq!(a.trigger_branches, 2);
        assert_eq!(a.pools, 1);
        assert_eq!(a.vm_image, "ubuntu-latest");
        assert_eq!(a.stages, 1);
        assert_eq!(a.jobs, 1);
        assert_eq!(a.steps, 4);
        assert_eq!(a.tasks, 1);
        assert_eq!(a.scripts, 1);
        assert_eq!(a.bash, 1);
        assert_eq!(a.checkouts, 1);
        assert_eq!(a.variables, 1);
        assert_eq!(a.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Azurepipe::parse(b"\x01\x02").is_none());
    }
}
