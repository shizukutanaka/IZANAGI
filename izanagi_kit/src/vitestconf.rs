//! Vitest config (`vitest.config.ts` / `vite.config.ts` `test:` block) parser.
//!
//! Detects `defineConfig`-style or `test:`-block configs by Vitest's
//! characteristic keys (`environment`/`include`/`exclude`/`coverage`/
//! `reporters`/`globals`/`pool`/`setupFiles`/`testTimeout`/`browser`/
//! `typecheck`/`workspace` …) and counts key occurrences by category.
//!
//! ```
//! let b = b"import { defineConfig } from 'vitest/config';\nexport default defineConfig({\n  test: {\n    environment: 'jsdom',\n    include: ['src/**/*.test.ts'],\n    coverage: { provider: 'v8' },\n    reporters: ['default'],\n  },\n});\n";
//! assert!(izanagi_kit::vitestconf::detect(b));
//! let c = izanagi_kit::vitestconf::Vitest::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed Vitest config summary.
#[derive(Debug, Clone)]
pub struct Vitest {
    /// Recognized config key occurrences.
    pub keys: usize,
    /// Path/pattern keys (`include`/`exclude`/`dir`/`root`/`testMatch`/`testNamePattern`/`setupFiles`/`deps`/`alias`/`projects`/`workspace`/`poolMatchGlobs`/`forceRerunTriggers`/`watchExclude`/`exclude`/`includeSource`/`typecheck`/`snapshotDir`/`resolveSnapshotPath`/`snapshotSerializers`).
    pub pattern_keys: usize,
    /// Coverage keys (`coverage`/`provider`/`all`/`thresholds`/`lines`/`functions`/`branches`/`statements`/`watermarks`/`reportsDirectory`/`reportOnFailure`/`skipFull`/`perFile`/`allowExternal`/`processingConcurrency`/`customProviderModule`/`coverage` options).
    pub coverage_keys: usize,
    /// Runtime keys (`environment`/`pool`/`threads`/`maxWorkers`/`minWorkers`/`isolate`/`fileParallelism`/`maxConcurrency`/`testTimeout`/`hookTimeout`/`teardownTimeout`/`globals`/`browser`/`headless`/`slowTestThreshold`/`dangerouslyIgnoreUnhandledErrors`/`expect`/`chaiConfig`/`env`/`poolOptions`).
    pub runtime_keys: usize,
    /// Reporter/output keys (`reporters`/`outputFile`/`ui`/`api`/`silent`/`logHeapUsage`/`passWithNoTests`/`update`/`sequence`/`retry`/`bail`/`benchmark`/`onConsoleLog`/`printConsoleTrace`/`snapshotFormat`/`diff`/`inspect`/`inspectBrk`).
    pub output_keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

/// Path/pattern keys.
const PATTERN_KEYS: &[&str] = &[
    "include",
    "exclude",
    "dir",
    "root",
    "testMatch",
    "testNamePattern",
    "setupFiles",
    "deps",
    "alias",
    "projects",
    "workspace",
    "poolMatchGlobs",
    "forceRerunTriggers",
    "watchExclude",
    "includeSource",
    "typecheck",
    "snapshotDir",
    "resolveSnapshotPath",
    "snapshotSerializers",
];

/// Coverage keys.
const COVERAGE_KEYS: &[&str] = &[
    "coverage",
    "provider",
    "thresholds",
    "lines",
    "functions",
    "branches",
    "statements",
    "watermarks",
    "reportsDirectory",
    "reportOnFailure",
    "skipFull",
    "perFile",
    "allowExternal",
    "processingConcurrency",
    "customProviderModule",
    "clean",
    "cleanOnRerun",
    "extension",
    "ignoreEmptyLines",
];

/// Runtime keys.
const RUNTIME_KEYS: &[&str] = &[
    "environment",
    "pool",
    "threads",
    "maxWorkers",
    "minWorkers",
    "isolate",
    "fileParallelism",
    "maxConcurrency",
    "testTimeout",
    "hookTimeout",
    "teardownTimeout",
    "globals",
    "browser",
    "headless",
    "slowTestThreshold",
    "dangerouslyIgnoreUnhandledErrors",
    "expect",
    "chaiConfig",
    "env",
    "poolOptions",
    "vmThreads",
    "vmForks",
    "forks",
    "singleThread",
    "singleFork",
];

/// Reporter/output keys.
const OUTPUT_KEYS: &[&str] = &[
    "reporters",
    "outputFile",
    "ui",
    "api",
    "silent",
    "logHeapUsage",
    "passWithNoTests",
    "update",
    "sequence",
    "retry",
    "bail",
    "benchmark",
    "onConsoleLog",
    "printConsoleTrace",
    "snapshotFormat",
    "diff",
    "inspect",
    "inspectBrk",
    "expandSnapshotDiff",
    "hideSkippedTests",
];

/// Anchor tokens that identify the file as Vitest config.
const ANCHORS: &[&str] = &["vitest", "defineConfig", "test:", "coverage:"];

/// Distinctive Vitest keys for the detect path.
const ALL: &[&str] = &[
    "environment",
    "include",
    "exclude",
    "coverage",
    "reporters",
    "globals",
    "pool",
    "setupFiles",
    "testTimeout",
    "browser",
    "typecheck",
    "workspace",
    "benchmark",
    "sequence",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("\"{k}\""))
        || t.contains(&format!("{k}:"))
        || t.contains(&format!("{k} :"))
        || t.contains(&format!("{k} ="))
}

/// Detect a Vitest-style config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Vitest {
    /// Count key categories in a Vitest config. Returns `None` when the input
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
            runtime_keys: 0,
            output_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with("/*") {
                c.comments += 1;
            }
        }
        for k in PATTERN_KEYS {
            c.pattern_keys += t.matches(k).count();
        }
        for k in COVERAGE_KEYS {
            c.coverage_keys += t.matches(k).count();
        }
        for k in RUNTIME_KEYS {
            c.runtime_keys += t.matches(k).count();
        }
        for k in OUTPUT_KEYS {
            c.output_keys += t.matches(k).count();
        }
        c.keys = c.pattern_keys + c.coverage_keys + c.runtime_keys + c.output_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts_vitest() {
        let b = b"// vitest\nimport { defineConfig } from 'vitest/config';\nexport default defineConfig({\n  test: {\n    environment: 'jsdom',\n    globals: true,\n    include: ['src/**/*.test.ts'],\n    exclude: ['node_modules'],\n    setupFiles: ['./setup.ts'],\n    coverage: { provider: 'v8', lines: 80 },\n    reporters: ['default', 'junit'],\n    testTimeout: 5000,\n    pool: 'threads',\n    sequence: { shuffle: true },\n  },\n});\n";
        assert!(detect(b));
        let c = Vitest::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.pattern_keys >= 3);
        assert!(c.coverage_keys >= 3);
        assert!(c.runtime_keys >= 3);
        assert!(c.output_keys >= 2);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_random_ts() {
        assert!(!detect(b"export const x = 1;\nconsole.log(x);\n"));
        assert!(Vitest::parse(b"a = b\n").is_none());
    }
}
