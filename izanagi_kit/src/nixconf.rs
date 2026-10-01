//! nix.conf / `nix.conf.d/*.conf` census.
//!
//! `key = value` flat settings (space-separated values):
//!
//! The parser counts `key = value` settings, recognised key names, and
//! whitespace-separated list values.
//!
//! ```rust
//! let n = concat!(
//!     "experimental-features = nix-command flakes\n",
//!     "substituters = https://cache.nixos.org/ https://nix-community.cachix.org\n",
//!     "trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=\n",
//!     "max-jobs = auto\n",
//! );
//! let c = izanagi_kit::nixconf::Nixconf::parse(n.as_bytes()).unwrap();
//! assert_eq!(c.settings, 4);
//! ```

/// nix.conf census.
#[derive(Debug, Clone)]
pub struct Nixconf {
    /// `key = value` settings.
    pub settings: usize,
    /// `include`/`requiredSystemFeatures`/`substituters`/`trusted-public-keys`/`extra-trusted-*`/`extra-substituters`/`experimental-features`/`access-tokens`/`secret-key-files`/`builders`/`netrc-file`/`trusted-users`/`allowed-users`/`trusted-substituters`/`connect-timeout`/`stalled-download-timeout`/`narinfo-cache-*`/`hashed-mirrors`/`max-jobs`/`cores`/`auto-optimise-store`/`keep-*`/`gc-*`/`build-*`/`eval-*`/`http-*`/`log-*`/`sandbox*`/`pure-*`/`impure-*`/`allow-*`/`warn-*`/`flake-*`/`tarball-ttl`/`accept-flake-config`/`commit-lockfile-summary`/`eval-cache`/`filter-syscalls`/`restrict-eval`/`use-xdg-base-directories`/`plugin-files`/`ssl-cert-file`/`system`/`system-features`/`extra-platforms`/`require-sigs`/`nar-buffer-size`/`offline`/`fallback`/`repeat`/`enforce-determinism`/`check-sigs`/`use-sqlite-wal`/`sync-before-registering`/`upgrade-nix`/`post-build-hook`/`pre-build-hook`/`build-poll-interval`/`free-*`/`min-free`/`max-free`/`gc-reserved`/`ignored-*`/`keep-build-log`/`keep-env-derivations`/`keep-failed`/`keep-outputs`/`keep-derivations`/`log-compression`/`log-format`/`log-lines`/`fsync-metadata`/`download-attempts`/`download-connections`/`http-connections`/`http2`/`proxy`/`netrc-file`/`ping`/`debugger-*`/`show-trace`/`use-case-hack`/`filter-paths`/`print-*`/`verbose`/`quiet`/`bash-prompt*`/`error-*`/`warn-dirty`/`debugger-on-warn`/`debug-*`/`tty`/`output*`/`timeout`/`max-silent-time`/`store`/`init*`/`regenerate`/`install-source-files`/`pure-eval`/`eval`/`include`-named keys.
    pub named: usize,
    /// Lines whose value is whitespace-separated tokens (>=2 tokens) — lists like `substituters = a b c`.
    pub lists: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Whether the buffer looks like nix.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("substituters")
        || t.contains("trusted-public-keys")
        || t.contains("experimental-features")
        || t.contains("max-jobs")
        || t.contains("sandbox") && t.contains("nixos.org")
}

impl Nixconf {
    /// Parse a nix.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            named: 0,
            lists: 0,
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
            let Some(eq) = s.find('=') else {
                continue;
            };
            c.settings += 1;
            let key = s[..eq].trim();
            let val = s[eq + 1..].trim();
            if val.split_whitespace().count() >= 2 {
                c.lists += 1;
            }
            if [
                "include",
                "requiredSystemFeatures",
                "substituters",
                "trusted-public-keys",
                "experimental-features",
                "access-tokens",
                "secret-key-files",
                "builders",
                "netrc-file",
                "trusted-users",
                "allowed-users",
                "trusted-substituters",
                "connect-timeout",
                "stalled-download-timeout",
                "hashed-mirrors",
                "max-jobs",
                "cores",
                "auto-optimise-store",
                "require-sigs",
                "nar-buffer-size",
                "offline",
                "fallback",
                "repeat",
                "enforce-determinism",
                "check-sigs",
                "use-sqlite-wal",
                "sync-before-registering",
                "upgrade-nix",
                "post-build-hook",
                "pre-build-hook",
                "build-poll-interval",
                "min-free",
                "max-free",
                "gc-reserved",
                "keep-build-log",
                "keep-env-derivations",
                "keep-failed",
                "keep-outputs",
                "keep-derivations",
                "log-compression",
                "log-format",
                "log-lines",
                "fsync-metadata",
                "download-attempts",
                "download-connections",
                "http-connections",
                "http2",
                "proxy",
                "ping",
                "show-trace",
                "use-case-hack",
                "filter-paths",
                "verbose",
                "quiet",
                "timeout",
                "max-silent-time",
                "store",
                "system",
                "system-features",
                "extra-platforms",
                "flake-registry",
                "accept-flake-config",
                "commit-lockfile-summary",
                "eval-cache",
                "filter-syscalls",
                "restrict-eval",
                "use-xdg-base-directories",
                "plugin-files",
                "ssl-cert-file",
                "warn-dirty",
                "debugger-on-warn",
                "tty",
                "pure-eval",
                "eval",
                "tarball-ttl",
                "warn-*",
                "narinfo-*",
            ]
            .contains(&key)
                || key.starts_with("extra-")
                || key.starts_with("keep-")
                || key.starts_with("gc-")
                || key.starts_with("build-")
                || key.starts_with("eval-")
                || key.starts_with("http-")
                || key.starts_with("log-")
                || key.starts_with("sandbox")
                || key.starts_with("pure-")
                || key.starts_with("impure-")
                || key.starts_with("allow-")
                || key.starts_with("warn-")
                || key.starts_with("flake-")
                || key.starts_with("narinfo-cache-")
                || key.starts_with("debugger-")
                || key.starts_with("print-")
                || key.starts_with("bash-prompt")
                || key.starts_with("error-")
                || key.starts_with("debug-")
                || key.starts_with("ignored-")
                || key.starts_with("free-")
                || key.starts_with("trusted-")
                || key.starts_with("required-")
            {
                c.named += 1;
            }
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
            "# nix.conf\n",
            "experimental-features = nix-command flakes\n",
            "substituters = https://cache.nixos.org/ https://nix-community.cachix.org https://numtide.cachix.org\n",
            "trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs=\n",
            "extra-substituters = https://devenv.cachix.org\n",
            "trusted-users = root ume\n",
            "max-jobs = auto\n",
            "cores = 8\n",
            "auto-optimise-store = true\n",
            "keep-outputs = true\n",
            "keep-derivations = true\n",
            "builders-use-substitutes = true\n",
            "connect-timeout = 5\n",
            "narinfo-cache-negative-ttl = 0\n",
            "include = /etc/nix/extra.conf\n",
        );
        let c = Nixconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 14);
        assert_eq!(c.comments, 1);
        assert!(c.named >= 12);
        assert!(c.lists >= 4);
    }

    #[test]
    fn rejects_other() {
        assert!(Nixconf::parse(b"foo = 1").is_none());
    }
}
