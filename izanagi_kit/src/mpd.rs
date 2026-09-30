//! Music Player Daemon `mpd.conf` census.
//!
//! mpd.conf is `key "value"`/`key value` lines plus brace blocks:
//! `audio_output { type "alsa" name "My ALSA" }`,
//! `playlist_plugin { … }`, `decoder { … }`,
//! `resampler { … }`, `input { … }`, `filter { … }`.
//! Flat keys include `bind_to_address`/`port`/`music_directory`/
//! `db_file`/`user`/`group`/`password`/`mixer_type`/`replaygain`/
//! `volume_normalization`/`audio_buffer_size`/`max_connections`.
//!
//! ```rust
//! let c = izanagi_kit::mpd::Mpd::parse(b"music_directory \"/music\"\naudio_output {\n}\n").unwrap();
//! assert_eq!(c.settings, 1);
//! ```

/// `mpd.conf` census.
#[derive(Debug, Clone)]
pub struct Mpd {
    /// `key "value"` settings (inside and outside blocks).
    pub settings: usize,
    /// `name {` blocks (audio_output/playlist_plugin/decoder/…).
    pub blocks: usize,
    /// `#` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "bind_to_address",
    "port",
    "music_directory",
    "playlist_directory",
    "db_file",
    "log_file",
    "pid_file",
    "state_file",
    "sticker_file",
    "user",
    "group",
    "password",
    "default_permissions",
    "host_permissions",
    "proxy_host",
    "proxy_port",
    "mixer_type",
    "replaygain",
    "replaygain_preamp",
    "replaygain_missing_preamp",
    "replaygain_limit",
    "volume_normalization",
    "audio_buffer_size",
    "buffer_before_play",
    "max_connections",
    "max_playlist_length",
    "auto_update",
    "follow_outside_symlinks",
    "follow_inside_symlinks",
    "zeroconf_enabled",
    "zeroconf_name",
    "despotify_user",
    "filesystem_charset",
    "metadata_to_use",
    "gapless_mp3_playback",
    "connection_timeout",
    "restore_paused",
    "save_absolute_paths_in_playlists",
    "audio_output_format",
    "samplerate_converter",
];

const BLOCKS: &[&str] = &[
    "audio_output",
    "playlist_plugin",
    "decoder",
    "resampler",
    "input",
    "filter",
    "input_cache",
    "neighbors",
];

/// Whether the buffer looks like an mpd.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    KEYS.iter().filter(|k| t.contains(**k)).count() >= 2
        || BLOCKS
            .iter()
            .any(|k| t.contains(&format!("{k} ")) && t.contains('{'))
}

impl Mpd {
    /// Parse an mpd.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            blocks: 0,
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
            if s == "{" || s == "}" {
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if BLOCKS.contains(&head) && s.contains('{') {
                c.blocks += 1;
            } else if s.len() > head.len() {
                c.settings += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mpdconf() {
        let b = concat!(
            "# mpd.conf\n",
            "music_directory \"/music\"\n",
            "playlist_directory \"/playlists\"\n",
            "bind_to_address \"localhost\"\n",
            "port \"6600\"\n",
            "user \"mpd\"\n",
            "audio_output {\n",
            "  type \"alsa\"\n",
            "  name \"My ALSA\"\n",
            "  mixer_type \"software\"\n",
            "}\n",
            "playlist_plugin {\n",
            "  name \"m3u\"\n",
            "  enabled \"true\"\n",
            "}\n",
        );
        let c = Mpd::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 10);
        assert_eq!(c.blocks, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Mpd::parse(b"key value\n").is_none());
    }
}
