//! Composer `composer.json` census (PHP).
//!
//! Composer-only keys: `require`, `require-dev`, `autoload`,
//! `autoload-dev`, `scripts`, `config`, `repositories`,
//! `minimum-stability`, `prefer-stable`, `suggest`, `conflict`,
//! `replace`, `provide`, `support`, `abandoned`, `funding`,
//! `extra`, `archive`, `_comment`, `time`, `dev-master`,
//! `prefer-installation-source`, `process-timeout`,
//! `use-include-path`, `prepend-autoloader`, `target-dir`,
//! `composer-plugin-api`, `plugin-modifies-*`, `non-feature-branches`.
//! Node `package.json` uses `dependencies`/`devDependencies`
//! instead — those are reject signals.
//!
//! ```rust
//! let k = b"{\n \"name\": \"acme/x\",\n \"require\": {\"php\": \">=8.1\"},\n \"require-dev\": {\"phpunit/phpunit\": \"^10\"},\n \"autoload\": {\"psr-4\": {\"Acme\\\\\": \"src/\"}},\n \"minimum-stability\": \"stable\"\n}\n";
//! assert!(izanagi_kit::composer::detect(k));
//! ```

/// composer.json census.
#[derive(Debug, Clone)]
pub struct Composer {
    /// `"key": value` pairs.
    pub pairs: usize,
    /// `{`/`}`/`[`/`]` structure lines.
    pub braces: usize,
    /// recognised composer keys present.
    pub keys: usize,
    /// `//` comment lines (JSONC).
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "require-dev",
    "autoload",
    "autoload-dev",
    "minimum-stability",
    "prefer-stable",
    "abandoned",
    "conflict",
    "replace",
    "provide",
    "suggest",
    "funding",
    "support",
    "archive",
    "repositories",
    "composer-plugin-api",
    "plugin-modifies-install-path",
    "plugin-modifies-download-path",
    "plugin-modifies-file",
    "non-feature-branches",
    "dev-master",
    "prefer-installation-source",
    "process-timeout",
    "use-include-path",
    "prepend-autoloader",
    "target-dir",
    "optimize-autoloader",
    "classmap-authoritative",
    "apcu-autoloader",
    "check-platform-reqs",
    "allow-plugins",
    "bump-after-update",
    "cafile",
    "capath",
    "github-domains",
    "gitlab-domains",
    "gitlab-token",
    "github-oauth",
    "gitlab-protocol",
    "http-basic",
    "bearer",
    "notify-on-install",
    "vendor-dir",
    "bin-dir",
    "data-dir",
    "cache-dir",
    "cache-files-dir",
    "cache-repo-dir",
    "cache-vcs-dir",
    "cache-files-ttl",
    "cache-files-maxsize",
    "cache-read-only",
    "disable-tls",
    "secure-http",
    "htaccess-protect",
    "lock",
    "platform-check",
    "platform",
    "sort-packages",
    "store-auths",
    "discard-changes",
    "autoloader-suffix",
    "bin-compat",
    "preferred-install",
];

const WEAK: &[&str] = &[
    "require",
    "name",
    "description",
    "license",
    "type",
    "version",
    "authors",
    "homepage",
    "keywords",
    "time",
    "source",
    "dist",
    "scripts",
    "config",
    "extra",
    "_comment",
    "readme",
    "non-dev",
];

const REJECT: &[&str] = &[
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "workspaces",
    "engines",
    "packageManager",
];

fn jkey(line: &str) -> Option<&str> {
    let s = line.trim();
    if !s.starts_with('"') {
        return None;
    }
    let end = s[1..].find('"')? + 1;
    let after = s[end + 1..].trim_start();
    if after.starts_with(':') {
        Some(&s[1..end])
    } else {
        None
    }
}

/// Detect a `composer.json` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut strong = 0usize;
    let mut weak = 0usize;
    let mut reject = 0usize;
    for line in t.lines() {
        if let Some(k) = jkey(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            } else if REJECT.contains(&k) {
                reject += 1;
            }
        }
    }
    reject == 0 && (strong >= 2 || (strong >= 1 && weak >= 2))
}

impl Composer {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            pairs: 0,
            braces: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if jkey(line).is_some() {
                c.pairs += 1;
            }
            if s.starts_with('{') || s.starts_with('}') || s.starts_with('[') || s.starts_with(']')
            {
                c.braces += 1;
            }
            if let Some(k) = jkey(line) {
                if STRONG.contains(&k) || WEAK.contains(&k) {
                    c.keys += 1;
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
    fn detects() {
        let b = b"{\n \"name\": \"acme/x\",\n \"require\": {\"php\": \">=8.1\"},\n \"require-dev\": {\"phpunit/phpunit\": \"^10\"},\n \"autoload\": {\"psr-4\": {\"Acme\\\\\": \"src/\"}},\n \"minimum-stability\": \"stable\"\n}\n";
        assert!(detect(b));
        let c = Composer::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(
            b"{\n \"name\": \"x\",\n \"dependencies\": {},\n \"devDependencies\": {}\n}\n"
        ));
        assert!(!detect(b"{\n \"name\": \"x\",\n \"version\": \"1\"\n}\n"));
    }
}
