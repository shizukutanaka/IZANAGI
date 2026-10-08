//! Quartz Scheduler `quartz.properties` census.
//!
//! quartz.properties is `key=value` properties under the
//! `org.quartz.` namespace: `org.quartz.scheduler.*`
//! (`instanceName`/`instanceId`/`rmi.*`/`wrapJobExecutionInUserTransaction`),
//! `org.quartz.threadPool.*` (`class`/`threadCount`/
//! `threadPriority`/`threadsInheritContextClassLoaderOfInitializingThread`),
//! `org.quartz.jobStore.*` (`class`/`misfireThreshold`/`driverDelegateClass`/
//! `useDriverToInitialize`/`tablePrefix`/`dataSource`/`dataSources`/`isClustered`),
//! `org.quartz.plugin.*` (`<name>.class`/`.fileName`/`.jobNames`/`.groupNames`),
//! `org.quartz.dataSource.*` (`driver`/`URL`/`user`/`password`/`maxConnections`),
//! `org.quartz.jobListener.*`/`triggerListener.*`,
//! `org.quartz.scheduler.jmx.*`.
//!
//! ```rust
//! let c = izanagi_kit::quartz::Quartz::parse(
//!     b"org.quartz.scheduler.instanceName: Q\norg.quartz.threadPool.threadCount: 3\norg.quartz.jobStore.class: x\n",
//! ).unwrap();
//! assert_eq!(c.entries, 3);
//! ```

/// quartz.properties census.
#[derive(Debug, Clone)]
pub struct Quartz {
    /// `key=value`/`key: value` entries.
    pub entries: usize,
    /// Entries under `org.quartz.threadPool.`/`jobStore.`/`plugin.`/`dataSource.`/`scheduler.`/`listener`s.
    pub namespaced: usize,
    /// `org.quartz.plugin.*` keys.
    pub plugins: usize,
    /// `#`/`!` comment lines.
    pub comments: usize,
}

const SCOPES: &[&str] = &[
    "org.quartz.scheduler.",
    "org.quartz.threadPool.",
    "org.quartz.jobStore.",
    "org.quartz.dataSource.",
    "org.quartz.plugin.",
    "org.quartz.jobListener.",
    "org.quartz.triggerListener.",
];

fn key_of(s: &str) -> &str {
    s.split(['=', ':']).next().unwrap_or("").trim_end()
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like quartz.properties.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines()
        .filter(|l| {
            let s = l.trim_start();
            !s.is_empty()
                && !s.starts_with('#')
                && !s.starts_with('!')
                && s.contains("org.quartz.")
                && (s.contains('=') || s.contains(':'))
        })
        .count()
        >= 3
}

impl Quartz {
    /// Parse quartz.properties into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            entries: 0,
            namespaced: 0,
            plugins: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with('!') {
                c.comments += 1;
                continue;
            }
            if !(s.contains('=') || s.contains(':')) {
                continue;
            }
            c.entries += 1;
            let k = key_of(s);
            if k.starts_with("org.quartz.plugin.") {
                c.plugins += 1;
            }
            if SCOPES.iter().any(|p| k.starts_with(p)) {
                c.namespaced += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quartz_properties() {
        let b = concat!(
            "# quartz.properties\n",
            "org.quartz.scheduler.instanceName: MySched\n",
            "org.quartz.threadPool.threadCount: 5\n",
            "org.quartz.jobStore.class: org.quartz.simpl.RAMJobStore\n",
            "org.quartz.plugin.triggHistory.class: org.quartz.plugins.history.LoggingTriggerHistoryPlugin\n",
            "org.quartz.plugin.jobInitializer.class: org.quartz.plugins.xml.XMLSchedulingDataProcessorPlugin\n",
            "org.quartz.dataSource.myDS.driver: org.postgresql.Driver\n",
            "threads=3\n",
        );
        let c = Quartz::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 7);
        assert_eq!(c.namespaced, 6);
        assert_eq!(c.plugins, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Quartz::parse(b"org.x.a=1\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
