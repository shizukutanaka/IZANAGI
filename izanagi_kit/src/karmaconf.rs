//! Karma runner config (`karma.conf.js`/`.coffee`) parser.
//!
//! Detects `config.set({...})` style configs by Karma's characteristic keys
//! (`frameworks`/`files`/`preprocessors`/`reporters`/`browsers`/`singleRun`/
//! `autoWatch`/`port`/`logLevel`/`customLaunchers`/`concurrency`/
//! `captureTimeout` …) and counts key occurrences by category.
//!
//! ```
//! let b = b"module.exports = function(config) {\n  config.set({\n    frameworks: ['jasmine'],\n    files: ['src/**/*.spec.js'],\n    browsers: ['Chrome'],\n    singleRun: true,\n  });\n};\n";
//! assert!(izanagi_kit::karmaconf::detect(b));
//! let c = izanagi_kit::karmaconf::Karma::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed Karma config summary.
#[derive(Debug, Clone)]
pub struct Karma {
    /// Recognized config key occurrences.
    pub keys: usize,
    /// File/preprocessor keys (`files`/`exclude`/`preprocessors`/`mime`/`proxies`/`proxyValidateSSL`/`urlRoot`/`files`/patterns/`served`/`included`/`watched`/`nocache`/`showFileList`).
    pub file_keys: usize,
    /// Framework/browser keys (`frameworks`/`browsers`/`customLaunchers`/`browserStack`/`captureTimeout`/`browserDisconnectTimeout`/`browserDisconnectTolerance`/`browserNoActivityTimeout`/`retryLimit`/`restartOnFileChange`/`concurrency`/`transports`/`forceJSONP`/`plugins`/`protocol`).
    pub browser_keys: usize,
    /// Run/report keys (`reporters`/`port`/`listenAddress`/`hostname`/`colors`/`logLevel`/`autoWatch`/`singleRun`/`failOnEmptyTestSuite`/`failOnFailingTestSuite`/`formatError`/`coverageReporter`/`junitReporter`/`client`/`pingTimeout`/`upstreamProxy`/`beforeMiddleware`/`middleware`/`httpsServerOptions`/`httpModule`/`basePath`/`crossOriginAttribute`/`autoWatchBatchDelay`/`browserConsoleLogOptions`/`customClientContextFile`/`customDebugFile`/`customContextFile`/`detached`/`processKillTimeout`).
    pub run_keys: usize,
    /// `//`/`/*`/`#` comment lines.
    pub comments: usize,
}

/// File/preprocessor keys.
const FILE_KEYS: &[&str] = &[
    "files",
    "exclude",
    "preprocessors",
    "mime",
    "proxies",
    "proxyValidateSSL",
    "urlRoot",
    "served",
    "included",
    "watched",
    "nocache",
    "showFileList",
];

/// Framework/browser keys.
const BROWSER_KEYS: &[&str] = &[
    "frameworks",
    "browsers",
    "customLaunchers",
    "browserStack",
    "captureTimeout",
    "browserDisconnectTimeout",
    "browserDisconnectTolerance",
    "browserNoActivityTimeout",
    "retryLimit",
    "restartOnFileChange",
    "concurrency",
    "transports",
    "forceJSONP",
    "plugins",
    "protocol",
];

/// Run/report keys.
const RUN_KEYS: &[&str] = &[
    "reporters",
    "port",
    "listenAddress",
    "hostname",
    "colors",
    "logLevel",
    "autoWatch",
    "singleRun",
    "failOnEmptyTestSuite",
    "failOnFailingTestSuite",
    "formatError",
    "coverageReporter",
    "junitReporter",
    "client",
    "pingTimeout",
    "upstreamProxy",
    "beforeMiddleware",
    "middleware",
    "httpsServerOptions",
    "httpModule",
    "basePath",
    "crossOriginAttribute",
    "autoWatchBatchDelay",
    "browserConsoleLogOptions",
    "customClientContextFile",
    "customDebugFile",
    "customContextFile",
    "detached",
    "processKillTimeout",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["config.set", "karma", "frameworks", "browsers"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "frameworks",
    "files",
    "preprocessors",
    "reporters",
    "browsers",
    "singleRun",
    "autoWatch",
    "port",
    "logLevel",
    "customLaunchers",
    "concurrency",
    "captureTimeout",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("\"{k}\""))
        || t.contains(&format!("{k}:"))
        || t.contains(&format!("{k} :"))
        || t.contains(&format!("{k} ="))
}

/// Detect a Karma config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Karma {
    /// Count key categories in a Karma config. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            file_keys: 0,
            browser_keys: 0,
            run_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with("/*") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in FILE_KEYS {
            c.file_keys += t.matches(k).count();
        }
        for k in BROWSER_KEYS {
            c.browser_keys += t.matches(k).count();
        }
        for k in RUN_KEYS {
            c.run_keys += t.matches(k).count();
        }
        c.keys = c.file_keys + c.browser_keys + c.run_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"// karma\nmodule.exports = function(config) {\n  config.set({\n    basePath: '',\n    frameworks: ['jasmine', '@angular-devkit/build-angular'],\n    files: ['src/**/*.spec.ts'],\n    exclude: [],\n    preprocessors: { 'src/**/*.ts': ['coverage'] },\n    reporters: ['progress', 'coverage'],\n    coverageReporter: { type: 'html' },\n    port: 9876,\n    colors: true,\n    logLevel: config.LOG_INFO,\n    autoWatch: true,\n    browsers: ['ChromeHeadless'],\n    customLaunchers: { ci: { base: 'ChromeHeadless' } },\n    singleRun: false,\n    concurrency: 4,\n  });\n};\n";
        assert!(detect(b));
        let c = Karma::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.file_keys >= 2);
        assert!(c.browser_keys >= 3);
        assert!(c.run_keys >= 5);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_random_js() {
        assert!(!detect(b"const x = 1;\nconsole.log(x);\n"));
        assert!(Karma::parse(b"a = b\n").is_none());
    }
}
