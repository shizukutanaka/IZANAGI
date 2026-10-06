//! Terragrunt `terragrunt.hcl` census.
//!
//! Terragrunt-only constructs: `include "..."`/`include {}` blocks,
//! `dependency "..."` blocks, `remote_state {`, `generate "..."`,
//! `inputs = {`, `locals` helpers `find_in_parent_folders`,
//! `get_parent_terragrunt_dir`, `path_relative_to_include`,
//! `path_relative_from_include`, `read_terragrunt_config`,
//! `get_repo_root`, `expose = true`, `merge_strategy`,
//! `skip_dependencies`, `retryable_errors`, `download_dir`,
//! `iam_role`, `prevent_destroy`, `skip`, `deep_merge`.
//!
//! ```rust
//! let k = b"include \"root\" {\n  path = find_in_parent_folders()\n}\ninputs = {\n  env = \"prod\"\n}\n";
//! assert!(izanagi_kit::terragrunt::detect(k));
//! ```

/// terragrunt.hcl census.
#[derive(Debug, Clone)]
pub struct Terragrunt {
    /// `include` blocks.
    pub includes: usize,
    /// `dependency`/`dependencies` blocks.
    pub dependencies: usize,
    /// `remote_state`/`generate`/`terraform`/`iam_*`/`download_dir` blocks.
    pub blocks: usize,
    /// Helper calls (`find_in_parent_folders`, `get_*`, `path_*`).
    pub helper_calls: usize,
    /// `key = value` assignments.
    pub assignments: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

const STRONG_TOKENS: &[&str] = &[
    "find_in_parent_folders",
    "get_parent_terragrunt_dir",
    "path_relative_to_include",
    "path_relative_from_include",
    "read_terragrunt_config",
    "get_repo_root",
    "get_terragrunt_dir",
    "remote_state",
    "dependency \"",
    "dependency{",
    "include \"",
    "include {",
    "expose =",
    "deep_merge",
    "skip_dependencies",
    "merge_strategy",
    "generate \"",
];

const HELPERS: &[&str] = &[
    "find_in_parent_folders(",
    "get_parent_terragrunt_dir(",
    "get_terragrunt_dir(",
    "path_relative_to_include(",
    "path_relative_from_include(",
    "read_terragrunt_config(",
    "get_repo_root(",
    "get_env(",
    "get_platform(",
    "get_aws_account_id(",
    "get_terraform_commands_that_need_vars(",
    "sops_decrypt_file(",
];

fn code_of(s: &str) -> &str {
    let mut s = s;
    if let Some(i) = s.find("//") {
        s = &s[..i];
    }
    if let Some(i) = s.find('#') {
        s = &s[..i];
    }
    s.trim()
}

/// Detect terragrunt.hcl content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut hits = 0usize;
    for line in t.lines() {
        let c = code_of(line);
        if c.is_empty() {
            continue;
        }
        if STRONG_TOKENS.iter().any(|tok| c.contains(tok)) {
            hits += 1;
        }
    }
    hits >= 1
}

impl Terragrunt {
    /// Census a terragrunt.hcl buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            includes: 0,
            dependencies: 0,
            blocks: 0,
            helper_calls: 0,
            assignments: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with('#') || s.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            let code = code_of(s);
            if code.is_empty() {
                continue;
            }
            if code.starts_with("include ") || code.starts_with("include{") || code == "include {" {
                c.includes += 1;
            }
            if code.starts_with("dependency ")
                || code.starts_with("dependency{")
                || code.starts_with("dependencies")
            {
                c.dependencies += 1;
            }
            if code.starts_with("remote_state")
                || code.starts_with("generate ")
                || code.starts_with("terraform ")
                || code.starts_with("iam_")
                || code.starts_with("download_dir")
            {
                c.blocks += 1;
            }
            c.helper_calls += HELPERS
                .iter()
                .map(|h| code.matches(h).count())
                .sum::<usize>();
            if code.contains('=') && !code.ends_with('{') {
                c.assignments += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_terragrunt() {
        let b = b"include \"root\" {\n  path = find_in_parent_folders()\n}\ninputs = {\n  env = \"prod\"\n}\n";
        assert!(detect(b));
        let c = Terragrunt::parse(b).unwrap();
        assert_eq!(c.includes, 1);
        assert_eq!(c.helper_calls, 1);
    }

    #[test]
    fn detects_remote_state() {
        let b = b"remote_state {\n  backend = \"s3\"\n  config = {\n    bucket = \"x\"\n  }\n}\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_plain_hcl() {
        // Plain terraform has none of the terragrunt-only tokens.
        assert!(!detect(
            b"terraform {\n  required_version = \">= 1.0\"\n}\nresource \"aws_s3_bucket\" \"b\" {}\n"
        ));
        // Comment-only mention must not detect.
        assert!(!detect(
            b"# find_in_parent_folders()\nlocals {\n  x = 1\n}\n"
        ));
    }
}
