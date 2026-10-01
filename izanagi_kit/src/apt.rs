//! APT sources list (`sources.list`/`*.sources`) census.
//!
//! One-line format: `deb [ opt=v opt2=v ] uri suite [comp …]`,
//! `deb-src`, `deb [arch=amd64 signed-by=…]`, `deb [trusted=yes]` —
//! plus deb822 `.sources` stanzas (`Types:`, `URIs:`, `Suites:`,
//! `Components:`, `Architectures:`, `Signed-By:`, `Enabled:`,
//! `X-Repolib-*:`, `Check-Date`, `Date-Max-Validity`, `NotAutomatic`,
//! `But-Automatic-Upgrades`, `Check-Valid-Until`, `Valid-Until`,
//! `Snapshots`, `PDiffs`, `InRelease-Path`, `Filename`, `Description`,
//! `Source`, `Key`, `Link`, `Standards-Version`), `apt.conf`-style
//! `Acquire::…`, `APT::…`, `Dir::…`, `DPkg::…`, `Unattended-Upgrade::…`
//! directives and `machine`/`login`/`password` auth.conf.
//!
//! ```rust
//! let a = concat!(
//!     "deb http://deb.debian.org/debian bookworm main non-free\n",
//!     "deb-src http://deb.debian.org/debian bookworm main\n",
//!     "deb [arch=amd64 signed-by=/key.gpg] https://dl.google.com/linux/chrome/deb stable main\n",
//! );
//! let c = izanagi_kit::apt::Apt::parse(a.as_bytes()).unwrap();
//! assert_eq!(c.sources, 3);
//! ```

/// APT source list census.
#[derive(Debug, Clone)]
pub struct Apt {
    /// `deb`/`deb-src` one-line entries.
    pub sources: usize,
    /// `[key=value …]` option bracket tokens across deb/deb-src lines.
    pub options: usize,
    /// `deb822` stanza keys (`Types`/`URIs`/`Suites`/`Components`/`Architectures`/`Signed-By`/`Enabled`/`X-Repolib-*`/`Check-*`/`Valid-Until*`/`Snapshots`/`PDiffs`/`InRelease-Path`/`Filename`/`Description`/`Source`/`Key`/`Link`/`Standards-Version`/`Date-Max-Validity`/`NotAutomatic`/`But-Automatic-Upgrades`/`Allow-Insecure`/`Allow-Weak`/`Allow-Downgrade-To-Insecure`/`Trusted`/`Changelogs`/`Test-`/`As-`/`By-Hash`/`IndexTargets`/`Repo-*`).
    pub stanzas: usize,
    /// `Acquire::`/`APT::`/`Dir::`/`DPkg::`/`Unattended-Upgrade::`/`RPM::`/`Binary-*::`/`Debug::`/`apt-key::`/`Machine`/`login`/`password`/`netrc`/`Blind*`/`Key*` apt.conf/auth keys.
    pub confs: usize,
}

/// Whether the buffer looks like an APT sources/auth file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| {
        let s = l.trim();
        s.starts_with("deb ") || s.starts_with("deb-src ")
    }) || (t.contains("Types:") && t.contains("URIs:") && t.contains("Suites:"))
        || t.contains("Acquire::")
        || t.contains("APT::")
}

impl Apt {
    /// Parse an APT config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sources: 0,
            options: 0,
            stanzas: 0,
            confs: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s.starts_with("deb ") || s.starts_with("deb-src ") {
                c.sources += 1;
                c.options += s.matches('[').count();
                continue;
            }
            if s.contains("::") {
                c.confs += 1;
                continue;
            }
            if s.contains(':') {
                let key = s.split(':').next().unwrap_or("").trim();
                if [
                    "Types",
                    "URIs",
                    "Suites",
                    "Components",
                    "Architectures",
                    "Signed-By",
                    "Enabled",
                    "Check-Date",
                    "Date-Max-Validity",
                    "NotAutomatic",
                    "But-Automatic-Upgrades",
                    "Check-Valid-Until",
                    "Valid-Until",
                    "Valid-Until-Min",
                    "Valid-Until-Max",
                    "Snapshots",
                    "PDiffs",
                    "InRelease-Path",
                    "Filename",
                    "Description",
                    "Source",
                    "Key",
                    "Link",
                    "Standards-Version",
                    "Allow-Insecure",
                    "Allow-Weak",
                    "Allow-Downgrade-To-Insecure",
                    "Trusted",
                    "Changelogs",
                    "IndexTargets",
                    "By-Hash",
                    "Repo-Name",
                ]
                .contains(&key)
                    || key.starts_with("X-")
                    || key.starts_with("Test-")
                    || key.starts_with("As-")
                {
                    c.stanzas += 1;
                    continue;
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
    fn parses_oneline() {
        let b = concat!(
            "# debian\n",
            "deb http://deb.debian.org/debian bookworm main non-free contrib\n",
            "deb-src http://deb.debian.org/debian bookworm main\n",
            "deb [arch=amd64 signed-by=/usr/share/keyrings/chrome.gpg] https://dl.google.com/linux/chrome/deb stable main\n",
            "deb [trusted=yes] file:/srv/repo ./\n",
        );
        let c = Apt::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sources, 4);
        assert_eq!(c.options, 2);
    }

    #[test]
    fn parses_deb822() {
        let b = concat!(
            "Types: deb deb-src\n",
            "URIs: http://deb.debian.org/debian\n",
            "Suites: bookworm bookworm-updates\n",
            "Components: main contrib\n",
            "Signed-By: /usr/share/keyrings/debian.gpg\n",
            "Enabled: yes\n",
        );
        let c = Apt::parse(b.as_bytes()).unwrap();
        assert_eq!(c.stanzas, 6);
    }

    #[test]
    fn rejects_other() {
        assert!(Apt::parse(b"foo = 1").is_none());
    }
}
