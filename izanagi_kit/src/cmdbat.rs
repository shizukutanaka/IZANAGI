//! Windows `.bat`/`.cmd` batch-file census.
//!
//! Batch scripts: `REM`/`::` comments, `@echo off`, `set VAR=`,
//! `set /p`/`set /a`, `if exist`/`if defined`/`if errorlevel`/`if %1==`,
//! `for %%i in (…) do`, `goto label`, `:label` targets,
//! `call`, `start`, `exit /b`, `echo`, `echo.` blank lines,
//! `%~dp0`/`%*`/`%ERRORLEVEL%` expansion, `&&`/`||`/`>`,`>>`,`<`,
//! `2>&1` redirection, `|` pipes, `command /c`/`command /k`.
//!
//! ```rust
//! let c = izanagi_kit::cmdbat::Cmdbat::parse(b"@echo off\nset X=1\nif %X%==1 echo yes\necho done\n").unwrap();
//! assert_eq!(c.commands, 4);
//! ```

use crate::textutil::strip_bom;
/// `.bat`/`.cmd` census.
#[derive(Debug, Clone)]
pub struct Cmdbat {
    /// Batch commands (non-label, non-comment lines).
    pub commands: usize,
    /// `set`/`set /p`/`set /a` assignments.
    pub sets: usize,
    /// `if`/`for`/`goto`/`call`/`exit` control flow.
    pub controls: usize,
    /// `:label` jump targets.
    pub labels: usize,
    /// `REM`/`::` comments.
    pub comments: usize,
}

const CMDS: &[&str] = &[
    "echo",
    "set",
    "if",
    "for",
    "goto",
    "call",
    "start",
    "exit",
    "rem",
    "pause",
    "cls",
    "cd",
    "chdir",
    "dir",
    "copy",
    "xcopy",
    "robocopy",
    "move",
    "ren",
    "rename",
    "del",
    "erase",
    "md",
    "mkdir",
    "rd",
    "rmdir",
    "type",
    "findstr",
    "find",
    "sort",
    "more",
    "attrib",
    "taskkill",
    "tasklist",
    "reg",
    "net",
    "sc",
    "bcdedit",
    "powercfg",
    "setx",
    "setlocal",
    "endlocal",
    "shift",
    "pushd",
    "popd",
    "choice",
    "color",
    "title",
    "ver",
    "vol",
    "assoc",
    "ftype",
    "mklink",
    "cipher",
    "compact",
    "expand",
    "certutil",
    "bitsadmin",
    "wmic",
    "powershell",
    "wscript",
    "cscript",
    "mshta",
    "rundll32",
    "mstsc",
    "shutdown",
    "reboot",
    "format",
    "diskpart",
    "sfc",
    "chkdsk",
    "defrag",
    "subst",
    "timeout",
    "ping",
    "ipconfig",
    "netstat",
    "nslookup",
    "arp",
    "route",
    "tracert",
    "pathping",
    "nbtstat",
    "whoami",
    "hostname",
    "systeminfo",
    "eventcreate",
    "schtasks",
    "cacls",
    "icacls",
    "takeown",
    "gpresult",
    "gpupdate",
    "netsh",
    "rasdial",
    "rasphone",
    "msconfig",
    "control",
    "appwiz",
    "hdwwiz",
    "devmgmt",
    "compmgmt",
    "eventvwr",
    "services",
    "secpol",
    "lusrmgr",
    "diskmgmt",
    "perfmon",
    "resmon",
    "taskmgr",
    "cleanmgr",
    "msinfo32",
    "dxdiag",
    "winver",
    "explorer",
    "notepad",
    "write",
    "mspaint",
    "snippingtool",
    "cmd",
];

