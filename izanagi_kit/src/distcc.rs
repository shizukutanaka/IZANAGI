//! distcc `hosts` ファイル（分散コンパイルホスト一覧）の検出と
//! 構造カウント。
//!
//! `HOST`/`HOST:PORT`/`HOST/N`/`HOST:PORT/N,cpp,lzo`/`--randomize`/
//! `+zeroconf`/`localhost` 形式のホスト指定行を識別する。
//!
//! ```
//! let b = b"localhost/8\n192.168.1.10:3632/4\nbuilder2/4,cpp,lzo\n--randomize\n";
//! assert!(izanagi_kit::distcc::detect(b));
//! let c = izanagi_kit::distcc::Distcc::parse(b).unwrap();
//! assert_eq!(c.hosts, 3);
//! assert_eq!(c.options, 1);
//! ```

/// Parsed distcc hosts summary.
#[derive(Debug, Clone)]
pub struct Distcc {
    /// Host-spec lines (`HOST`, `HOST/N`, `HOST:PORT/N,...`).
    pub hosts: usize,
    /// Option lines (`--randomize`, `+zeroconf`).
    pub options: usize,
    /// `#` comment lines.
    pub comments: usize,
}

fn host_spec(tr: &str) -> bool {
    if tr.is_empty() || tr.starts_with('#') {
        return false;
    }
    if tr.starts_with("--randomize") || tr.starts_with("+zeroconf") {
        return false;
    }
    // Strip option suffix `,cpp,lzo`
    let head = tr.split(',').next().unwrap_or("");
    // `host[/N]` or `host:port[/N]`
    let mut it = head.split('/');
    let hostport = it.next().unwrap_or("");
    if hostport.is_empty() || hostport.starts_with('-') || hostport.contains(char::is_whitespace) {
        return false;
    }
    // Optional port must be digits
    let hparts: Vec<&str> = hostport.split(':').collect();
    if hparts.len() > 2 {
        return false;
    }
    if hparts.len() == 2 && hparts[1].chars().any(|c| !c.is_ascii_digit()) {
        return false;
    }
    // Host part must look like hostname/IP/localhost
    let host = hparts[0];
    let ok_host = host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        && !host.is_empty();
    if !ok_host {
        return false;
    }
    // Optional `/N` slot count must be digits
    match it.next() {
        None => true,
        Some(n) => !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()),
    }
}

fn option_line(tr: &str) -> bool {
    tr.starts_with("--randomize") || tr.starts_with("+zeroconf")
}

/// Detect a distcc hosts file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hosts = 0usize;
    let mut opts = 0usize;
    let mut other = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if host_spec(tr) {
            hosts += 1;
        } else if option_line(tr) {
            opts += 1;
        } else {
            other += 1;
        }
    }
    (hosts >= 2 && other == 0) || (hosts >= 1 && opts >= 1 && other == 0) || hosts >= 4
}

impl Distcc {
    /// Count categories. Returns `None` when the input does not look like
    /// a distcc hosts file.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            hosts: 0,
            options: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if host_spec(tr) {
                c.hosts += 1;
            } else if option_line(tr) {
                c.options += 1;
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`Distcc::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<Distcc> {
    Distcc::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# build farm\nlocalhost/8\n192.168.1.10:3632/4\n192.168.1.11/4,cpp,lzo\nbuilder2/4,cpp\nbuilder3:3632/2,lzo\n--randomize\n";
        assert!(detect(b));
        let c = Distcc::parse(b).unwrap();
        assert_eq!(c.hosts, 5);
        assert_eq!(c.options, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_zeroconf() {
        assert!(detect(b"+zeroconf\nlocalhost/4\n"));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"127.0.0.1 localhost\n::1 localhost\n"));
        assert!(!detect(b"example line one\nanother line\n"));
        assert!(!detect(b"option = value\nkey = 2\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(Distcc::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"localhost/8");
        assert!(!detect(&b));
    }
}
