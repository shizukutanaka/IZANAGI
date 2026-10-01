//! MusicBrainz Picard `Picard.ini` census.
//!
//! Picard stores settings in a Qt-style INI file (`Picard.ini`):
//! `[application]`, `[setting]`, `[plugins]` sections plus keys like
//! `server_host`/`server_port`/`username`/`save_images_to_files`/
//! `save_images_to_tags`/`enabled_plugins`/`file_renaming_scripts`/
//! `move_files`/`rename_files`/`ca_providers`/`oauth_*`/`fingerprinting`/
//! `acoustid_*`/`caa_*`/`track_`/`release_` options.
//!
//! ```rust
//! let c = izanagi_kit::picard::Picard::parse(b"[application]\nserver_host=musicbrainz.org\nsave_images_to_files=true\n").unwrap();
//! assert_eq!(c.settings, 2);
//! ```

/// Picard `Picard.ini` census.
#[derive(Debug, Clone)]
pub struct Picard {
    /// `[section]` headers.
    pub sections: usize,
    /// `key=value` settings.
    pub settings: usize,
    /// `true`/`false` settings.
    pub booleans: usize,
    /// `;`/`#` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "server_host",
    "server_port",
    "username",
    "save_images_to_files",
    "save_images_to_tags",
    "enabled_plugins",
    "file_renaming_scripts",
    "move_files",
    "rename_files",
    "ca_providers",
    "oauth_access_token",
    "oauth_refresh_token",
    "fingerprinting",
    "acoustid_fpcalc",
    "acoustid_apikey",
    "caa_image_size",
    "track_",
    "release_",
    "window",
    "media_optical",
    "ignore",
];

/// Whether the buffer looks like a Picard.ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    KEYS.iter().filter(|k| t.contains(**k)).count() >= 2 || t.contains("[application]")
}

impl Picard {
    /// Parse a Picard.ini into census counts.
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
            if s.starts_with(';') || s.starts_with('#') {
                c.comments += 1;
            } else if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
            } else if s.contains('=') {
                c.settings += 1;
                let v = s.split('=').nth(1).unwrap_or("").trim();
                if v == "true" || v == "false" || v == "True" || v == "False" {
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
    fn parses_picard_ini() {
        let b = concat!(
            "[application]\n",
            "version=2.9\n",
            "server_host=musicbrainz.org\n",
            "server_port=443\n",
            "username=picarduser\n",
            "[setting]\n",
            "save_images_to_files=true\n",
            "save_images_to_tags=true\n",
            "move_files=false\n",
            "rename_files=true\n",
            "enabled_plugins=\n",
            "caa_image_size=500\n",
            "[plugins]\n",
            "plugin1=x\n",
            "; comment\n",
        );
        let c = Picard::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.settings, 11);
        assert_eq!(c.booleans, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Picard::parse(b"[foo]\nbar=baz\n").is_none());
    }
}
