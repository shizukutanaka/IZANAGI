//! `collectd.conf` census.
//!
//! `Key value` global options (`BaseDir`, `Interval`, `Hostname` …),
//! `LoadPlugin name` lines, and XML-ish `<Plugin name>` … `</Plugin>` /
//! `<Include dir>` blocks. `#` comments.
//!
//! ```rust
//! let c = b"Hostname \"db1\"\nInterval 10\nLoadPlugin cpu\nLoadPlugin memory\n<Plugin rrdtool>\nDataDir \"/var/lib/collectd\"\n</Plugin>\n";
//! assert!(izanagi_kit::collectd::detect(c));
//! let s = izanagi_kit::collectd::Collectd::parse(c).unwrap();
//! assert_eq!(s.load_plugins, 2);
//! ```

/// collectd.conf census.
#[derive(Debug, Clone)]
pub struct Collectd {
    /// `Key value` option lines matching a known collectd key.
    pub settings: usize,
    /// `LoadPlugin name` lines.
    pub load_plugins: usize,
    /// `<Block>`/`</Block>` open lines (`<Plugin>`/`<Include>`/`<Chain>`/…).
    pub blocks: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Top-level option keys (first word).
const KEYS: &[&str] = &[
    "AutoLoadPlugin",
    "BaseDir",
    "CollectInternalStats",
    "FQDNLookup",
    "Hostname",
    "Interval",
    "MaxReadInterval",
    "PIDFile",
    "PluginDir",
    "ReadQueueLimitHigh",
    "ReadQueueLimitLow",
    "ReadThreads",
    "Timeout",
    "TypesDB",
    "WriteQueueLimitHigh",
    "WriteQueueLimitLow",
    "WriteThreads",
];

/// Block openers (`<Name` at line start).
const BLOCKS: &[&str] = &[
    "<Aggregation",
    "<Chain",
    "<FilePath",
    "<Filter",
    "<Include",
    "<Match",
    "<Plugin",
    "<PostCacheChain",
    "<PreCacheChain",
    "<Rule",
    "<Target",
];

fn first_word(t: &str) -> &str {
    t.split(char::is_whitespace).next().unwrap_or("")
}

fn is_block(t: &str) -> bool {
    BLOCKS.iter().any(|p| t.starts_with(p))
}

/// Detect a `collectd.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut keys = 0usize;
    let mut plugins = 0usize;
    let mut blocks = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if tr.starts_with("LoadPlugin ") || tr == "LoadPlugin" {
            plugins += 1;
            continue;
        }
        if is_block(tr) || tr.starts_with("</") {
            blocks += 1;
            continue;
        }
        if KEYS.contains(&first_word(tr)) {
            keys += 1;
        }
    }
    plugins >= 2 || (plugins >= 1 && (keys >= 1 || blocks >= 2)) || keys >= 3
}

impl Collectd {
    /// Count options, LoadPlugin lines, and blocks. Returns `None` when
    /// the input does not look like a `collectd.conf`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            load_plugins: 0,
            blocks: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("LoadPlugin ") || tr == "LoadPlugin" {
                c.load_plugins += 1;
                continue;
            }
            if is_block(tr) || tr.starts_with("</") {
                c.blocks += 1;
                continue;
            }
            if KEYS.contains(&first_word(tr)) {
                c.settings += 1;
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
        let b = b"# collectd\nHostname \"db1\"\nFQDNLookup true\nBaseDir \"/var/lib/collectd\"\nPIDFile \"/run/collectd.pid\"\nInterval 10\nTimeout 2\nReadThreads 5\nLoadPlugin syslog\nLoadPlugin cpu\nLoadPlugin memory\nLoadPlugin rrdtool\n<Plugin rrdtool>\n\tDataDir \"/var/lib/collectd/rrd\"\n\tCacheTimeout 120\n</Plugin>\n<Plugin network>\n\tServer \"ff18::efc0:4a42\"\n</Plugin>\n";
        assert!(detect(b));
        let c = Collectd::parse(b).unwrap();
        assert_eq!(c.settings, 7);
        assert_eq!(c.load_plugins, 4);
        assert_eq!(c.blocks, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[main]\ndns=none\n"));
        assert!(!detect(b"foo bar\n"));
        assert!(Collectd::parse(b"").is_none());
    }
}
