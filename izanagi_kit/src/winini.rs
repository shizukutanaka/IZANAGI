//! Windows 3.x `win.ini`/`system.ini` census.
//!
//! win.ini/system.ini are INI with `[windows]`/`[Desktop]`/
//! `[fonts]`/`[Extensions]`/`[boot]`/`[drivers]`/`[drivers32]`/
//! `[mci]`/`[Network]`/`[intfiles]`/`[Mail]`/`[intl]`/`[ports]`/
//! `[Devices]`/`[PrinterPorts]`/`[BootDescription]`/`[OLE]`/
//! `[Compatibility]`/`[FontSubstitutes]`/`[ScreenSavers]` sections
//! and `key=value` entries (`load=`/`run=`/`shell=`/`ScreenSave*=`).
//!
//! ```rust
//! let c = izanagi_kit::winini::Winini::parse(b"[windows]\nload=\nrun=\n[Desktop]\nWallpaper=none\n").unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// win.ini/system.ini census.
#[derive(Debug, Clone)]
pub struct Winini {
    /// `[section]` headers.
    pub sections: usize,
    /// `key=value` entries.
    pub entries: usize,
    /// `;` comments.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "windows",
    "Desktop",
    "fonts",
    "Extensions",
    "boot",
    "drivers",
    "drivers32",
    "mci",
    "Network",
    "intfiles",
    "Mail",
    "intl",
    "ports",
    "Devices",
    "PrinterPorts",
    "BootDescription",
    "OLE",
    "Compatibility",
    "FontSubstitutes",
    "ScreenSavers",
    "NonWindowsApp",
    "Sounds",
    "MSDOS",
    "PCL",
    "PostScript",
    "Pscript.Drv",
    "HEPHardware",
    "boot.shell",
    "386Enh",
    "keyboard",
    "NonWindowsApps",
];

/// Whether the buffer looks like a win.ini/system.ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    SECTIONS
        .iter()
        .filter(|s| t.contains(&format!("[{s}]")))
        .count()
        >= 2
        || (t.contains("[windows]") && (t.contains("load=") || t.contains("run=")))
}

impl Winini {
    /// Parse a win.ini/system.ini into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            entries: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with(';') {
                c.comments += 1;
            } else if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
            } else if s.contains('=') {
                c.entries += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_winini() {
        let b = concat!(
            "[windows]\n",
            "load=\n",
            "run=\n",
            "NullPort=None\n",
            "[Desktop]\n",
            "Wallpaper=(None)\n",
            "TileWallpaper=0\n",
            "[boot]\n",
            "shell=ProgMan.exe\n",
            "[extensions]\n",
            "ini=notepad.exe ^.ini\n",
            "txt=notepad.exe ^.txt\n",
            "; comment\n",
        );
        let c = Winini::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.entries, 8);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Winini::parse(b"[foo]\nx=1\n").is_none());
    }
}
