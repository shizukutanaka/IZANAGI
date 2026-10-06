//! Bazarr `config.ini`/`config.yaml` census.
//!
//! Bazarr writes `config.ini` (sections: `[auth]`, `[sonarr]`,
//! `[radarr]`, `[general]`, `[backup]`, `[subtitles]`, `[embeddedsubtitles]`,
//! `[postprocessing]`, `[proxy]`, `[schedule]`, `[ui]`, `[analytics]`,
//! `[notifications]`, `[settings]`/flattened `[provider]`/`[subsync]`)
//! — the YAML form mirrors it with top-level `auth:`/`sonarr:`/`radarr:`/
//! `general:`/`backup:`/`subtitles:`/`postprocessing:`/`proxy:`/`schedule:`/
//! `ui:`/`analytics:`/`subsync:`/`providers:`/`notification` keys plus
//! Bazarr-only leaf names like `use_sonarr`, `sonarr_url`,
//! `radarr_url`, `enabled_providers`, `single_language`,
//! `minimum_score`, `adapted_subtitles`, `use_postprocessing`,
//! `postprocessing_cmd`, `postprocessing_threshold`,
//! `upgrade_subs`, `upgrade_manual`, `days_to_upgrade_subs`,
//! `minimum_score_movie`, `minimum_score_series`, `use_embedded_subs`,
//! `embedded_ass`, `embedded_fallbacks`, `serie_default_language`,
//! `serie_default_audio`, `movie_default_language`,
//! `movie_default_audio`, `download_cancelled`, `wl_token`,
//! `sonarr_sync`, `radarr_sync`, `series_sync`, `movies_sync`,
//! `wanted_search_interval`, `subfolder`, `path_mappings`,
//! `path_mappings_movie`, `path_mappings_series`, `page_size`,
//! `seriefolder_format`, `moviefolder_format`.
//!
//! ```rust
//! let k = b"[general]\nip = 0.0.0.0\nport = 6767\n[sonarr]\nip = 127.0.0.1\nport = 8989\napikey = x\n[radarr]\nip = 127.0.0.1\napikey = y\n";
//! assert!(izanagi_kit::bazarr::detect(k));
//! ```

/// Bazarr config census.
#[derive(Debug, Clone)]
pub struct Bazarr {
    /// `[x]`/`x:` section headers.
    pub sections: usize,
    /// `key = value`/`key: value` assignments.
    pub settings: usize,
    /// recognised bazarr markers present.
    pub keys: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "sonarr",
    "radarr",
    "general",
    "backup",
    "subtitles",
    "embeddedsubtitles",
    "postprocessing",
    "schedule",
    "ui",
    "analytics",
    "notifications",
    "settings",
    "subsync",
    "provider",
    "auth",
    "proxy",
    "providers",
];

const MARKERS: &[&str] = &[
    "use_sonarr",
    "sonarr_url",
    "radarr_url",
    "enabled_providers",
    "single_language",
    "minimum_score",
    "adapted_subtitles",
    "use_postprocessing",
    "postprocessing_cmd",
    "postprocessing_threshold",
    "upgrade_subs",
    "upgrade_manual",
    "days_to_upgrade_subs",
    "minimum_score_movie",
    "minimum_score_series",
    "use_embedded_subs",
    "embedded_ass",
    "embedded_fallbacks",
    "serie_default_language",
    "serie_default_audio",
    "movie_default_language",
    "movie_default_audio",
    "download_cancelled",
    "wl_token",
    "sonarr_sync",
    "radarr_sync",
    "series_sync",
    "movies_sync",
    "wanted_search_interval",
    "path_mappings_movie",
    "path_mappings_series",
    "seriefolder_format",
    "moviefolder_format",
    "page_size",
    "subfolder",
    "sonarr",
    "radarr",
    "bazarr",
    "subzero",
    "subsync",
    "opensubtitles",
    "addic7ed",
    "subtitulamos",
    "supersubtitles",
    "tvsubtitles",
    "podnapisi",
    "bsplayer",
    "xsubs",
    "hdbits",
    "napiprojekt",
    "betaseries",
    "animetosho",
    "whisperai",
    "deep_translator",
    "lingarr",
];

fn section(line: &str) -> Option<&str> {
    let s = line.trim();
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim())
}

fn marker(line: &str) -> bool {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
        return false;
    }
    MARKERS.iter().any(|m| s.contains(m))
}

/// Detect a Bazarr `config.ini`/`config.yaml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `[sonarr]`/`[radarr]`/`sonarr_url`/`use_embedded_subs`/
    // `enabled_providers`/`opensubtitles` are bazarr-exclusive.
    let mut n = 0usize;
    for line in t.lines() {
        if let Some(sec) = section(line) {
            if SECTIONS.contains(&sec) {
                n += 1;
            }
        } else if marker(line) {
            n += 1;
        }
    }
    n >= 3
}

impl Bazarr {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            keys: 0,
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
            if let Some(sec) = section(line) {
                c.sections += 1;
                if SECTIONS.contains(&sec) {
                    c.keys += 1;
                }
            } else if s.contains('=') || s.contains(':') {
                c.settings += 1;
                if marker(line) {
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
        let b = b"[general]\nip = 0.0.0.0\nport = 6767\n[sonarr]\nip = 127.0.0.1\nport = 8989\napikey = x\n[radarr]\nip = 127.0.0.1\napikey = y\n";
        assert!(detect(b));
        let c = Bazarr::parse(b).unwrap();
        assert!(c.keys >= 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[server]\nport = 1\n[db]\nhost = x\n"));
        assert!(!detect(
            b"# [sonarr]\n# [radarr]\n# enabled_providers = x\n[y]\nz = 1\n"
        ));
    }
}
