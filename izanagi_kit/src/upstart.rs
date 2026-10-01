//! Census of an Upstart job `.conf` file (`/etc/init/*.conf`).
//!
//! Stanza lines: `start on …`/`stop on …` events, `respawn`,
//! `expect daemon`/`expect fork`/`expect stop`, `pre-start script`/
//! `post-start script`/`pre-stop script`/`post-stop script`/`script`/
//! `end script` blocks, `exec …` direct launches, `env`/`setuid`/
//! `setgid`/`console`/`kill signal`/`reload signal`/`limit`/`oom`/
//! `chroot`/`chdir`/`umask`/`nice`/`manual`/`task` settings.
//! Counts stanzas by kind plus script blocks and comments.
//!
//! ```rust
//! let c = izanagi_kit::upstart::Upstart::parse(
//!     b"# job\ndescription \"x\"\nstart on runlevel [2345]\nstop on runlevel [!2345]\n\
//!       respawn\nexec /usr/bin/daemon -D\n",
//! ).unwrap();
//! assert_eq!(c.events, 2);
//! ```
#![forbid(unsafe_code)]

/// Upstart job census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Upstart {
    /// `start on`/`stop on` event stanzas.
    pub events: usize,
    /// `script`/`pre-start`/`post-start`/`pre-stop`/`post-stop` block heads.
    pub script_blocks: usize,
    /// `exec` stanzas.
    pub execs: usize,
    /// `env`/`setuid`/`setgid`/`console`/`kill`/`reload`/`limit`/`oom`/`chroot`/`chdir`/`umask`/`nice`/`manual`/`task` stanzas.
    pub settings: usize,
    /// `respawn`/`expect`/`instance`/`description`/`author`/`emits` stanzas.
    pub meta: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// `on`-event heads.
const EVENT_HEADS: &[&str] = &["start on", "stop on"];

/// Script block heads.
const SCRIPT_HEADS: &[&str] = &[
    "script",
    "pre-start script",
    "post-start script",
    "pre-stop script",
    "post-stop script",
];

/// Setting stanzas.
const SETTING_HEADS: &[&str] = &[
    "env ",
    "setuid ",
    "setgid ",
    "console ",
    "kill signal",
    "reload signal",
    "limit ",
    "oom ",
    "chroot ",
    "chdir ",
    "umask ",
    "nice ",
    "manual",
    "task",
];

/// Meta stanzas.
const META_HEADS: &[&str] = &[
    "respawn",
    "expect",
    "instance",
    "description",
    "author",
    "emits",
    "version",
];

/// True if `b` looks like an Upstart job conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("start on ") && (t.contains("stop on ") || t.contains("exec ")))
        || t.contains("end script")
}

impl Upstart {
    /// Parse an Upstart job conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            events: 0,
            script_blocks: 0,
            execs: 0,
            settings: 0,
            meta: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
            } else if EVENT_HEADS.iter().any(|h| l.starts_with(h)) {
                c.events += 1;
            } else if SCRIPT_HEADS.contains(&l) {
                c.script_blocks += 1;
            } else if l.starts_with("exec ") {
                c.execs += 1;
            } else if SETTING_HEADS
                .iter()
                .any(|h| l.starts_with(h) || l == h.trim_end())
            {
                c.settings += 1;
            } else if META_HEADS.iter().any(|h| l.starts_with(h)) {
                c.meta += 1;
            }
        }
        if c.events == 0 && c.execs == 0 && c.script_blocks == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# demo job\n",
            "description \"demo\"\n",
            "start on runlevel [2345]\n",
            "stop on runlevel [!2345]\n",
            "respawn\n",
            "expect daemon\n",
            "env FOO=bar\n",
            "exec /usr/bin/daemon -D\n",
            "post-start script\n",
            "  echo hi\n",
            "end script\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Upstart::parse(b.as_bytes()).unwrap();
        assert_eq!(c.events, 2);
        assert_eq!(c.script_blocks, 1);
        assert_eq!(c.execs, 1);
        assert_eq!(c.settings, 1);
        assert_eq!(c.meta, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[section]\nkey=1\n"));
        assert!(Upstart::parse(b"# only\n").is_none());
    }
}
