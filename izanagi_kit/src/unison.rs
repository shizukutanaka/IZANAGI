//! Census of a Unison `.prf` profile file.
//!
//! `key = value` lines: `root` (local + remote roots), `path`,
//! `ignore = Name/Path/BelowPath`, `ignorenot`, `ignorecase`,
//! `follow`, `include other.prf`, `batch`, `auto`, `silent`,
//! `prefer`/`preferpartial`, `copyonconflict`, `fastcheck`,
//! `confirmbigdel`, `times`, `perms`, `dontchmod`, `fat`,
//! `group`/`owner`, `rsync`, `sshargs`, `servercmd`, `copyprog`,
//! `sortnewfirst`, `logfile`, `maxthreads`, `numericids`,
//! `backup`/`backupdir`/`backuplocation`/`maxbackups`,
//! `addversionno`/`versioncontrol`/`merge`/`diff`.
//! `#` comments. Counts roots, paths, ignores, settings.
//!
//! ```rust
//! let c = izanagi_kit::unison::Unison::parse(
//!     b"root = /home/user\nroot = ssh://host//home/user\n\
//!       path = Documents\nignore = Name *.tmp\nbatch = true\n",
//! ).unwrap();
//! assert_eq!(c.roots, 2);
//! assert_eq!(c.paths, 1);
//! assert_eq!(c.ignores, 1);
//! ```
#![forbid(unsafe_code)]

/// unison .prf census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unison {
    /// `root =` lines.
    pub roots: usize,
    /// `path =`/`include =` lines.
    pub paths: usize,
    /// `ignore`/`ignorenot`/`ignorecase`/`follow`/`immutable`/`immutablenot` lines.
    pub ignores: usize,
    /// `prefer`/`preferpartial`/`merge`/`diff`/`backup*`/`versioncontrol`/`copy*`/`servercmd`/`sshargs`/`rsync` sync-behavior lines.
    pub behaviors: usize,
    /// Other `key = value` settings.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Ignore-family keys.
const IGNORES: &[&str] = &[
    "ignore",
    "ignorenot",
    "ignorecase",
    "follow",
    "immutable",
    "immutablenot",
];

/// Behavior-family keys.
const BEHAVIORS: &[&str] = &[
    "prefer",
    "preferpartial",
    "merge",
    "merge2",
    "diff",
    "backup",
    "backupdir",
    "backuplocation",
    "backupprefix",
    "backupsuffix",
    "maxbackups",
    "versioncontrol",
    "addversionno",
    "copyprog",
    "copyprogrest",
    "copyquoterem",
    "copythreshold",
    "servercmd",
    "sshargs",
    "rshargs",
    "rsync",
    "xferbycopying",
    "forcepartial",
];

/// True if `b` looks like a unison profile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("root =") || t.contains("root="))
        && (t.contains("path =") || t.contains("ignore") || t.contains("batch"))
}

impl Unison {
    /// Parse a .prf file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            roots: 0,
            paths: 0,
            ignores: 0,
            behaviors: 0,
            settings: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let Some(eq) = l.find('=') else {
                continue;
            };
            let key = l[..eq].trim();
            match key {
                "root" => c.roots += 1,
                "path" | "include" => c.paths += 1,
                k if IGNORES.contains(&k) => c.ignores += 1,
                k if BEHAVIORS.contains(&k) => c.behaviors += 1,
                "" => {}
                _ => c.settings += 1,
            }
        }
        if c.roots == 0 {
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
            "# profile\n",
            "root = /home/user\n",
            "root = ssh://host//home/user\n",
            "path = Documents\n",
            "path = Pictures\n",
            "ignore = Name *.tmp\n",
            "ignore = Path {Downloads}\n",
            "follow = Name *.lnk\n",
            "prefer = newer\n",
            "batch = true\n",
            "auto = true\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Unison::parse(b.as_bytes()).unwrap();
        assert_eq!(c.roots, 2);
        assert_eq!(c.paths, 2);
        assert_eq!(c.ignores, 3);
        assert_eq!(c.behaviors, 1);
        assert_eq!(c.settings, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(Unison::parse(b"# none\n").is_none());
    }
}
