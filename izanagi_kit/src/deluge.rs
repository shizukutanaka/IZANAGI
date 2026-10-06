//! Deluge `core.conf`/`web.conf` census.
//!
//! JSON `{"file": 1, "format": 1, "config": {...}}` wrapper with
//! `config` keys: `allow_remote`, `autoadd_enable`, `autoadd_location`,
//! `cache_expiry`, `compact_allocation`, `copy_torrent_file`,
//! `daemon_port`, `del_copy_torrent_file`, `dht`,
//! `dont_count_slow_torrents`, `download_location`, `enable_new_release_check`,
//! `enabled_plugins`, `enc_in_policy`, `enc_level`, `enc_out_policy`,
//! `geoip_db_location`, `ignore_limits_on_local_network`, `info_sent`,
//! `listen_interface`, `listen_ports`, `listen_random_port`,
//! `listen_use_sys_port`, `lsd`, `max_active_downloading`,
//! `max_active_limit`, `max_active_seeding`, `max_connections_global`,
//! `max_connections_per_second`, `max_connections_per_torrent`,
//! `max_download_speed`, `max_download_speed_per_torrent`,
//! `max_half_open_connections`, `max_upload_slots_global`,
//! `max_upload_slots_per_torrent`, `max_upload_speed`,
//! `max_upload_speed_per_torrent`, `move_completed`, `move_completed_path`,
//! `natpmp`, `new_release_check`, `outgoing_interface`, `outgoing_ports`,
//! `path_chooser_accelerator_string`, `path_chooser_auto_complete_enabled`,
//! `path_chooser_popup_on_close`, `path_chooser_recent_chooser`,
//! `path_chooser_show_hidden_files`, `path_chooser_show_size_info`,
//! `peer_tos`, `plugins_location`, `pre_allocate_storage`,
//! `proxy`, `proxy_peer_connections`, `proxy_tracker_connections`,
//! `random_outgoing_ports`, `random_port`, `rate_limit_ip_overhead`,
//! `send_info`, `share_ratio_limit`, `shared_limit`,
//! `stop_seed_at_ratio`, `stop_seed_ratio`, `super_seeding`,
//! `torrentfiles_location`, `upnp`, `utpex`, `priority_torrents`,
//! `torrent_location`, `watch_folder`.
//!
//! ```rust
//! let k = b"{\n \"file\": 1,\n \"format\": 1,\n \"config\": {\n  \"daemon_port\": 58846,\n  \"dht\": true,\n  \"download_location\": \"/x\"\n }\n}\n";
//! assert!(izanagi_kit::deluge::detect(k));
//! ```

/// deluge conf census.
#[derive(Debug, Clone)]
pub struct Deluge {
    /// `"key": value` pairs.
    pub pairs: usize,
    /// recognised deluge keys present.
    pub keys: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "allow_remote",
    "autoadd_enable",
    "autoadd_location",
    "cache_expiry",
    "compact_allocation",
    "copy_torrent_file",
    "daemon_port",
    "del_copy_torrent_file",
    "dht",
    "dont_count_slow_torrents",
    "download_location",
    "enable_new_release_check",
    "enabled_plugins",
    "enc_in_policy",
    "enc_level",
    "enc_out_policy",
    "geoip_db_location",
    "ignore_limits_on_local_network",
    "info_sent",
    "listen_interface",
    "listen_ports",
    "listen_random_port",
    "listen_use_sys_port",
    "lsd",
    "max_active_downloading",
    "max_active_limit",
    "max_active_seeding",
    "max_connections_global",
    "max_connections_per_second",
    "max_connections_per_torrent",
    "max_download_speed",
    "max_download_speed_per_torrent",
    "max_half_open_connections",
    "max_upload_slots_global",
    "max_upload_slots_per_torrent",
    "max_upload_speed",
    "max_upload_speed_per_torrent",
    "move_completed",
    "move_completed_path",
    "natpmp",
    "new_release_check",
    "outgoing_interface",
    "outgoing_ports",
    "path_chooser_accelerator_string",
    "path_chooser_auto_complete_enabled",
    "path_chooser_popup_on_close",
    "path_chooser_recent_chooser",
    "path_chooser_show_hidden_files",
    "path_chooser_show_size_info",
    "peer_tos",
    "plugins_location",
    "pre_allocate_storage",
    "proxy_peer_connections",
    "proxy_tracker_connections",
    "random_outgoing_ports",
    "random_port",
    "rate_limit_ip_overhead",
    "send_info",
    "share_ratio_limit",
    "shared_limit",
    "stop_seed_at_ratio",
    "stop_seed_ratio",
    "super_seeding",
    "torrentfiles_location",
    "priority_torrents",
    "torrent_location",
    "watch_folder",
    "utpex",
    "upnp",
    "proxy",
    "seed_time_limit",
    "remove_seed_at_ratio",
    "sequential_download",
    "force_recheck",
    "is_finished",
    "is_seed",
    "mapped_files",
    "all",
    "file_progress",
    "file_priorities",
    "prioritize_first_last_pieces",
    "seeding_time_limit",
    "upload_rate_limit",
    "download_rate_limit",
    "max_downloads",
    "max_uploads",
    "max_connections",
];

fn jkey(line: &str) -> Option<&str> {
    let s = line.trim();
    if !s.starts_with('"') {
        return None;
    }
    let end = s[1..].find('"')? + 1;
    let after = s[end + 1..].trim_start();
    if after.starts_with(':') {
        Some(&s[1..end])
    } else {
        None
    }
}

/// Detect a Deluge `*.conf` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `daemon_port`/`autoadd_location`/`del_copy_torrent_file`/
    // `dont_count_slow_torrents`/`stop_seed_*`/`*_per_torrent` are
    // deluge-exclusive key names.
    let mut n = 0usize;
    for line in t.lines() {
        if let Some(k) = jkey(line) {
            if KEYS.contains(&k) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Deluge {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            pairs: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = jkey(line) {
                c.pairs += 1;
                if KEYS.contains(&k) {
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
        let b = b"{\n \"file\": 1,\n \"format\": 1,\n \"config\": {\n  \"daemon_port\": 58846,\n  \"dht\": true,\n  \"download_location\": \"/x\"\n }\n}\n";
        assert!(detect(b));
        let c = Deluge::parse(b).unwrap();
        assert!(c.keys >= 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"{\n \"name\": \"x\",\n \"port\": 1\n}\n"));
        assert!(!detect(b"{\n \"dht\": true\n}\n"));
    }
}
