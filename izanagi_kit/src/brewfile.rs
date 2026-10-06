//! Homebrew `Brewfile` census.
//!
//! A Brewfile is Ruby DSL: `tap "user/repo"`, `brew "name"`,
//! `cask "name"`, `mas "name", id: 1`, `vscode "ext"`,
//! `whalebrew "img"` calls, plus `freeze`/`conflicts_with`
//! directives and `#` comments.
//!
//! ```rust
//! let k = b"tap \"homebrew/bundle\"\nbrew \"git\"\nbrew \"ripgrep\", args: [\"with-short-names\"]\ncask \"firefox\"\n";
//! assert!(izanagi_kit::brewfile::detect(k));
//! ```

/// Brewfile census.
#[derive(Debug, Clone)]
pub struct Brewfile {
    /// `brew "…"`/ `brew […]` calls.
    pub brews: usize,
    /// `cask "…"` calls.
    pub casks: usize,
    /// `tap "…"` calls.
    pub taps: usize,
    /// `mas`/`vscode`/`whalebrew` calls.
    pub others: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const CALLS: &[&str] = &["brew", "cask", "tap", "mas", "vscode", "whalebrew"];

fn call(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    for c in CALLS {
        if let Some(rest) = s.strip_prefix(c) {
            let rest = rest.trim_start();
            if rest.starts_with('"') || rest.starts_with('\'') || rest.starts_with('[') {
                return Some(c);
            }
        }
    }
    None
}

/// Detect a Homebrew `Brewfile`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // a single `brew "x"` line could be Ruby code in another context;
    // require two DSL calls to be safe.
    t.lines().filter(|l| call(l).is_some()).count() >= 2
}

impl Brewfile {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            brews: 0,
            casks: 0,
            taps: 0,
            others: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            match call(line) {
                Some("brew") => c.brews += 1,
                Some("cask") => c.casks += 1,
                Some("tap") => c.taps += 1,
                Some(_) => c.others += 1,
                None => {}
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
        let b = b"tap \"homebrew/bundle\"\nbrew \"git\"\ncask \"firefox\"\n";
        assert!(detect(b));
        let c = Brewfile::parse(b).unwrap();
        assert_eq!(c.taps, 1);
        assert_eq!(c.brews, 1);
        assert_eq!(c.casks, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"brew install git\n"));
        assert!(!detect(b"# brew \"git\"\n# cask \"x\"\n"));
    }
}
