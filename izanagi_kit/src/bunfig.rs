//! Bun `bunfig.toml` census.
//!
//! bunfig.toml is TOML: `[install]`(`production`/`exact`/`optional`/
//! `dev`/`peer`/`auto`/`frozenLockfile`/`dry`/`concurrentScripts`),
//! `[install.scopes]`/`[install.cache]`/`[install.registry]`,
//! `[run]`/`[test]`(`coverage*`), `[serve]`(`port`/`static`),
//! `[build]`/`[debug]`/`[smol]`/`[telemetry]`/`[console]`/
//! `[addon]`/`[bundle]`/`[deploy]`/`[jwt]`/`[macro]`/`[lockfile]`/
//! `[workers]`/`[grpc]`/`[http]`/`[postgres]`/`[sqlite]`/`[redis]`.
//!
//! ```rust
//! let c = izanagi_kit::bunfig::Bunfig::parse(b"[install]\nexact = true\n[run]\nshell = \"system\"\n").unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// `bunfig.toml` census.
#[derive(Debug, Clone)]
pub struct Bunfig {
    /// `[section]`/`[sub.section]` headers.
    pub sections: usize,
    /// `key = value` settings.
    pub settings: usize,
    /// `true`/`false` settings.
    pub booleans: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const TOPS: &[&str] = &[
    "install",
    "install.scopes",
    "install.cache",
    "install.registry",
    "install.security",
    "run",
    "run.bun",
    "test",
    "test.coverage",
    "serve",
    "serve.static",
    "serve.routes",
    "build",
    "debug",
    "console",
    "smol",
    "telemetry",
    "addon",
    "bundle",
    "deploy",
    "jwt",
    "macro",
    "lockfile",
    "workers",
    "grpc",
    "http",
    "postgres",
    "sqlite",
    "redis",
    "preload",
];

const BUNKEYS: &[&str] = &[
    "exact",
    "production",
    "concurrentScripts",
    "frozenLockfile",
    "dry",
    "minimumReleaseAge",
    "cafile",
    "coverage",
    "preload",
    "bun",
    "shell",
    "broadcastConsoleLogToDebugger",
    "port",
    "telemetry",
    "smol",
    "entrypoints",
    "outdir",
    "define",
    "saveText",
];

/// Whether the buffer looks like a bunfig.toml file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with('[') {
            let inner = s
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim_matches('"');
            if TOPS.contains(&inner) {
                hits += 2;
                continue;
            }
        }
        if let Some((k, _)) = s.split_once('=') {
            if BUNKEYS.contains(&k.trim()) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

impl Bunfig {
    /// Parse a bunfig.toml file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            booleans: 0,
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
            if s.starts_with('[') {
                c.sections += 1;
                continue;
            }
            if let Some((_, v)) = s.split_once('=') {
                c.settings += 1;
                let v = v.trim();
                if v == "true" || v == "false" {
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
    fn parses_bunfig() {
        let b = concat!(
            "# bun\n",
            "[install]\n",
            "exact = true\n",
            "production = false\n",
            "registry = \"https://registry.npmjs.org\"\n",
            "[install.scopes]\n",
            "myorg = { token = \"$npm_token\", url = \"https://npm.myorg.com\" }\n",
            "[run]\n",
            "shell = \"system\"\n",
            "bun = true\n",
            "[test]\n",
            "coverage = true\n",
            "[smol]\n",
            "heap = 16\n",
            "[telemetry]\n",
            "enabled = false\n",
        );
        let c = Bunfig::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.settings, 9);
        assert_eq!(c.booleans, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Bunfig::parse(b"[foo]\nbar = 1").is_none());
    }
}
