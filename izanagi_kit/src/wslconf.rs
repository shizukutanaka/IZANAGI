//! `wsl.conf` / `.wslconfig` INI-style format.
//!
//! wsl.conf sections: `[boot]`, `[network]`, `[automount]`,
//! `[interop]`, `[user]`, `[gpu]`, `[time]`, `[general]`,
//! `[experimental]` with `key = value` booleans/strings
//! (`systemd`, `generateHosts`, `mountFsTab`, `enabled`,
//! `appendWindowsPath`, `default`, `command`, `hostname`,
//! `dnsTunneling`, `firewall`, `autoProxy`, `nestedVirtualization`,
//! `vmIdleTimeout`, `guiApplications`, `debugConsole`,
//! `instanceIdleTimeout`, `sparseVhd`, `memory`, `processors`,
//! `swap`, `localhostForwarding`, `localhostforwarding`,
//! `networkingMode`, `vmSwitch`, `dhcp`, `macAddress`,
//! `ipv6`, `ignoredPorts`, `hostAddressLoopback`,
//! `bestEffortDnsParsing`, `initialAutoProxyTimeout`,
//! `useWindowsDnsCache`, `wsl2AutoMemoryReclaim`,
//! `dnsTunnelingIpAddress`, `kernel`, `kernelCommandLine`,
//! `safeMode`, `virtio9p`, `nestedVirtualization`,
//! `autoMemoryReclaim`, `hostAddressLoopback`, `pageReporting`,
//! `gpu`, `guiApplications`, `swapFile`, `pageReporting`,
//! `idleTimeout`, `vmIdleTimeout`, `processors`, `memory`,
//! `swap`, `localhostForwarding`, `nestedVirtualization`,
//! `vmIdleTimeout`, `useWindowsDnsCache`, `wsl2AutoMemoryReclaim`,
//! `diskSize`, `defaultVhdSize`).
//!
//! ```
//! let b = concat!(
//!     "[boot]\n",
//!     "systemd = true\n",
//!     "[network]\n",
//!     "generateHosts = false\n",
//!     "[user]\n",
//!     "default = devin\n",
//!     "[interop]\n",
//!     "enabled = true\n",
//!     "appendWindowsPath = false\n"
//! ).as_bytes();
//! assert!(izanagi_kit::wslconf::detect(b));
//! let c = izanagi_kit::wslconf::Wslconf::parse(b).unwrap();
//! assert_eq!(c.sections, 4);
//! assert_eq!(c.entries, 5);
//! ```

/// Parsed wsl.conf summary.
#[derive(Debug, Clone)]
pub struct Wslconf {
    /// `[section]` headers.
    pub sections: usize,
    /// `key = value` entries.
    pub entries: usize,
    /// `true`/`false` boolean values.
    pub booleans: usize,
    /// `[boot]`/`[automount]`/`[network]`/`[interop]`/`[user]`/`[gpu]`/`[time]`/`[general]`/`[experimental]` sections.
    pub known: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "boot",
    "automount",
    "network",
    "interop",
    "user",
    "gpu",
    "time",
    "general",
    "experimental",
    "wsl2",
    "vm",
];

/// Whether the buffer looks like wsl.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut seen = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with('[') && tr.ends_with(']') {
            let inner = tr[1..tr.len() - 1].to_ascii_lowercase();
            if SECTIONS.contains(&inner.as_str()) {
                seen += 1;
            }
        }
    }
    seen >= 1
        && t.lines().any(|l| {
            let tr = l.trim();
            tr.contains('=')
                && (tr.ends_with("true") || tr.ends_with("false") || tr.contains("command"))
        })
}

impl Wslconf {
    /// Parses a wsl.conf summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            entries: 0,
            booleans: 0,
            known: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
                let inner = tr[1..tr.len() - 1].to_ascii_lowercase();
                if SECTIONS.contains(&inner.as_str()) {
                    c.known += 1;
                }
                continue;
            }
            if tr.contains('=') {
                c.entries += 1;
                if tr.ends_with("true") || tr.ends_with("false") {
                    c.booleans += 1;
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
    fn parses_wslconf() {
        let b = concat!(
            "[boot]\n",
            "systemd = true\n",
            "[network]\n",
            "generateHosts = false\n",
            "[automount]\n",
            "mountFsTab = false\n",
            "[interop]\n",
            "enabled = true\n",
            "appendWindowsPath = false\n",
            "[user]\n",
            "default = devin\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Wslconf::parse(b).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.entries, 6);
        assert_eq!(c.booleans, 5);
        assert_eq!(c.known, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[other]\nkey = value\n"));
        assert!(Wslconf::parse(b"x").is_none());
    }
}
