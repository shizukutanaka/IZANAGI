//! Renovate `renovate.json` / `.renovaterc` の検出・カウント。
//!
//! `extends`/`packageRules`/`dependencyDashboard`/`enabledManagers`/`schedule`
//! 等 Renovate 固有キーを持つ JSON 設定。
//!
//! ```
//! let cfg = br#"{"extends":["config:recommended"],"packageRules":[],
//!                "dependencyDashboard":true,"enabledManagers":["npm"]}"#;
//! assert!(izanagi_kit::renovate::detect(cfg));
//! let c = izanagi_kit::renovate::parse(cfg).unwrap();
//! assert_eq!(c.entries, 4);
//! ```

/// プリセット・継承系キー。
const EXTENDS: &[&str] = &["extends", "presets", "$schema", "description", "enabled"];

/// パッケージルール・マッチ系キー (プレフィックス判定併用)。
const RULES: &[&str] = &[
    "packagerules",
    "depname",
    "packagename",
    "packagepattern",
    "versioning",
    "rangestrategy",
    "updatetype",
    "extractversion",
    "filematch",
    "managerfilepatterns",
    "matchfilenames",
    "matchpaths",
    "allowedversions",
    "followtag",
    "respectlatest",
    "ignoredeps",
    "ignorepaths",
    "ignoreunstable",
    "pindigests",
    "digest",
    "ignorenpmrcfile",
    "minimumgroupsize",
    "groupname",
    "groupslug",
    "transitiveremediation",
    "vulnerabilityalerts",
    "osvvulnerabilityalerts",
    "vulnerabilitystrategy",
];

/// スケジュール・自動マージ系キー。
const SCHEDULE: &[&str] = &[
    "schedule",
    "timezone",
    "before",
    "after",
    "automerge",
    "automergetype",
    "automergeschedule",
    "automergestrategy",
    "rebasewhen",
    "rebaselabel",
    "stopupdatinglabel",
    "prcreation",
    "prconcurrentlimit",
    "branchconcurrentlimit",
    "recreatewhen",
    "recreateclosed",
];

/// プラットフォーム・認証系キー。
const PLATFORM: &[&str] = &[
    "platform",
    "endpoint",
    "token",
    "username",
    "gitauthor",
    "repositories",
    "onboarding",
    "onboardingbranch",
    "onboardingcommitmessage",
    "onboardingprtitle",
    "onboardingnodeps",
    "requireconfig",
    "forkprocessing",
    "forktoken",
    "includemirrors",
    "platformautomerge",
    "gitlabignoreapprovals",
    "hostrules",
    "registryurls",
    "defaultregistryurls",
    "customendpoints",
    "localdir",
    "optimizefordisabled",
    "persistrepodata",
    "repositorycache",
];

/// ダッシュボード・メタ系キー。
const UX: &[&str] = &[
    "dependencydashboard",
    "dependencydashboardtitle",
    "dependencydashboardosvvulnerabilitysummary",
    "enabledmanagers",
    "labels",
    "assignees",
    "reviewers",
    "assigneesfromcodeowners",
    "reviewersfromcodeowners",
    "additionalassignees",
    "additionalreviewers",
    "commitbodytable",
    "separateminorpatch",
    "separatemplemajor",
    "separatemajorminor",
    "separatepatchreleases",
    "skipinstalls",
    "semanticcommits",
    "prheader",
    "prfooter",
    "prbodydefinitions",
    "prbodytemplate",
    "prtitle",
    "prtitlestrict",
    "suppressnotifications",
    "unicodeemoji",
    "emoji",
    "bumpversion",
    "postupdateoptions",
    "enabledmanagersfilepatterns",
    "constraints",
    "vulnerabilityseverity",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// JSON キートークン総数。
    pub entries: usize,
    /// `extends`/`presets`/`$schema` 等継承系キー数。
    pub extends: usize,
    /// `packageRules`/`match*`/`exclude*`/`versioning` 等パッケージルール系キー数。
    pub rules: usize,
    /// `schedule`/`automerge*`/`pr*` 等スケジュール・自動マージ系キー数。
    pub schedule: usize,
    /// `platform`/`token`/`onboarding*`/`hostRules` 等プラットフォーム系キー数。
    pub platform: usize,
    /// `dependencyDashboard`/`labels`/`assignees` 等 UI・メタ系キー数。
    pub ux: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が Renovate 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 3)
}

/// キートークンを走査 (`"name"` の直後が `:` のもの)。
fn keys(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            let mut esc = false;
            while j < bytes.len() {
                if esc {
                    esc = false;
                } else if bytes[j] == 0x5C {
                    esc = true;
                } else if bytes[j] == b'"' {
                    break;
                }
                j += 1;
            }
            let mut k = j + 1;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b':' && j > start {
                out.push(text[start..j].to_ascii_lowercase());
            }
            i = k;
        } else {
            i += 1;
        }
    }
    out
}

/// `b` を `renovate.json` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let trimmed = text.trim_start();
    if !trimmed.starts_with('{') {
        return None;
    }
    let keys = keys(text);
    if keys.is_empty() {
        return None;
    }
    let mut c = Counts {
        entries: keys.len(),
        extends: 0,
        rules: 0,
        schedule: 0,
        platform: 0,
        ux: 0,
        misc: 0,
    };
    for k in &keys {
        let k = k.as_str();
        if EXTENDS.contains(&k) {
            c.extends += 1;
        } else if RULES.contains(&k)
            || k.starts_with("match")
            || k.starts_with("exclude")
            || k.starts_with("allowed")
        {
            c.rules += 1;
        } else if SCHEDULE.contains(&k)
            || (k.starts_with("pr") && k.len() > 2)
            || k.starts_with("automerge")
            || k.starts_with("rebase")
            || k.starts_with("branch")
            || k.starts_with("commit")
        {
            c.schedule += 1;
        } else if PLATFORM.contains(&k) || k.starts_with("onboarding") {
            c.platform += 1;
        } else if UX.contains(&k) || k.starts_with("dependencydashboard") {
            c.ux += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.entries - c.misc >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn renovate() {
        let cfg = br#"{"$schema":"x","extends":["a"],"packageRules":[{"matchPackageNames":["x"]}],"enabledManagers":["npm"],"dependencyDashboard":true,"schedule":["at any time"],"automerge":true,"platform":"github","token":"t","onboarding":false,"hostRules":[]}"#;
        let c = parse(cfg).unwrap();
        assert_eq!(c.entries, 12);
        assert_eq!(c.extends, 2);
        assert_eq!(c.rules, 2);
        assert_eq!(c.schedule, 2);
        assert_eq!(c.platform, 4);
        assert_eq!(c.ux, 2);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_renovate() {
        assert!(parse(br#"{"a":1}"#).is_none());
        assert!(parse(b"not json").is_none());
    }
}
