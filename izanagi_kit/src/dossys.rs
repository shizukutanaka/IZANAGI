//! DOS `CONFIG.SYS`/`AUTOEXEC.BAT`/`MSDOS.SYS` census.
//!
//! CONFIG.SYS: `DEVICE=C:\\DOS\\HIMEM.SYS`, `DEVICEHIGH=`,
//! `FILES=`, `BUFFERS=`, `DOS=HIGH,UMB`, `STACKS=`, `DRIVPARM=`,
//! `INSTALL=`, `SET `, `REM `, `SHELL=`, `COUNTRY=`, `SWITCHES=`,
//! `BREAK=`, `FCBS=`, `LASTDRIVE=`, `DOSDATA=` blocks `[MENU]`.
//! AUTOEXEC.BAT/`MSDOS.SYS`: `PATH`, `SET VAR=`, `PROMPT`,
//! `LOADHIGH`/`LH`, `smartdrv`, `mouse`, `echo` lines.
//!
//! ```rust
//! let c = izanagi_kit::dossys::Dossys::parse(b"DEVICE=C:\\DOS\\HIMEM.SYS\nFILES=40\n").unwrap();
//! assert_eq!(c.directives, 2);
//! ```

/// DOS system-file census.
#[derive(Debug, Clone)]
pub struct Dossys {
    /// `KEY=value`/`KEY value` directives.
    pub directives: usize,
    /// `REM`/`::` comments.
    pub comments: usize,
    /// `SET` assignments / `PATH`/`PROMPT` lines.
    pub envs: usize,
    /// Batch commands (`CALL`/`IF`/`GOTO`/`LOADHIGH`/`LH`/…).
    pub commands: usize,
}

const DIRS: &[&str] = &[
    "DEVICE",
    "DEVICEHIGH",
    "FILES",
    "BUFFERS",
    "DOS",
    "STACKS",
    "DRIVPARM",
    "INSTALL",
    "SHELL",
    "COUNTRY",
    "SWITCHES",
    "BREAK",
    "FCBS",
    "LASTDRIVE",
    "DOSDATA",
    "BUFFERSHIGH",
    "FILESHIGH",
    "HIDEBOOTLOGO",
    "COMMON_HI",
    "ACCESSLOCK",
    "NOFILES",
    "NUMLOCK",
];

const CMDS: &[&str] = &[
    "SET", "PATH", "PROMPT", "LOADHIGH", "LH", "CALL", "IF", "GOTO", "ECHO", "@ECHO", "SMARTDRV",
    "MOUSE", "SHARE", "CLS", "PAUSE", "FOR", "IN", "DO", "START", "WIN", "TYPE", "COPY", "XCOPY",
    "CHKDSK", "SCANDISK", "MSCDEX", "SHSUCDX", "LOADFIX", "RUN",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a DOS system file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let hits = DIRS
        .iter()
        .chain(CMDS.iter())
        .filter(|d| {
            t.lines().any(|l| {
                let s = l.trim().to_ascii_uppercase();
                s == **d
                    || s.starts_with(&format!("{d}="))
                    || s.starts_with(&format!("{d} ="))
                    || s.starts_with(&format!("{d} "))
            })
        })
        .count();
    hits >= 2 || t.contains("MSDOS.SYS") || t.contains("himem.sys") || t.contains("HIMEM")
}

impl Dossys {
    /// Parse a DOS system file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            directives: 0,
            comments: 0,
            envs: 0,
            commands: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("REM")
                || s.starts_with("rem")
                || s.starts_with("::")
                || s.starts_with('#')
            {
                c.comments += 1;
                continue;
            }
            let head = s
                .split(['=', ' ', ':'])
                .next()
                .unwrap_or("")
                .to_ascii_uppercase();
            if DIRS.contains(&head.as_str()) {
                c.directives += 1;
            } else if head == "SET" || head == "PATH" || head == "PROMPT" {
                c.envs += 1;
            } else if CMDS.contains(&head.as_str()) {
                c.commands += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_sys() {
        let b = concat!(
            "REM CONFIG.SYS\n",
            "DEVICE=C:\\DOS\\HIMEM.SYS\n",
            "DEVICEHIGH=C:\\DOS\\EMM386.EXE\n",
            "DOS=HIGH,UMB\n",
            "FILES=40\n",
            "BUFFERS=20\n",
            "SHELL=C:\\DOS\\COMMAND.COM /P\n",
            "LASTDRIVE=Z\n",
        );
        let c = Dossys::parse(b.as_bytes()).unwrap();
        assert_eq!(c.directives, 7);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn parses_autoexec() {
        let b = concat!(
            "@ECHO OFF\n",
            "PATH C:\\DOS;C:\\TOOLS\n",
            "SET TEMP=C:\\TEMP\n",
            "LH MOUSE\n",
            "SMARTDRV\n",
            "WIN\n",
        );
        let c = Dossys::parse(b.as_bytes()).unwrap();
        assert_eq!(c.commands, 4);
        assert_eq!(c.envs, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Dossys::parse(b"hello\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
