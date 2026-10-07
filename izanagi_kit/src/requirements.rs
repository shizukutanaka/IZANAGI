//! `requirements.txt` census.
//!
//! `pkg[extras]==1.0`/`pkg>=1`/`pkg~=1`/`pkg` lines plus
//! `-r other.txt`/`-c constraints.txt`/`-e .`/`--index-url`/`--extra-index-url`/
//! `--find-links`/`--no-index`/`--pre`/`--hash=sha256:…`/`#` comments and
//! environment markers (`; python_version < "3"`).
//!
//! ```rust
//! let r = "# deps\nflask==3.0\nrequests[security]>=2\n-r base.txt\n-e .\n";
//! let c = izanagi_kit::requirements::Requirements::parse(r.as_bytes()).unwrap();
//! assert_eq!(c.specs, 2);
//! assert_eq!(c.pinned, 1);
//! ```

/// requirements.txt census.
#[derive(Debug, Clone)]
pub struct Requirements {
    /// Package spec lines (`name[extra] op version`).
    pub specs: usize,
    /// `==` pinned specs.
    pub pinned: usize,
    /// `>=`/`<=`/`~=`/`!=`/`>`/`<` ranged specs.
    pub ranged: usize,
    /// `-r`/`-c` include lines.
    pub includes: usize,
    /// `-e`/`--*` option lines.
    pub options: usize,
    /// `[extras]` markers seen.
    pub extras: usize,
    /// `; marker` environment markers.
    pub markers: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Whether the buffer looks like a requirements.txt.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut specs = 0;
    let mut opts = 0;
    let mut content = 0;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        content += 1;
        if s.starts_with('-') {
            opts += 1;
            continue;
        }
        if is_spec_line(s) {
            specs += 1;
        }
    }
    // requirements files are *mostly* spec/option lines — a stray pair
    // of bare words inside an unrelated file does not qualify
    specs + opts >= 2 && (specs >= 1 || opts >= 2) && (specs + opts) * 2 >= content
}

/// A requirement spec line: `name`, `name[extras]`, `name op version`
/// (`==`/`>=`/`<=`/`~=`/`!=`/`>`/`<`), `name @ url` or `name ; marker`.
/// The name is `alnum (alnum|- _ .)*`; anything after it must begin
/// with an extras/version/marker/direct-ref introducer, so prose like
/// `Hello world` does not count.
fn is_spec_line(s: &str) -> bool {
    let b = s.as_bytes();
    if !b[0].is_ascii_alphanumeric() {
        return false;
    }
    let mut i = 0;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'-' | b'_' | b'.')) {
        i += 1;
    }
    let rest = s[i..].trim_start();
    rest.is_empty()
        || rest.starts_with('[')
        || rest.starts_with(';')
        || rest.starts_with('@')
        || ["==", ">=", "<=", "~=", "!=", ">", "<", "==="]
            .iter()
            .any(|op| rest.starts_with(op))
}

impl Requirements {
    /// Parse a requirements.txt into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            specs: 0,
            pinned: 0,
            ranged: 0,
            includes: 0,
            options: 0,
            extras: 0,
            markers: 0,
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
            if s.starts_with("-r ")
                || s.starts_with("-c ")
                || s.starts_with("--requirement")
                || s.starts_with("--constraint")
            {
                c.includes += 1;
                continue;
            }
            if s.starts_with('-') {
                c.options += 1;
                continue;
            }
            c.specs += 1;
            if s.contains("==") {
                c.pinned += 1;
            } else if s.contains(">=")
                || s.contains("<=")
                || s.contains("~=")
                || s.contains("!=")
                || s.contains('>')
                || s.contains('<')
            {
                c.ranged += 1;
            }
            if s.contains('[') {
                c.extras += 1;
            }
            if s.contains(';') {
                c.markers += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reqs() {
        let b = concat!(
            "# base requirements\n",
            "flask==3.0\n",
            "requests[security]>=2.31\n",
            "numpy~=1.26; python_version >= \"3.9\"\n",
            "pytest\n",
            "-r common.txt\n",
            "-c constraints.txt\n",
            "-e .\n",
            "--index-url https://pypi.org/simple\n",
            "--hash=sha256:abc\n",
        );
        let c = Requirements::parse(b.as_bytes()).unwrap();
        assert_eq!(c.specs, 4);
        assert_eq!(c.pinned, 1);
        assert_eq!(c.ranged, 2);
        assert_eq!(c.includes, 2);
        assert_eq!(c.options, 3);
        assert_eq!(c.extras, 1);
        assert_eq!(c.markers, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Requirements::parse(b"hello world").is_none());
    }
}
