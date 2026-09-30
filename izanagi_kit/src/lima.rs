//! Lima (`lima.yaml`) virtual-machine definition format.
//!
//! lima.yaml carries `images:`/`arch:`/`cpus:`/`memory:`/`disk:`/
//! `mounts:`/`mountType:`/`mountInotify:`/`mountsUnsupported:`/
//! `ssh:`/`containerd:`/`provisioning:`/`provision:`/`probes:`/
//! `portForwards:`/`networks:`/`env:`/`dns:`/`hostResolver:`/
//! `firmware:`/`video:`/`audio:`/`vmType:`/`os:`/`rosetta:`/`plain:`/
//! `guest*:`/`upgradePackages:`/`caCerts:`/`timeZone:`/
//! `user:`/`param:`/`vmOpts:`/`vmType`/`*:`/`hostname:`/
//! `network:`/`additionalDisks:`/`mountType` keys, mostly
//! `key: value` or `key:\n  - item` lists.
//!
//! ```
//! let b = concat!(
//!     "vmType: qemu\n",
//!     "arch: aarch64\n",
//!     "cpus: 2\n",
//!     "memory: 2GiB\n",
//!     "images:\n",
//!     "  - location: https://x\n",
//!     "    arch: aarch64\n",
//!     "mounts:\n",
//!     "  - location: /tmp\n",
//!     "    writable: true\n",
//!     "ssh:\n",
//!     "  localPort: 60022\n",
//!     "containerd:\n",
//!     "  system: false\n",
//!     "  user: false\n"
//! ).as_bytes();
//! assert!(izanagi_kit::lima::detect(b));
//! let c = izanagi_kit::lima::Lima::parse(b).unwrap();
//! assert_eq!(c.top_keys, 8);
//! ```

/// Parsed lima.yaml summary.
#[derive(Debug, Clone)]
pub struct Lima {
    /// Top-level `key:`/`key: value` occurrences.
    pub top_keys: usize,
    /// `- item` list entries.
    pub items: usize,
    /// `images:` entries.
    pub images: usize,
    /// `mounts:` entries.
    pub mounts: usize,
    /// `portForwards:` entries.
    pub port_forwards: usize,
    /// `provision:`/`probes:` entries.
    pub provisions: usize,
    /// `env:` entries.
    pub env: usize,
    /// `containerd:`/`ssh:`/`hostResolver:`/`firmware:`/`video:`/`audio:`/`networks:`/`caCerts:`/`rosetta:`/`mounts:`/`upgradePackages:`/`cpuType:`/`disk:`/`param:`/`vmOpts:`/`vmType:`/`os:`/`plain:`/`guest*:`/`timeZone:`/`user:`/`hostname:`/`additionalDisks:`/`mountInotify:`/`mountsUnsupported:`/`mountType:`/`mounts:`/`networks:`/`portForwards:`/`provision:`/`probes:`/`ssh:`/`containerd:`/`dns:`/`env:`/`firmware:`/`hostResolver:`/`video:`/`audio:`/`virtualize:`/`upgradePackages:`/`upgradePackage:`/`cpuType:`/`memory:`/`disk:`/`arch:`/`cpus:`/`images:`/`rosetta:`/`plain:`/`guest*:`/`timeZone:`/`user:`/`hostname:`/`additionalDisks:`/`param:`/`vmOpts:`/`limactl`/`lima` top-level keys.
    pub known: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const TOP_KW: &[&str] = &[
    "vmType",
    "os",
    "arch",
    "cpus",
    "memory",
    "disk",
    "hostname",
    "images",
    "mounts",
    "mountType",
    "mountInotify",
    "mountsUnsupported",
    "ssh",
    "containerd",
    "provision",
    "provisioning",
    "probes",
    "portForwards",
    "networks",
    "env",
    "dns",
    "hostResolver",
    "firmware",
    "video",
    "audio",
    "rosetta",
    "plain",
    "upgradePackages",
    "caCerts",
    "timeZone",
    "user",
    "param",
    "vmOpts",
    "additionalDisks",
    "guestAgent",
    "guestInstallPrefix",
    "guestHome",
    "guestSudo",
    "guestMustSudo",
    "guestSshUser",
    "guestUser",
    "guestHomeDir",
    "arch",
    "cpuType",
    "nestedVirtualization",
    "virtiofsd",
    "rosetta",
    "snapshot",
    "resolve",
    "resolveDNS",
    "resolveDotLocal",
];

/// Whether the buffer looks like lima.yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut score = 0usize;
    for l in t.lines() {
        let tr = l.trim_end();
        if tr.starts_with(' ') || tr.starts_with('\t') || tr.starts_with('-') {
            continue;
        }
        if let Some((k, _)) = tr.split_once(':') {
            let k = k.trim();
            if TOP_KW.contains(&k) {
                score += 1;
            }
        }
    }
    score >= 3
}

