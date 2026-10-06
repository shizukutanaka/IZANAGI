//! Packit `.packit.yaml`/`packit.yaml`/`packit.yml` census.
//!
//! Packit top-level keys: `specfile_path`, `upstream_project_url`,
//! `upstream_package_name`, `downstream_package_name`,
//! `upstream_tag_template`, `upstream_tag_include`,
//! `upstream_tag_exclude`, `synced_files`, `files_to_sync`,
//! `copy_upstream_release_description`, `jobs`, `actions`,
//! `targets`, `merge_pr_in_ci`, `srpm_build_deps`,
//! `issue_repository`, `release_suffix`, `update_release`,
//! `allowed_pr_authors`, `allowed_committers`,
//! `tmt_plan`, `tf_extra_params`, `module_hotfixes`,
//! `dist_git_base_url`, `patch_generation_ignore_paths`,
//! `patch_generation_patch_id_digits`, `notifications`,
//! `upstream_ref`, `sig`, `config_file_path`,
//! `spec_source_id`, `produce_in_packit_instance`,
//! `enable_net`, `id`, `identifier`, `images`, `rpm_build`, `brew`.
//!
//! ```rust
//! let k = b"specfile_path: pkg.spec\nupstream_project_url: https://github.com/x/y\njobs:\n- job: copr_build\n  trigger: pull_request\n";
//! assert!(izanagi_kit::packit::detect(k));
//! ```

/// Packit config census.
#[derive(Debug, Clone)]
pub struct Packit {
    /// `jobs:` entries.
    pub jobs: usize,
    /// `synced_files`/`files_to_sync`/`actions`/`targets` items.
    pub lists: usize,
    /// Recognised top-level keys present.
    pub sections: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "specfile_path",
    "upstream_project_url",
    "upstream_package_name",
    "downstream_package_name",
    "upstream_tag_template",
    "upstream_tag_include",
    "upstream_tag_exclude",
    "synced_files",
    "files_to_sync",
    "copy_upstream_release_description",
    "jobs",
    "actions",
    "merge_pr_in_ci",
    "srpm_build_deps",
    "issue_repository",
    "release_suffix",
    "update_release",
    "allowed_pr_authors",
    "allowed_committers",
    "tmt_plan",
    "tf_extra_params",
    "module_hotfixes",
    "dist_git_base_url",
    "patch_generation_ignore_paths",
    "patch_generation_patch_id_digits",
    "notifications",
    "upstream_ref",
    "sig",
    "config_file_path",
    "spec_source_id",
    "produce_in_packit_instance",
    "enable_net",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => Some(s[..i].trim()),
        None => None,
    }
}

/// Detect a Packit config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut hits = 0usize;
    let mut has_jobs = false;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            // `jobs`/`actions`/`targets` are shared with other CI configs;
            // only packit-exclusive keys count toward detection.
            if !matches!(k, "jobs" | "actions" | "targets") && KEYS.contains(&k) {
                hits += 1;
            }
            if k == "jobs" {
                has_jobs = true;
            }
        }
    }
    hits >= 2 || (hits >= 1 && has_jobs)
}

impl Packit {
    /// Census a Packit buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            jobs: 0,
            lists: 0,
            sections: 0,
            settings: 0,
            comments: 0,
        };
        let mut section = "";
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = top_key(line) {
                section = k;
                if KEYS.contains(&k) || k == "targets" {
                    c.sections += 1;
                }
                c.settings += 1;
                continue;
            }
            if s.starts_with("- ") {
                match section {
                    "jobs" => c.jobs += 1,
                    "synced_files" | "files_to_sync" | "actions" | "targets" => {
                        c.lists += 1;
                    }
                    _ => {}
                }
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_packit() {
        let b = b"specfile_path: pkg.spec\nupstream_project_url: https://github.com/x/y\njobs:\n- job: copr_build\n  trigger: pull_request\n- job: propose_downstream\n";
        assert!(detect(b));
        let c = Packit::parse(b).unwrap();
        assert_eq!(c.jobs, 2);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"jobs:\n  build:\n    steps: []\n"));
        assert!(!detect(b"key: value\nlist:\n- a\n"));
        // Comment-only mention.
        assert!(!detect(b"# specfile_path: x\n# jobs:\nkind: y\n"));
    }
}
