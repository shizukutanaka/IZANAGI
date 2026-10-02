//! Coveralls `.coveralls.yml` の検出・カウント。
//!
//! `repo_token`/`service_*`/`flag_name`/`parallel`/`carryforward`/`git`
//! 等 Coveralls 固有キーを持つ YAML 設定。
//!
//! ```
//! let cfg = b"repo_token: x\nservice_name: github-actions\nflag_name: unit\nparallel: true\n";
//! assert!(izanagi_kit::coveralls::detect(cfg));
//! let c = izanagi_kit::coveralls::parse(cfg).unwrap();
//! assert_eq!(c.entries, 4);
//! ```

/// サービス系キー。
const SERVICE: &[&str] = &[
    "service_name",
    "service_number",
    "service_job_id",
    "service_job_number",
    "service_build_url",
    "service_branch",
    "service_pull_request",
    "service_event_name",
    "service_event_type",
    "service_folder",
    "service_flag_name",
];

/// VCS 系キー。
const VCS: &[&str] = &[
    "git",
    "branch",
    "commit_sha",
    "repo_name",
    "repo_base_url",
    "base",
    "compare",
];

/// 実行系キー。
const RUN: &[&str] = &[
    "repo_token",
    "flag_name",
    "flags",
    "parallel",
    "carryforward",
    "base_path",
    "root_dir",
    "measure",
    "format",
    "source_encoding",
    "environment",
    "filename",
    "output_filename",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルキー総数。
    pub entries: usize,
    /// `service_*` 系キー数。
    pub service: usize,
    /// `git`/`branch`/`commit_sha` 等 VCS 系キー数。
    pub vcs: usize,
    /// `repo_token`/`flag_name`/`parallel`/`carryforward` 等実行系キー数。
    pub run: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が Coveralls 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.service + c.run + c.vcs >= 2)
}

/// トップレベルキー名を列挙。
fn top_keys(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with([' ', '\t', '-', '#']) || line.trim().is_empty() {
            continue;
        }
        if let Some(end) = line.find(':') {
            let key = line[..end].trim();
            if !key.is_empty() {
                out.push(key);
            }
        }
    }
    out
}

/// `b` を `.coveralls.yml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let keys = top_keys(text);
    if keys.is_empty() {
        return None;
    }
    let mut c = Counts {
        entries: keys.len(),
        service: 0,
        vcs: 0,
        run: 0,
        misc: 0,
    };
    for k in keys {
        if SERVICE.contains(&k) || k.starts_with("service_") {
            c.service += 1;
        } else if VCS.contains(&k) {
            c.vcs += 1;
        } else if RUN.contains(&k) {
            c.run += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.service + c.run + c.vcs >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn coveralls() {
        let cfg = b"repo_token: x\nservice_name: gha\nservice_job_id: j1\nflag_name: u\nparallel: true\ncarryforward: f1,f2\ngit:\n  branch: main\nbranch: dev\nother: 9\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.entries, 9);
        assert_eq!(c.service, 2);
        assert_eq!(c.run, 4);
        assert_eq!(c.vcs, 2);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_coveralls() {
        assert!(parse(b"foo: 1\n").is_none());
    }
}
