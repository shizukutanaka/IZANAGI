//! Cypress config (`cypress.config.js`/`.ts`, legacy `cypress.json`) parser.
//!
//! Detects `defineConfig`-style or `cypress.json` configs by Cypress's
//! characteristic keys (`baseUrl`/`specPattern`/`supportFile`/`fixturesFolder`/
//! `viewportWidth`/`video`/`retries`/`defaultCommandTimeout`/`e2e`/`component`/
//! `setupNodeEvents`/`env`/`excludeSpecPattern` …) and counts key
//! occurrences by category.
//!
//! ```
//! let b = b"const { defineConfig } = require('cypress');\nmodule.exports = defineConfig({\n  e2e: {\n    baseUrl: 'http://localhost:3000',\n    specPattern: 'cypress/e2e/**/*.cy.ts',\n    supportFile: 'cypress/support/e2e.ts',\n  },\n});\n";
//! assert!(izanagi_kit::cypressconf::detect(b));
//! let c = izanagi_kit::cypressconf::Cypress::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed Cypress config summary.
#[derive(Debug, Clone)]
pub struct Cypress {
    /// Recognized config key occurrences.
    pub keys: usize,
    /// Spec/support-file keys (`specPattern`/`excludeSpecPattern`/`supportFile`/`supportFolder`/`fixturesFolder`/`downloadsFolder`/`screenshotsFolder`/`videosFolder`/`indexHtmlFile`/`devServer`/`componentIndexHtmlFile`/`slowTestThreshold`/`resolvedNodePath`/`resolvedNodeVersion`/`experimentalCspAllowList`).
    pub path_keys: usize,
    /// Viewport/recording keys (`viewportWidth`/`viewportHeight`/`video`/`videoCompression`/`screenshotOnRunFailure`/`trashAssetsBeforeRuns`/`numTestsKeptInMemory`/`watchForFileChanges`/`chromeWebSecurity`/`modifyObstructiveCode`/`includeShadowDom`/`scrollBehavior`/`waitForAnimations`/`animationDistanceThreshold`/`testIsolation`).
    pub media_keys: usize,
    /// Timeout/retry keys (`defaultCommandTimeout`/`execTimeout`/`taskTimeout`/`pageLoadTimeout`/`requestTimeout`/`responseTimeout`/`retries`/`redirectionLimit`).
    pub timing_keys: usize,
    /// Block/env keys (`e2e`/`component`/`env`/`reporter`/`reporterOptions`/`setupNodeEvents`/`baseUrl`/`experimentalStudio`/`experimentalOriginDependencies`/`experimentalModifyObstructiveThirdPartyCode`/`experimentalSourceRewriting`/`experimentalInteractiveRunEvents`/`experimentalRunAllSpecs`/`experimentalMemoryManagement`/`experimentalFetchPolyfill`/`experimentalWebKitSupport`/`experimentalSkipDomainInjection`/`clientCertificates`/`hosts`/`isInteractive`/`platform`/`testingType`/`userAgent`/`version`/`morgan`/`port`/`proxyUrl`/`remote`/`resolved`/`state`/`testFiles`/`integrationFolder`/`pluginsFile`/`ignoreTestFiles`/`blacklistHosts`/`firefoxGcInterval`).
    pub block_keys: usize,
    /// `//`/`/*`/`#` comment lines.
    pub comments: usize,
}

/// Spec/support-file keys.
const PATH_KEYS: &[&str] = &[
    "specPattern",
    "excludeSpecPattern",
    "supportFile",
    "supportFolder",
    "fixturesFolder",
    "downloadsFolder",
    "screenshotsFolder",
    "videosFolder",
    "indexHtmlFile",
    "devServer",
    "componentIndexHtmlFile",
    "slowTestThreshold",
];

/// Viewport/recording keys.
const MEDIA_KEYS: &[&str] = &[
    "viewportWidth",
    "viewportHeight",
    "video",
    "videoCompression",
    "screenshotOnRunFailure",
    "trashAssetsBeforeRuns",
    "numTestsKeptInMemory",
    "watchForFileChanges",
    "chromeWebSecurity",
    "modifyObstructiveCode",
    "includeShadowDom",
    "scrollBehavior",
    "waitForAnimations",
    "animationDistanceThreshold",
    "testIsolation",
];

/// Timeout/retry keys.
const TIMING_KEYS: &[&str] = &[
    "defaultCommandTimeout",
    "execTimeout",
    "taskTimeout",
    "pageLoadTimeout",
    "requestTimeout",
    "responseTimeout",
    "retries",
    "redirectionLimit",
];

/// Top-level block/env/experimental keys.
const BLOCK_KEYS: &[&str] = &[
    "e2e",
    "component",
    "env",
    "reporter",
    "reporterOptions",
    "setupNodeEvents",
    "baseUrl",
    "experimentalStudio",
    "experimentalOriginDependencies",
    "experimentalModifyObstructiveThirdPartyCode",
    "experimentalSourceRewriting",
    "experimentalInteractiveRunEvents",
    "experimentalRunAllSpecs",
    "experimentalMemoryManagement",
    "experimentalFetchPolyfill",
    "experimentalWebKitSupport",
    "experimentalSkipDomainInjection",
    "clientCertificates",
    "hosts",
    "testingType",
    "userAgent",
    "testFiles",
    "integrationFolder",
    "pluginsFile",
    "ignoreTestFiles",
    "blacklistHosts",
    "firefoxGcInterval",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["cypress", "defineConfig", "e2e", "component", "baseUrl"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "baseUrl",
    "specPattern",
    "supportFile",
    "fixturesFolder",
    "viewportWidth",
    "video",
    "retries",
    "defaultCommandTimeout",
    "e2e",
    "component",
    "setupNodeEvents",
    "excludeSpecPattern",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("\"{k}\""))
        || t.contains(&format!("{k}:"))
        || t.contains(&format!("{k} :"))
        || t.contains(&format!("{k} ="))
}

/// Detect a Cypress config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Cypress {
    /// Count key categories in a Cypress config. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            path_keys: 0,
            media_keys: 0,
            timing_keys: 0,
            block_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with("/*") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in PATH_KEYS {
            c.path_keys += t.matches(k).count();
        }
        for k in MEDIA_KEYS {
            c.media_keys += t.matches(k).count();
        }
        for k in TIMING_KEYS {
            c.timing_keys += t.matches(k).count();
        }
        for k in BLOCK_KEYS {
            c.block_keys += t.matches(k).count();
        }
        c.keys = c.path_keys + c.media_keys + c.timing_keys + c.block_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"// cypress\nconst { defineConfig } = require('cypress');\nmodule.exports = defineConfig({\n  viewportWidth: 1280,\n  viewportHeight: 720,\n  video: false,\n  retries: 1,\n  defaultCommandTimeout: 8000,\n  e2e: {\n    baseUrl: 'http://localhost:3000',\n    specPattern: 'cypress/e2e/**/*.cy.ts',\n    supportFile: 'cypress/support/e2e.ts',\n    setupNodeEvents(on, config) { return config; },\n  },\n});\n";
        assert!(detect(b));
        let c = Cypress::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.path_keys >= 2);
        assert!(c.media_keys >= 3);
        assert!(c.timing_keys >= 2);
        assert!(c.block_keys >= 3);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_random_js() {
        assert!(!detect(b"const x = 1;\nconsole.log(x);\n"));
        assert!(Cypress::parse(b"a = b\n").is_none());
    }
}
