//! Playwright Test config (`playwright.config.ts`) parser.
//!
//! Detects `defineConfig`/`@playwright/test` configs by the characteristic
//! keys (`testDir`/`testMatch`/`retries`/`workers`/`reporter`/`use`/
//! `projects`/`webServer`/`timeout`/`expect`/`forbidOnly`/`fullyParallel`/
//! `globalSetup`/`outputDir` …) and counts key occurrences by category.
//!
//! ```
//! let b = b"import { defineConfig } from '@playwright/test';\nexport default defineConfig({\n  testDir: './tests',\n  retries: 2,\n  workers: 4,\n  reporter: 'html',\n  use: { baseURL: 'http://localhost' },\n});\n";
//! assert!(izanagi_kit::playwrightconf::detect(b));
//! let c = izanagi_kit::playwrightconf::Playwright::parse(b).unwrap();
//! assert!(c.keys >= 5);
//! ```

/// Parsed Playwright config summary.
#[derive(Debug, Clone)]
pub struct Playwright {
    /// Recognized config key occurrences.
    pub keys: usize,
    /// Path keys (`testDir`/`testMatch`/`testIgnore`/`outputDir`/`snapshotDir`/`globalSetup`/`globalTeardown`/`snapshotPathTemplate`/`testIdAttribute`).
    pub path_keys: usize,
    /// Execution keys (`timeout`/`globalTimeout`/`forbidOnly`/`fullyParallel`/`retries`/`workers`/`repeatEach`/`maxFailures`/`quiet`/`grep`/`grepInvert`/`updateSnapshots`/`preserveOutput`/`reportSlowTests`/`shard`/`failOnFlakyTests`/`tsconfig`/`dependencies`).
    pub exec_keys: usize,
    /// `use:`-block / context keys (`baseURL`/`trace`/`screenshot`/`video`/`actionTimeout`/`navigationTimeout`/`locale`/`timezoneId`/`geolocation`/`permissions`/`viewport`/`userAgent`/`deviceScaleFactor`/`isMobile`/`hasTouch`/`javaScriptEnabled`/`ignoreHTTPSErrors`/`extraHTTPHeaders`/`httpCredentials`/`offline`/`colorScheme`/`reducedMotion`/`serviceWorkers`/`acceptDownloads`/`bypassCSP`/`launchOptions`/`connectOptions`/`storageState`/`proxy`/`recordVideo`/`recordHar`/`screencast`/`contextOptions`/`headless`/`channel`/`testIdAttribute`).
    pub use_keys: usize,
    /// Reporter/project keys (`reporter`/`reporters`/`projects`/`webServer`/`metadata`/`name`/`expect`/`version`/`captureGitInfo`).
    pub misc_keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

/// Path keys.
const PATH_KEYS: &[&str] = &[
    "testDir",
    "testMatch",
    "testIgnore",
    "outputDir",
    "snapshotDir",
    "globalSetup",
    "globalTeardown",
    "snapshotPathTemplate",
    "testIdAttribute",
];

/// Execution keys.
const EXEC_KEYS: &[&str] = &[
    "timeout",
    "globalTimeout",
    "forbidOnly",
    "fullyParallel",
    "retries",
    "workers",
    "repeatEach",
    "maxFailures",
    "quiet",
    "grep",
    "grepInvert",
    "updateSnapshots",
    "preserveOutput",
    "reportSlowTests",
    "shard",
    "failOnFlakyTests",
    "tsconfig",
    "dependencies",
];

/// `use:`-block / browser context keys.
const USE_KEYS: &[&str] = &[
    "baseURL",
    "trace",
    "screenshot",
    "video",
    "actionTimeout",
    "navigationTimeout",
    "locale",
    "timezoneId",
    "geolocation",
    "permissions",
    "viewport",
    "userAgent",
    "deviceScaleFactor",
    "isMobile",
    "hasTouch",
    "javaScriptEnabled",
    "ignoreHTTPSErrors",
    "extraHTTPHeaders",
    "httpCredentials",
    "offline",
    "colorScheme",
    "reducedMotion",
    "serviceWorkers",
    "acceptDownloads",
    "bypassCSP",
    "launchOptions",
    "connectOptions",
    "storageState",
    "proxy",
    "recordVideo",
    "recordHar",
    "screencast",
    "contextOptions",
    "headless",
    "channel",
];

/// Reporter/project/misc keys.
const MISC_KEYS: &[&str] = &[
    "reporter",
    "reporters",
    "projects",
    "webServer",
    "metadata",
    "name",
    "expect",
    "version",
    "captureGitInfo",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["playwright", "defineConfig", "testDir", "projects"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "testDir",
    "testMatch",
    "retries",
    "workers",
    "reporter",
    "use",
    "projects",
    "webServer",
    "timeout",
    "forbidOnly",
    "fullyParallel",
    "globalSetup",
    "outputDir",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("\"{k}\""))
        || t.contains(&format!("{k}:"))
        || t.contains(&format!("{k} :"))
        || t.contains(&format!("{k} ="))
}

/// Detect a Playwright Test config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Playwright {
    /// Count key categories in a Playwright config. Returns `None` when the
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
            exec_keys: 0,
            use_keys: 0,
            misc_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with("/*") {
                c.comments += 1;
            }
        }
        for k in PATH_KEYS {
            c.path_keys += t.matches(k).count();
        }
        for k in EXEC_KEYS {
            c.exec_keys += t.matches(k).count();
        }
        for k in USE_KEYS {
            c.use_keys += t.matches(k).count();
        }
        for k in MISC_KEYS {
            c.misc_keys += t.matches(k).count();
        }
        c.keys = c.path_keys + c.exec_keys + c.use_keys + c.misc_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"// playwright\nimport { defineConfig } from '@playwright/test';\nexport default defineConfig({\n  testDir: './e2e',\n  testMatch: '*.spec.ts',\n  timeout: 30000,\n  retries: 2,\n  workers: 4,\n  fullyParallel: true,\n  reporter: [['html'], ['junit', { outputFile: 'r.xml' }]],\n  use: { baseURL: 'http://x', trace: 'on', screenshot: 'only-on-failure' },\n  projects: [{ name: 'chromium', use: { channel: 'chrome' } }],\n  webServer: { command: 'npm start', port: 3000 },\n});\n";
        assert!(detect(b));
        let c = Playwright::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.path_keys >= 2);
        assert!(c.exec_keys >= 3);
        assert!(c.use_keys >= 4);
        assert!(c.misc_keys >= 3);
        assert!(c.keys >= 12);
    }

    #[test]
    fn rejects_random_ts() {
        assert!(!detect(b"const x = 1;\nconsole.log(x);\n"));
        assert!(Playwright::parse(b"a = b\n").is_none());
    }
}