impl Lima {
    /// Parses a lima.yaml summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            top_keys: 0,
            items: 0,
            images: 0,
            mounts: 0,
            port_forwards: 0,
            provisions: 0,
            env: 0,
            known: 0,
            comments: 0,
        };
        let mut section: Option<&str> = None;
        for l in t.lines() {
            let tr = l.trim_end();
            if tr.trim().starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("- ") || tr == "-" {
                c.items += 1;
                match section {
                    Some("images") => c.images += 1,
                    Some("mounts") => c.mounts += 1,
                    Some("portForwards") => c.port_forwards += 1,
                    Some("provision") | Some("provisioning") | Some("probes") => c.provisions += 1,
                    Some("env") => c.env += 1,
                    _ => {}
                }
                continue;
            }
            if !l.starts_with(' ') && !l.starts_with('\t') {
                if let Some((k, _)) = tr.split_once(':') {
                    let k = k.trim();
                    c.top_keys += 1;
                    if TOP_KW.contains(&k) {
                        c.known += 1;
                    }
                    section = match k {
                        "images" => Some("images"),
                        "mounts" => Some("mounts"),
                        "portForwards" => Some("portForwards"),
                        "provision" | "provisioning" | "probes" => Some("provision"),
                        "env" => Some("env"),
                        _ => None,
                    };
                    continue;
                }
            }
            if (l.starts_with(' ') || l.starts_with('\t'))
                && (tr.trim_start().starts_with("- ") || tr.trim() == "-")
            {
                c.items += 1;
                match section {
                    Some("images") => c.images += 1,
                    Some("mounts") => c.mounts += 1,
                    Some("portForwards") => c.port_forwards += 1,
                    Some("provision") | Some("provisioning") | Some("probes") => c.provisions += 1,
                    Some("env") => c.env += 1,
                    _ => {}
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
    fn parses_lima() {
        let b = concat!(
            "vmType: qemu\n",
            "arch: aarch64\n",
            "cpus: 2\n",
            "memory: 2GiB\n",
            "disk: 100GiB\n",
            "images:\n",
            "  - location: https://x\n",
            "    arch: aarch64\n",
            "  - location: https://y\n",
            "    arch: x86_64\n",
            "mounts:\n",
            "  - location: /tmp\n",
            "    writable: true\n",
            "ssh:\n",
            "  localPort: 60022\n",
            "containerd:\n",
            "  system: false\n",
            "  user: false\n",
            "portForwards:\n",
            "  - guestPort: 80\n",
            "    hostPort: 8080\n",
            "env:\n",
            "  FOO: bar\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Lima::parse(b).unwrap();
        assert_eq!(c.top_keys, 11);
        assert_eq!(c.items, 4);
        assert_eq!(c.images, 2);
        assert_eq!(c.mounts, 1);
        assert_eq!(c.port_forwards, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"a: 1\nb: 2\n"));
        assert!(Lima::parse(b"x").is_none());
    }
}
