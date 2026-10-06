//! Firebase `firebase.json` census.
//!
//! Top-level keys: `hosting` (`public`/`rewrites`/`redirects`/
//! `headers`/`cleanUrls`/`trailingSlash`/`ignore`/`appAssociation`/
//! `i18n`/`frameworksBackend`), `functions` (`source`/`codebase`/
//! `ignore`/`predeploy`/`postdeploy`/`runtime`), `firestore`
//! (`rules`/`indexes`), `database` (`rules`), `storage` (`rules`),
//! `emulators` (`auth`/`functions`/`firestore`/`database`/`hosting`/
//! `pubsub`/`storage`/`ui`/`logging`/`hub`/`singleProjectMode`/
//! `dataconnect`/`eventarc`/`tasks`), `remoteconfig`, `extensions`,
//! `dataconnect`, `crashlytics`, `apphosting`/`backendId`,
//! `flutter`/`platforms`, `react`/`ng` workspace targets, `projects`,
//! `targets`, `etags`. Reject: `dependencies`/`devDependencies`.
//!
//! ```rust
//! let k = b"{\n \"hosting\": {\"public\": \"dist\"},\n \"functions\": {\"source\": \"functions\"},\n \"firestore\": {\"rules\": \"firestore.rules\"},\n \"emulators\": {\"ui\": {\"enabled\": true}}\n}\n";
//! assert!(izanagi_kit::firebase::detect(k));
//! ```

/// firebase.json census.
#[derive(Debug, Clone)]
pub struct Firebase {
    /// `"key": value` pairs.
    pub pairs: usize,
    /// `{`/`}`/`[`/`]` structure lines.
    pub braces: usize,
    /// recognised firebase keys present.
    pub keys: usize,
    /// `//` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "hosting",
    "firestore",
    "emulators",
    "remoteconfig",
    "dataconnect",
    "apphosting",
    "frameworksBackend",
    "crashlytics",
    "rewrites",
    "cleanUrls",
    "trailingSlash",
    "appAssociation",
    "singleProjectMode",
    "pubsub",
    "eventarc",
    "codebase",
    "predeploy",
    "postdeploy",
    "runtime",
    "indexes",
    "backendId",
    "alwaysDeployToDefault",
    "site",
    "flutter",
    "platforms",
    "targets",
    "projects",
    "etags",
    "ui",
    "extensions",
    "i18n",
    "redirects",
    "headers",
];

const WEAK: &[&str] = &[
    "public",
    "ignore",
    "source",
    "rules",
    "database",
    "storage",
    "functions",
    "auth",
    "logging",
    "hub",
    "enabled",
    "port",
    "host",
    "name",
    "root",
    "default",
    "export",
    "import",
    "seed",
    "demo",
];

const REJECT: &[&str] = &[
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "workspaces",
    "packageManager",
    "scripts",
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

/// Detect a `firebase.json` file.
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

impl Firebase {
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
            if s.starts_with('{') || s.starts_with('}') || s.starts_with('[') || s.starts_with(']')
            {
                c.braces += 1;
            }
            if let Some(k) = jkey(line) {
                c.pairs += 1;
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
        let b = b"{\n \"hosting\": {\"public\": \"dist\"},\n \"functions\": {\"source\": \"functions\"},\n \"firestore\": {\"rules\": \"firestore.rules\"},\n \"emulators\": {\"ui\": {\"enabled\": true}}\n}\n";
        assert!(detect(b));
        let c = Firebase::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"{\n \"name\": \"x\",\n \"dependencies\": {}\n}\n"));
        assert!(!detect(b"{\n \"name\": \"x\",\n \"version\": \"1\"\n}\n"));
    }
}
