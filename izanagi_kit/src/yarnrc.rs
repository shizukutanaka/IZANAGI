//! Yarn `.yarnrc.yml` census (Yarn ≥2/Berry).
//!
//! Top-level keys: `nodeLinker`, `npmRegistryServer`, `yarnPath`,
//! `plugins`, `packageExtensions`, `enableGlobalCache`,
//! `compressionLevel`, `cacheFolder`, `pnpMode`, `pnpShebang`,
//! `supportedArchitectures`, `unsafeHttpWhitelist`, `logFilters`,
//! `changesetBaseRefs`, `defaultSemverRangePrefix`,
//! `preferInteractive`, `checksumBehavior`, `patchFolder`,
//! `lockfileFilename`, `installStatePath`, `virtualFolder`,
//! `pnpDataPath`, `pnpUnpluggedFolder`, `preferWorkspacePackages`,
//! `networkConcurrency`, `httpProxy`, `httpsProxy`, `httpTimeout`,
//! `httpRetry`, `nmMode`, `nmHoistingLimits`, `nmSelfReferences`,
//! `linkType`, `telemetryUserId`, `enableTelemetry`,
//! `enableImmutableInstalls`, `enableImmutableCache`,
//! `caFilePath`, `certFilePath`, `keyFilePath`, `gitBinaryPath`,
//! `cloneConcurrency`, `initScope`, `initFields`,
//! `constraintsPath`, `injectEnvironmentFiles`, `languageName`,
//! `enableMirror`, `enableNetwork`, `globalFolder`,
//! `installStatePath`, `lastUpdateCheck`, `networkSettings`,
//! `npmAlwaysAuth`, `npmAuditRegistry`, `npmAuthIdent`,
//! `npmAuthToken`, `npmPublishAccess`, `npmPublishRegistry`,
//! `npmScopes`, `preferTruncatedLines`, `preferDeferred`,
//! `supportedArchitectures`, `winLinkType`, `defaultSemverRangePrefix`,
//! `deferDependencyResolution`, `injectEnvironmentFiles`.
//!
//! ```rust
//! let k = b"nodeLinker: pnp\nnpmRegistryServer: \"https://r\"\nyarnPath: .yarn/releases/yarn-4.cjs\n";
//! assert!(izanagi_kit::yarnrc::detect(k));
//! ```

/// `.yarnrc.yml` census.
#[derive(Debug, Clone)]
pub struct Yarnrc {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "nodeLinker",
    "yarnPath",
    "packageExtensions",
    "enableGlobalCache",
    "compressionLevel",
    "cacheFolder",
    "pnpMode",
    "pnpShebang",
    "supportedArchitectures",
    "unsafeHttpWhitelist",
    "logFilters",
    "changesetBaseRefs",
    "defaultSemverRangePrefix",
    "preferInteractive",
    "checksumBehavior",
    "patchFolder",
    "lockfileFilename",
    "installStatePath",
    "virtualFolder",
    "pnpDataPath",
    "pnpUnpluggedFolder",
    "preferWorkspacePackages",
    "nmMode",
    "nmHoistingLimits",
    "nmSelfReferences",
    "linkType",
    "telemetryUserId",
    "enableTelemetry",
    "enableImmutableInstalls",
    "enableImmutableCache",
    "caFilePath",
    "certFilePath",
    "keyFilePath",
    "gitBinaryPath",
    "cloneConcurrency",
    "initScope",
    "initFields",
    "constraintsPath",
    "injectEnvironmentFiles",
    "languageName",
    "enableMirror",
    "globalFolder",
    "lastUpdateCheck",
    "npmAlwaysAuth",
    "npmAuditRegistry",
    "npmAuthIdent",
    "npmAuthToken",
    "npmPublishAccess",
    "npmPublishRegistry",
    "npmScopes",
    "preferTruncatedLines",
    "preferDeferred",
    "winLinkType",
    "deferDependencyResolution",
    "progressBarStyle",
    "nmHardlinksMode",
    "pnpEnableEsmLoader",
    "pnpEnableInlining",
    "pnpFallbackMode",
    "pnpIgnorePatterns",
    "pnpEnableNativeESM",
    "cacheMigrationMode",
    "enableHardenedMode",
    "httpProxy",
    "httpsProxy",
    "httpTimeout",
    "httpRetry",
    "networkConcurrency",
    "networkSettings",
    "enableNetwork",
];

const WEAK: &[&str] = &[
    "npmRegistryServer",
    "plugins",
    "initialPath",
    "ignorePath",
    "preferInteractive",
    "changesetIgnorePatterns",
    "npmRegistry",
    "enableColors",
    "enableHyperlinks",
    "enableInlineBuilds",
    "enableTimers",
    "enableTransparentWorkspaces",
    "enableVersions",
    "preferDeferred",
    "registry",
    "bstatePath",
    "restrictedPackages",
    "minimalNetwork",
    "httpsCertificateFilePath",
    "httpsKeyFilePath",
    "logFilters",
    "packageLogFilters",
    "refreshInterval",
    "supportedArchitectures",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"').trim_matches('\'');
            if k.is_empty() {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a `.yarnrc.yml` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `nodeLinker`/`yarnPath`/`pnp*`/`nm*`/`cacheFolder` keys are
    // Yarn-Berry exclusive.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 1 && strong + weak >= 2
}

impl Yarnrc {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"nodeLinker: pnp\nnpmRegistryServer: \"https://r\"\nyarnPath: .yarn/releases/yarn-4.cjs\n";
        assert!(detect(b));
        let c = Yarnrc::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"name: x\nversion: 1\n"));
        assert!(!detect(b"# nodeLinker: pnp\n# yarnPath: x\nregistry: y\n"));
    }
}
