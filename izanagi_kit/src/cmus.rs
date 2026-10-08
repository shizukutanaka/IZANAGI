//! cmus `autosave`/`rc`/`command-mode` file census.
//!
//! cmus persists and accepts commands such as
//! `set continue=true`, `set softvol=true`, `set dsp.alsa.device=…`,
//! `bind common ^C quit`, `unbind common u`, `colorscheme xterm-white`,
//! `view playlist`, `add -q file.mp3`, `factivate text-black`,
//! `fset`/`filter`, `win-add-*`, `player-next` —
//! `autosave`/`rc` are plain command files (one per line).
//!
//! ```rust
//! let c = izanagi_kit::cmus::Cmus::parse(b"set continue=true\nbind common ^C quit\n").unwrap();
//! assert_eq!(c.commands, 2);
//! ```

use crate::textutil::strip_bom;
/// cmus command-file census.
#[derive(Debug, Clone)]
pub struct Cmus {
    /// Recognized cmus commands (`set`/`bind`/`fset`/`view`/…).
    pub commands: usize,
    /// `set name=value` entries.
    pub sets: usize,
    /// `bind`/`unbind`/`factivate`/`fset` entries.
    pub binds: usize,
    /// `#` comments.
    pub comments: usize,
}

const CMDS: &[&str] = &[
    "set",
    "bind",
    "unbind",
    "fset",
    "factivate",
    "colorscheme",
    "view",
    "add",
    "add-from-stdin",
    "filter",
    "clear",
    "load",
    "save",
    "quit",
    "echo",
    "inverted",
    "toggle",
    "win-*",
    "player-",
    "cd",
    "run",
    "shell",
    "rand",
    "pl-import",
    "pl-export",
    "live-filter",
];

/// Whether the buffer looks like a cmus autosave/rc file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let hits = t
        .lines()
        .filter(|l| {
            let s = l.trim();
            CMDS.iter().any(|c| {
                if c.ends_with('*') || c.ends_with('-') {
                    s.starts_with(c.trim_end_matches('*'))
                } else {
                    s == *c || s.starts_with(&format!("{c} "))
                }
            })
        })
        .count();
    hits >= 2
}

impl Cmus {
    /// Parse a cmus command file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            commands: 0,
            sets: 0,
            binds: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let is_cmd = CMDS.iter().any(|c| {
                if c.ends_with('*') || c.ends_with('-') {
                    s.starts_with(c.trim_end_matches('*'))
                } else {
                    s == *c || s.starts_with(&format!("{c} "))
                }
            });
            if is_cmd {
                c.commands += 1;
                if s.starts_with("set ") {
                    c.sets += 1;
                } else if s.starts_with("bind ")
                    || s.starts_with("unbind ")
                    || s.starts_with("factivate ")
                    || s.starts_with("fset ")
                {
                    c.binds += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_autosave() {
        let b = concat!(
            "set continue=true\n",
            "set repeat=false\n",
            "set softvol=true\n",
            "bind common ^C quit\n",
            "bind common u win-update\n",
            "colorscheme xterm-white\n",
            "view playlist\n",
            "add -q ~/music/track.mp3\n",
            "fset text=red\n",
            "# comment\n",
        );
        let c = Cmus::parse(b.as_bytes()).unwrap();
        assert_eq!(c.commands, 9);
        assert_eq!(c.sets, 3);
        assert_eq!(c.binds, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Cmus::parse(b"ls -la\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
