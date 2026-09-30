//! pacman.conf (INI) census.
//!
//! A pacman.conf is INI: `[options]` globals (`RootDir`/`DBPath`/`CacheDir*`/
//! `HookDir*`/`GPGDir`/`LogFile`/`LogLevel*`/`LogAction`/`DownloadUser`/
//! `XferCommand`/`UseSyslog`/`Color`/`NoProgressBar`/`VerbosePkgLists`/
//! `ParallelDownloads`/`CheckSpace`/`TotalDownload`/`DisableDownloadTimeout`/
//! `DisableSandbox`/`IgnorePkg`/`IgnoreGroup`/`NoUpgrade`/`NoExtract`/
//! `CleanMethod`/`SigLevel`/`LocalFileSigLevel`/`RemoteFileSigLevel`/
//! `HoldPkg`/`Include`/`Architecture`/`UseDelta`/`ReposDir`/`MaxConcurrency`/
//! `LogFile`), repo sections `[core]`/`[extra]`/`[multilib]`/`[community]`/
//! `[testing]`/`[kde-unstable]`/`[custom]`/`[name]` with `Server`/`Include`/
//! `SigLevel`/`Usage`/`CacheServer`/`Priority`/`MaxConcurrency`/`Key*`,
//! comments `#`/`;`, and `HoldPkg`/`NoExtract`/`IgnorePkg`/`IgnoreGroup`
//! multi-values.
//!
//! ```rust
//! let p = concat!(
//!     "[options]\n",
//!     "HoldPkg = pacman glibc\n",
//!     "Architecture = auto\n",
//!     "SigLevel = Required DatabaseOptional\n",
//!     "Color\n",
//!     "[core]\n",
//!     "Include = /etc/pacman.d/mirrorlist\n",
//! );
//! let c = izanagi_kit::pacman::Pacman::parse(p.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// pacman.conf census.
#[derive(Debug, Clone)]
pub struct Pacman {
    /// `[options]`/repo `[name]` section headers.
    pub sections: usize,
    /// `key = value` settings inside `[options]` (with value).
    pub options: usize,
    /// `key = value` settings inside repo sections (`Server`/`Include`/`SigLevel`/`Usage`/`CacheServer`/`Priority`/`MaxConcurrency`/`Key*`/any other key).
    pub servers: usize,
    /// Valueless flag keys (`Color`/`VerbosePkgLists`/`TotalDownload`/`CheckSpace`/`UseSyslog`/`NoProgressBar`/`DisableDownloadTimeout`/`DisableSandbox`/`ParallelDownloads`/`NoWait`/`ILoveCandy`/`ShowSize`/`Quiet`/`LogAction`).
    pub flags: usize,
}

/// Whether the buffer looks like pacman.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("[options]")
        && (t.contains("SigLevel")
            || t.contains("HoldPkg")
            || t.contains("CacheDir")
            || t.contains("IgnorePkg")
            || t.contains("Architecture")))
        || (t.contains("Server =") && t.contains("mirrorlist"))
}

impl Pacman {
    /// Parse a pacman.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            options: 0,
            servers: 0,
            flags: 0,
        };
        let mut in_options = false;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                in_options = &s[1..s.len() - 1] == "options";
                continue;
            }
            if s.contains('=') {
                if in_options {
                    c.options += 1;
                } else {
                    c.servers += 1;
                }
                continue;
            }
            c.flags += 1;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_conf() {
        let b = concat!(
            "[options]\n",
            "RootDir = /\n",
            "DBPath = /var/lib/pacman/\n",
            "CacheDir = /var/cache/pacman/pkg/\n",
            "LogFile = /var/log/pacman.log\n",
            "GPGDir = /etc/pacman.d/gnupg/\n",
            "HookDir = /etc/pacman.d/hooks/\n",
            "HoldPkg = pacman glibc\n",
            "XferCommand = /usr/bin/curl -L -C - -o %o %u\n",
            "CleanMethod = KeepInstalled\n",
            "UseDelta = 0.7\n",
            "Architecture = auto\n",
            "IgnorePkg = linux\n",
            "NoUpgrade = /etc/pacman.d/mirrorlist\n",
            "NoExtract = usr/lib/locale/*\n",
            "SigLevel = Required DatabaseOptional\n",
            "LocalFileSigLevel = Optional\n",
            "RemoteFileSigLevel = Required\n",
            "Color\n",
            "VerbosePkgLists\n",
            "ParallelDownloads = 5\n",
            "CheckSpace\n",
            "[core]\n",
            "Include = /etc/pacman.d/mirrorlist\n",
            "[extra]\n",
            "Server = https://mirror.example/$repo/os/$arch\n",
            "SigLevel = PackageRequired\n",
            "[custom]\n",
            "Server = file:///srv/repo\n",
            "SigLevel = Never\n",
        );
        let c = Pacman::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.flags, 3);
        assert_eq!(c.servers, 5);
        assert!(c.options >= 17);
    }

    #[test]
    fn rejects_other() {
        assert!(Pacman::parse(b"foo = 1").is_none());
    }
}
