//! ncmpcpp `config`/`bindings` census.
//!
//! ncmpcpp config is `key = value` lines: `mpd_host`/`mpd_port`/
//! `ncmpcpp_directory`/`visualizer_data_source`/
//! `display_screens_numbers_on_start`/`song_columns_list_format`/
//! `song_list_format`/`song_status_format`/`song_window_title_format`/
//! `execute_on_song_change`/`ignore_leading_the`/`regular_expressions`/
//! `block_item_constraints`/`header_visibility`/`statusbar_visibility`/
//! `titles_visibility`/`progressbar_look`/`colors_enabled`/`*_color`.
//! The bindings file uses `def_key "key"`/`def_key "key" … push_*`.
//!
//! ```rust
//! let c = izanagi_kit::ncmpcpp::Ncmpcpp::parse(b"mpd_host = localhost\nmpd_port = 6600\n").unwrap();
//! assert_eq!(c.settings, 2);
//! ```

/// ncmpcpp config census.
#[derive(Debug, Clone)]
pub struct Ncmpcpp {
    /// `key = value` settings.
    pub settings: usize,
    /// `*_color` appearance keys.
    pub colors: usize,
    /// `def_key`/`push_*` bindings-file entries.
    pub bindings: usize,
    /// `#` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "mpd_host",
    "mpd_port",
    "mpd_connection_timeout",
    "mpd_music_dir",
    "ncmpcpp_directory",
    "visualizer_data_source",
    "display_screens_numbers_on_start",
    "song_columns_list_format",
    "song_list_format",
    "song_status_format",
    "song_window_title_format",
    "execute_on_song_change",
    "ignore_leading_the",
    "regular_expressions",
    "block_item_constraints",
    "header_visibility",
    "statusbar_visibility",
    "titles_visibility",
    "progressbar_look",
    "colors_enabled",
    "autocenter_mode",
    "centered_cursor",
    "cyclic_scrolling",
    "mouse_support",
    "volume_change_step",
    "search_engine",
    "startup_screen",
    "default_place_to_search_in",
    "delayed_seeking_period",
    "visualizer_type",
    "visualizer_in_stereo",
    "visualizer_fifo_path",
];

/// Whether the buffer looks like an ncmpcpp config/bindings file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    KEYS.iter().filter(|k| t.contains(**k)).count() >= 2
        || t.contains("def_key")
        || (t.contains("mpd_") && t.contains('='))
}

impl Ncmpcpp {
    /// Parse an ncmpcpp config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            colors: 0,
            bindings: 0,
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
            if s.starts_with("def_key") || s.starts_with("def_visual_key") || s.contains("push_") {
                c.bindings += 1;
            } else if s.contains('=') {
                c.settings += 1;
                let key = s.split('=').next().unwrap_or("").trim();
                if key.ends_with("_color") || key == "colors_enabled" {
                    c.colors += 1;
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
    fn parses_config() {
        let b = concat!(
            "mpd_host = localhost\n",
            "mpd_port = 6600\n",
            "ncmpcpp_directory = ~/.ncmpcpp\n",
            "visualizer_data_source = /tmp/fifo\n",
            "colors_enabled = yes\n",
            "main_window_color = white\n",
            "header_window_color = cyan\n",
            "progressbar_color = black\n",
            "# comment\n",
        );
        let c = Ncmpcpp::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 8);
        assert_eq!(c.colors, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn parses_bindings() {
        let c = Ncmpcpp::parse(b"def_key \"up\"\n  scroll_up\n\ndef_key \"q\"\n  quit\n").unwrap();
        assert_eq!(c.bindings, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Ncmpcpp::parse(b"foo = bar\n").is_none());
    }
}
