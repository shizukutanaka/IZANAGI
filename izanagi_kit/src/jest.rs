//! Jest config (`jest.config.js`/`.ts`/`package.json` `"jest"` block) parser.
//!
//! Detects `module.exports`-style configs or an embedded `"jest"` object by
//! the characteristic keys (`testEnvironment`/`testMatch`/`testRegex`/
//! `collectCoverage`/`setupFiles`/`moduleNameMapper`/`transform`/`preset`/
//! `projects`/`coverageThreshold` …) and counts key occurrences by category.
//!
//! ```
//! let b = b"module.exports = {\n  testEnvironment: 'node',\n  testMatch: ['**/*.test.js'],\n  collectCoverage: true,\n  coverageThreshold: { global: { lines: 80 } },\n};\n";
//! assert!(izanagi_kit::jest::detect(b));
//! let c = izanagi_kit::jest::Jest::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed Jest config summary.
#[derive(Debug, Clone)]
pub struct Jest {
    /// Recognized config key occurrences.
    pub keys: usize,
    /// Test-path pattern keys (`testMatch`/`testRegex`/`roots`/`testPathIgnorePatterns`/`setupFiles*`/`globalSetup`/`globalTeardown`/`transform`/`moduleNameMapper`/`moduleFileExtensions`/`moduleDirectories`/`modulePaths`/`projects`/`watchPathIgnorePatterns`).
    pub pattern_keys: usize,
    /// Coverage keys (`collectCoverage*`/`coverage*`).
    pub coverage_keys: usize,
    /// Environment/runtime keys (`testEnvironment`/`preset`/`runner`/`testRunner`/`sandboxInjectedGlobals`/`injectGlobals`/`testLocationInResults`/`globals`/`setupFilesAfterEach`).
    pub env_keys: usize,
    /// `//`/`/*`/`#` comment lines.
    pub comments: usize,
}

/// Test-path pattern keys.
const PATTERN_KEYS: &[&str] = &[
    "testMatch",
    "testRegex",
    "roots",
    "testPathIgnorePatterns",
    "setupFiles",
    "setupFilesAfterEach",
    "globalSetup",
    "globalTeardown",
    "transform",
    "moduleNameMapper",
    "moduleFileExtensions",
    "moduleDirectories",
    "modulePaths",
    "projects",
    "watchPathIgnorePatterns",
    "resolver",
];

/// Coverage keys.
const COVERAGE_KEYS: &[&str] = &[
    "collectCoverage",
    "coverageDirectory",
    "coveragePathIgnorePatterns",
    "coverageProvider",
    "coverageReporters",
    "coverageThreshold",
    "collectCoverageFrom",
];

/// Environment/runtime keys.
const ENV_KEYS: &[&str] = &[
    "testEnvironment",
    "preset",
    "runner",
    "testRunner",
    "sandboxInjectedGlobals",
    "injectGlobals",
    "globals",
    "displayName",
    "bail",
    "clearMocks",
    "resetMocks",
    "restoreMocks",
    "verbose",
    "silent",
    "errorOnDeprecated",
    "maxWorkers",
    "maxConcurrency",
    "slowTestThreshold",
    "testTimeout",
    "watchman",
];

/// All recognized keys (detect requires ≥2).
const ALL: &[&str] = &[
    "testEnvironment",
    "testMatch",
    "testRegex",
    "collectCoverage",
    "setupFiles",
    "moduleNameMapper",
    "transform",
    "preset",
    "projects",
    "coverageThreshold",
    "roots",
    "reporters",
    "watchPlugins",
    "snapshotSerializers",
    "jest",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("\"{k}\""))
        || t.contains(&format!("{k}:"))
        || t.contains(&format!("{k} :"))
        || t.contains(&format!("{k} ="))
}

/// Detect a `jest.config.*` or `"jest"`-block style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    hits >= 2 || (t.contains("\"jest\"") && hits >= 1)
}

impl Jest {
    /// Count key categories in a Jest config. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            pattern_keys: 0,
            coverage_keys: 0,
            env_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with("/*") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in PATTERN_KEYS {
            c.pattern_keys += t.matches(k).count();
        }
        for k in COVERAGE_KEYS {
            c.coverage_keys += t.matches(k).count();
        }
        for k in ENV_KEYS {
            c.env_keys += t.matches(k).count();
        }
        for k in ALL {
            c.keys += t.matches(k).count();
        }
        c.keys += c.pattern_keys + c.coverage_keys + c.env_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts_js() {
        let b = b"// jest config\nmodule.exports = {\n  preset: 'ts-jest',\n  testEnvironment: 'node',\n  roots: ['<rootDir>/src'],\n  testMatch: ['**/*.test.ts'],\n  collectCoverage: true,\n  collectCoverageFrom: ['src/**/*.ts'],\n  coverageThreshold: { global: { lines: 80 } },\n  setupFilesAfterEach: ['./jest.setup.js'],\n  transform: { '^.+\\\\.tsx?$': 'ts-jest' },\n  moduleNameMapper: { '^@/(.*)$': '<rootDir>/src/$1' },\n  verbose: true,\n};\n";
        assert!(detect(b));
        let c = Jest::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.pattern_keys >= 6);
        assert!(c.coverage_keys >= 3);
        assert!(c.env_keys >= 3);
        assert!(c.keys >= 10);
    }

    #[test]
    fn detects_package_jest() {
        let b = br#"{"name": "app", "jest": {"testEnvironment": "jsdom", "testMatch": ["**/*.test.tsx"], "collectCoverage": true}}"#;
        assert!(detect(b));
        let c = Jest::parse(b).unwrap();
        assert!(c.env_keys >= 1);
        assert!(c.pattern_keys >= 1);
    }

    #[test]
    fn rejects_random_js() {
        assert!(!detect(b"const x = 1;\nconsole.log(x);\n"));
        assert!(Jest::parse(b"a = b\n").is_none());
    }
}