/// cmd 固有の信号: `set /a`,`%~`,`%X%` 変数,`errorlevel`,`nul` リダイレクト,
/// `if exist`,`call :label`,`::` ラベル行…は他言語では出ない。
fn is_exclusive(line: &str) -> bool {
    let l = line.trim();
    let low = l.to_ascii_lowercase();
    if low.starts_with('@') {
        return true;
    }
    let head = low.split([' ', '\t']).next().unwrap_or("");
    const EXCLUSIVE_HEADS: &[&str] = &[
        "rem",
        "setlocal",
        "endlocal",
        "enabledelayedexpansion",
        "verify",
        "assoc",
        "ftype",
        "attrib",
        "mklink",
        "subst",
        "doskey",
        "vol",
        "ver",
        "title",
        "color",
        "mode",
        "choice",
        "wmic",
        "certutil",
        "bitsadmin",
        "bcdedit",
        "powercfg",
        "sfc",
        "chkdsk",
        "defrag",
        "diskpart",
        "wscript",
        "cscript",
        "mshta",
        "rundll32",
        "mstsc",
        "robocopy",
        "xcopy",
        "findstr",
        "taskkill",
        "tasklist",
        "setx",
        "schtasks",
        "netsh",
        "regsvr32",
        "takeown",
        "icacls",
        "cacls",
        "compact",
        "cipher",
        "expand",
        "tree",
        "goto",
    ];
    EXCLUSIVE_HEADS.contains(&head)
        || low.contains("%~")
        || low.contains("errorlevel")
        || low.contains("enabledelayedexpansion")
        || low.starts_with("if exist ")
        || low.starts_with("if errorlevel")
        || low.starts_with("set /")
        || low.starts_with("call :")
        || low.starts_with("2>")
        || low.contains(">nul")
        || l.matches('%').count() >= 2
        || l.starts_with("::")
}

/// Whether the buffer looks like a batch file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    let mut exclusive = 0usize;
    for l in t.lines() {
        let s = l.trim().to_ascii_lowercase();
        if CMDS.iter().any(|c| {
            s == *c
                || s.starts_with(&format!("{c} "))
                || s.starts_with(&format!("{c}\t"))
                || s.starts_with(&format!("@{c}"))
                || s.starts_with(&format!("{c}."))
        }) {
            hits += 1;
        }
        if is_exclusive(l) {
            exclusive += 1;
        }
    }
    hits >= 2 && exclusive >= 1
}

impl Cmdbat {
    /// Parse a batch file into census counts.
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
            controls: 0,
            labels: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            let low = s.to_ascii_lowercase();
            if low.starts_with("rem ") || low == "rem" || s.starts_with("::") {
                c.comments += 1;
                continue;
            }
            if s.starts_with(':') && s.len() > 1 {
                c.labels += 1;
                continue;
            }
            let head = low
                .split([' ', '\t', '=', '/', '&'])
                .find(|w| !w.is_empty())
                .unwrap_or("");
            let head = head.trim_start_matches('@');
            if CMDS.contains(&head) {
                c.commands += 1;
                if head == "set" {
                    c.sets += 1;
                } else if matches!(head, "if" | "for" | "goto" | "call" | "exit") {
                    c.controls += 1;
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
    fn parses_bat() {
        let b = concat!(
            "@echo off\n",
            "rem setup\n",
            "set LOG=build.log\n",
            "set /a RETRIES=3\n",
            "if exist build.log del build.log\n",
            ":loop\n",
            "call build.cmd\n",
            "for %%f in (*.dll) do echo %%f\n",
            "goto end\n",
            "echo done\n",
            ":end\n",
            "exit /b 0\n",
        );
        let c = Cmdbat::parse(b.as_bytes()).unwrap();
        assert_eq!(c.comments, 1);
        assert_eq!(c.labels, 2);
        assert_eq!(c.sets, 2);
        assert_eq!(c.controls, 5);
        assert_eq!(c.commands, 9);
    }

    #[test]
    fn rejects_other() {
        assert!(Cmdbat::parse(b"hello\nworld\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
