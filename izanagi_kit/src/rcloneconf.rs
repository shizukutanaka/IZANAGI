//! Census of an `rclone.conf` file.
//!
//! INI remotes: `[name]` stanzas each with `type =` (s3/drive/gcs/
//! b2/sftp/webdav/crypt/…) and per-backend keys (`token`, `client_id`,
//! `client_secret`, `scope`, `drive_id`, `remote`, `service_account_file`,
//! `access_key_id`, `secret_access_key`, `endpoint`, `region`, `url`,
//! `user`, `pass` (obscured), `key_file`, `chunk_size`, `password`,
//! `password2`, `filename_encryption`, `directory_name_encryption`,
//! `mount_*`/`vfs_*`). `#`/`;` comments. Counts remotes, types, keys.
//!
//! ```rust
//! let c = izanagi_kit::rcloneconf::RcloneConf::parse(
//!     b"[gdrive]\ntype = drive\ntoken = {\"x\":1}\nscope = drive\n",
//! ).unwrap();
//! assert_eq!(c.remotes, 1);
//! assert_eq!(c.types, 1);
//! ```
#![forbid(unsafe_code)]

/// rclone.conf census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RcloneConf {
    /// `[name]` remote stanzas.
    pub remotes: usize,
    /// `type =` backend declarations.
    pub types: usize,
    /// `key = value` credential/option lines.
    pub keys: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Common backend type names used by `detect`.
const TYPES: &[&str] = &[
    "s3",
    "drive",
    "gcs",
    "b2",
    "sftp",
    "webdav",
    "crypt",
    "onedrive",
    "dropbox",
    "box",
    "pcloud",
    "mega",
    "swift",
    "azureblob",
    "azurefiles",
    "ftp",
    "smb",
    "local",
    "memory",
    "union",
    "combine",
    "alias",
    "chunker",
    "compress",
    "hasher",
    "link",
    "sharefile",
    "filefabric",
    "googledrive",
    "googlecloudstorage",
    "protondrive",
    "seafile",
    "storj",
    "internetarchive",
    "quatrix",
];

/// True if `b` looks like rclone.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains('[') && t.contains("type =") && t.contains(']'))
        && (t.contains("token")
            || t.contains("client_id")
            || t.contains("scope")
            || TYPES.iter().any(|ty| t.contains(&format!("type = {ty}"))))
}

impl RcloneConf {
    /// Parse rclone.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            remotes: 0,
            types: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') || l.starts_with(';') {
                c.comments += 1;
            } else if l.starts_with('[') && l.ends_with(']') {
                c.remotes += 1;
            } else if l.contains('=') {
                let key = l.split('=').next().unwrap_or("").trim();
                if key == "type" {
                    c.types += 1;
                } else {
                    c.keys += 1;
                }
            }
        }
        if c.remotes == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# rclone\n",
            "[gdrive]\n",
            "type = drive\n",
            "token = {\"access_token\":\"x\"}\n",
            "scope = drive\n",
            "[crypt]\n",
            "type = crypt\n",
            "remote = gdrive:enc\n",
            "password = obscured\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = RcloneConf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.remotes, 2);
        assert_eq!(c.types, 2);
        assert_eq!(c.keys, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(RcloneConf::parse(b"# none\n").is_none());
    }
}
