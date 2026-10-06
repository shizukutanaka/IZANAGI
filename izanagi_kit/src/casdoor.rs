//! Casdoor `app.conf` INI config census.
//!
//! Casdoor flat `key = value` config: `appname`, `httpport`,
//! `runmode`, `copyrequestbody`, `sessionOn`, `origin`,
//! `staticBaseURL`, `isDemoMode`, `batchSize`, `quota`,
//! `logConfig`, `redisEndpoint`, `channelBufferSize`,
//! `useProxy`, `verificationCodeTimeout`, `initScore`,
//! `logPostOnly`, `isUsernameLowered`, `frontendBaseDir`,
//! `driverName`, `dataSourceName`, `dbType`, `db`.
//!
//! ```rust
//! let k = b"appname = casdoor\nhttpport = 8000\norigin = https://door.example.com\nstaticBaseURL = https://cdn.example.com\n";
//! assert!(izanagi_kit::casdoor::detect(k));
//! ```

/// Casdoor config census.
#[derive(Debug, Clone)]
pub struct Casdoor {
    /// `key =` assignment lines.
    pub assignments: usize,
    /// recognised Casdoor keys present.
    pub keys: usize,
    /// `[section]` lines.
    pub sections: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "origin",
    "staticBaseURL",
    "isDemoMode",
    "batchSize",
    "quota",
    "logConfig",
    "redisEndpoint",
    "channelBufferSize",
    "useProxy",
    "verificationCodeTimeout",
    "initScore",
    "logPostOnly",
    "isUsernameLowered",
    "frontendBaseDir",
    "isCloudIntranet",
    "sessionExpireTime",
];

const WEAK: &[&str] = &[
    "appname",
    "httpport",
    "runmode",
    "copyrequestbody",
    "sessionOn",
    "db",
    "dataSourceName",
    "dbType",
    "driverName",
    "master",
    "slaveId",
    "enableGzip",
    "pluginRepos",
];

fn assign_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') || s.starts_with(';') || s.starts_with('[') {
        return None;
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"');
            if k.is_empty() || k.contains(' ') {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a Casdoor `app.conf`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // beego-style shared keys (`appname`/`httpport`/`runmode`) alone are
    // not evidence; require Casdoor-exclusive keys.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = assign_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Casdoor {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            assignments: 0,
            keys: 0,
            sections: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') {
                c.sections += 1;
                continue;
            }
            if let Some(k) = assign_key(line) {
                c.assignments += 1;
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
        let b = b"appname = casdoor\nhttpport = 8000\norigin = https://door.example.com\nstaticBaseURL = https://cdn.example.com\n";
        assert!(detect(b));
        let c = Casdoor::parse(b).unwrap();
        assert_eq!(c.assignments, 4);
        assert_eq!(c.keys, 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"appname = x\nhttpport = 8000\n"));
        assert!(!detect(b"# origin = x\n# staticBaseURL = y\n"));
        assert!(!detect(b"name = pkg\nversion = 1.0\n"));
    }
}
