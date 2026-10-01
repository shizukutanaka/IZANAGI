//! Sidekiq `sidekiq.yml` census.
//!
//! sidekiq.yml uses colon-prefixed YAML keys: `:concurrency:`,
//! `:queues:` (list of `- [name, weight]` pairs), `:timeout:`,
//! `:verbose:`, `:pidfile:`, `:logfile:`, `:require:`,
//! `:environment:`, `:max_retries:`, `:dead_max_jobs:`,
//! `:dead_timeout_in_seconds:`, `:retry:`/`:backtrace:`/
//! `:schedule:` (cron entries with `cron`/`class`/`queue`/
//! `active_job`/`description`), and optional per-environment
//! top-level sections (`production:`/`staging:`/`development:`).
//!
//! ```rust
//! let c = izanagi_kit::sidekiq::Sidekiq::parse(
//!     b":concurrency: 10\n:queues:\n  - [critical, 2]\n",
//! ).unwrap();
//! assert_eq!(c.queues, 1);
//! ```

/// sidekiq.yml census.
#[derive(Debug, Clone)]
pub struct Sidekiq {
    /// `:key:` option lines plus plain keys.
    pub keys: usize,
    /// `- [queue, weight]` / `- queue` list entries under `:queues:`.
    pub queues: usize,
    /// Entries inside `:schedule:`.
    pub schedule_entries: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    ":concurrency:",
    ":queues:",
    ":timeout:",
    ":verbose:",
    ":pidfile:",
    ":logfile:",
    ":require:",
    ":environment:",
    ":max_retries:",
    ":dead_max_jobs:",
    ":dead_timeout_in_seconds:",
    ":retry:",
    ":backtrace:",
    ":schedule:",
    ":staging:",
    ":production:",
    ":development:",
    ":test:",
    ":limits:",
    ":statsd:",
    ":labels:",
    ":lifecycle_events:",
];

/// Whether the buffer looks like sidekiq.yml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains(":concurrency:")
        || t.contains(":queues:")
        || (t.contains(":schedule:") && t.contains("cron"))
        || KEYS.iter().filter(|k| t.contains(*k)).count() >= 3
}

impl Sidekiq {
    /// Parse sidekiq.yml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            queues: 0,
            schedule_entries: 0,
            comments: 0,
        };
        let mut scope = "";
        for l in t.lines() {
            let s = l.trim_end();
            if s.trim().is_empty() {
                continue;
            }
            if s.trim().starts_with('#') {
                c.comments += 1;
                continue;
            }
            let indent = s.len() - s.trim_start().len();
            let body = s.trim_start();
            if indent == 0 {
                scope = body.trim_end_matches(':');
                c.keys += 1;
                continue;
            }
            if body.starts_with('-') {
                if scope == ":queues" || scope == "queues" {
                    c.queues += 1;
                } else {
                    c.schedule_entries += 1;
                }
            } else if body.contains(':') {
                c.keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sidekiq_yml() {
        let b = concat!(
            "# sidekiq.yml\n",
            ":concurrency: 10\n",
            ":timeout: 30\n",
            ":verbose: false\n",
            ":queues:\n",
            "  - [critical, 2]\n",
            "  - [default, 1]\n",
            "  - mailers\n",
            ":schedule:\n",
            "  my_job:\n",
            "    cron: \"0 3 * * *\"\n",
            "    class: CleanupJob\n",
        );
        let c = Sidekiq::parse(b.as_bytes()).unwrap();
        assert_eq!(c.keys, 8);
        assert_eq!(c.queues, 3);
        assert_eq!(c.schedule_entries, 0);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Sidekiq::parse(b"foo: 1\n").is_none());
    }
}
