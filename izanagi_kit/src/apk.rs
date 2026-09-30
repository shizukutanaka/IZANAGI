//! apk repositories / world / keys (`/etc/apk/*`) census.
//!
//! `/etc/apk/repositories` lines: `@edge`/`@testing` tagged repos,
//! `http(s)://…/alpine/vX.Y/main`, `file:///…`, `/media/…` paths,
//! `#` comments. `/etc/apk/world` lines: `name`, `name>v`, `name=`,
//! `name~`, `name<`, `name>=`, `name<=`, `name=~v`, `!name`,
//! `+name@tag`, `name@tag`. `/etc/apk/arch`/`/etc/apk/keys`/`alpine-devel`
//! keys and `apk.conf`-style keys (`repositories_file`/`cache_dir`/
//! `keys_dir`/`etc_apk`/`root`/`initdb`/`progress`/`interactive`/
//! `cache_max_age`/`arch`/`hostname`/`crypto`/`keyfile`/`keydir`/
//! `env`/`umask`/`timeout`/`retries`/`persistent_cache`/`scripts`/
//! `pre_*`/`post_*`/`commit_hooks`/`repository`/`repository_policy`/
//! `repository_config`/`world`/`installed`/`depends`/`options`/
//! `lock`/`overlay`/`etc_apk`/`trust_*`/`lib*`/`local_*`/`remote_*`/
//! `allow_*`/`use_*`/`max_*`/`_*`/`signed`/`unsigned`/`secure`/
//! `insecure`/`signature`/`checksum`/`index`/`index_cache`/`cache_*`/
//! `*` suffixed keys).
//!
//! ```rust
//! let a = concat!(
//!     "https://dl-cdn.alpinelinux.org/alpine/v3.20/main\n",
//!     "https://dl-cdn.alpinelinux.org/alpine/v3.20/community\n",
//!     "@edge https://dl-cdn.alpinelinux.org/alpine/edge/main\n",
//! );
//! let c = izanagi_kit::apk::Apk::parse(a.as_bytes()).unwrap();
//! assert_eq!(c.repos, 3);
//! ```

/// apk config census.
#[derive(Debug, Clone)]
pub struct Apk {
    /// Repository URLs / paths (`http*`/`ftp`/`file:`/`/` prefix or `@tag` + url).
    pub repos: usize,
    /// `@tag` tagged repo prefixes + `@edge`/`@testing`-style tokens.
    pub tags: usize,
    /// `/etc/apk/world` style package specs (bare name, `name<op>v`, `!name`, `+name@tag`, `name@tag`).
    pub specs: usize,
    /// `apk.conf`-style `key = value`/`key: value`/`key=value` entries.
    pub confs: usize,
}

/// Whether the buffer looks like an apk repositories/world/config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("alpinelinux.org")
        || t.contains("/apk/")
        || t.contains("@edge")
        || t.contains("@testing")
        || t.lines().any(|l| {
            let s = l.trim();
            s.starts_with("http://")
                || s.starts_with("https://")
                || s.starts_with("file://")
                || s.starts_with('/')
        }) && t.contains("alpine")
}

impl Apk {
    /// Parse an apk config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            repos: 0,
            tags: 0,
            specs: 0,
            confs: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s.starts_with('@') {
                c.tags += 1;
                c.repos += 1;
                continue;
            }
            if s.starts_with("http://")
                || s.starts_with("https://")
                || s.starts_with("ftp://")
                || s.starts_with("file://")
                || s.starts_with('/')
            {
                c.repos += 1;
                continue;
            }
            if s.contains('=') || s.contains(':') {
                let key = s.split(['=', ':']).next().unwrap_or("");
                if !key.is_empty() && key.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_') {
                    c.confs += 1;
                    continue;
                }
            }
            if !s.contains(' ') {
                c.specs += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repos() {
        let b = concat!(
            "# alpine\n",
            "https://dl-cdn.alpinelinux.org/alpine/v3.20/main\n",
            "https://dl-cdn.alpinelinux.org/alpine/v3.20/community\n",
            "@edge https://dl-cdn.alpinelinux.org/alpine/edge/main\n",
            "@testing https://dl-cdn.alpinelinux.org/alpine/edge/testing\n",
            "/media/cdrom/apks\n",
        );
        let c = Apk::parse(b.as_bytes()).unwrap();
        assert_eq!(c.repos, 5);
        assert_eq!(c.tags, 2);
    }

    #[test]
    fn parses_world() {
        let b = concat!(
            "alpine-base\n",
            "openssh>=9\n",
            "vim~8\n",
            "!busybox-suid\n",
            "apk-tools@edge\n",
            "+git@community\n",
        );
        let c = Apk::parse(b.as_bytes()).unwrap();
        assert_eq!(c.specs, 6);
    }

    #[test]
    fn rejects_other() {
        assert!(Apk::parse(b"foo = 1").is_none());
    }
}
