//! LVM `lvm.conf` パーサ。
//!
//! `key = value` 代入と `devices { ... }`/`global { ... }`/`activation { ... }` 等の
//! 名前付きブロック、`filter`/`use_lvmetad`/`use_devicesfile` 等既知キーを計数する。
//!
//! ```
//! use izanagi_kit::lvmconf;
//! let conf = b"devices {\n  filter = [ \"a|/dev/sda|\", \"r|.*|\" ]\n}\nglobal {\n  udev_sync = 1\n}\n";
//! assert!(lvmconf::detect(conf));
//! let c = lvmconf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 2);
//! assert_eq!(c.known_keys, 2);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 名前付き `{` ブロック数。
    pub blocks: usize,
    /// 既知ブロック名数。
    pub known_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_SECTIONS: &[&str] = &[
    "devices",
    "global",
    "activation",
    "backup",
    "log",
    "allocation",
    "local",
    "pv",
    "vg",
    "shell",
    "metadata",
    "dmeventd",
    "config",
    "profile",
    "command_profile_selection",
    "report",
    "tags",
];

const KNOWN_KEYS: &[&str] = &[
    "filter",
    "global_filter",
    "use_lvmetad",
    "use_devicesfile",
    "devicesfile",
    "cache",
    "scan_lvs",
    "allow_changes_with_duplicate_pvs",
    "md_component_detection",
    "fw_raid_component_detection",
    "prefer_udev",
    "udev_sync",
    "udev_rules",
    "locking_type",
    "locking_dir",
    "waiting_for_lock",
    "unit_prefix",
    "si_unit_consistency",
    "activation_checks",
    "mirror_region_size",
    "metadata_read_only",
    "pv_min_size",
    "reserved_memory",
    "proc",
    "data_alignment",
    "data_alignment_detection",
    "data_alignment_offset_detection",
    "issue_discards",
    "raid_fault_policy",
    "thin_check_executable",
    "thin_dump_executable",
    "thin_repair_executable",
    "cache_check_executable",
    "cache_dump_executable",
    "cache_repair_executable",
    "cache_metadata_format",
    "executable_paths",
    "abort_on_internal_errors",
    "detect_internal_vg_cache_corruption",
    "system_id",
    "local_system_id",
    "backup_enable",
    "archive_enable",
    "archive_dir",
    "backup_dir",
    "retain_min",
    "retain_days",
    "log_level",
    "log_file",
    "log_command_names",
    "vg_extend_enable",
    "test_mode",
];

/// `lvm.conf` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => (c.known_sections >= 1 && c.known_keys >= 2) || c.known_keys >= 3,
        None => false,
    }
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        blocks: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('}') {
            continue;
        }
        if let Some(pos) = t.find('{') {
            c.blocks += 1;
            let name = t[..pos].trim().to_ascii_lowercase();
            if KNOWN_SECTIONS.contains(&name.as_str()) {
                c.known_sections += 1;
            }
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim().to_ascii_lowercase();
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key.as_str()) {
            c.known_keys += 1;
        }
    }
    (c.entries > 0 || c.blocks > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"devices {\n  global_filter = [ \"a|/dev/sda|\", \"r|.*|\" ]\n  use_devicesfile = 0\n}\nglobal {\n  udev_sync = 1\n  locking_type = 1\n}\nactivation {\n  mirror_region_size = 4096\n}\n";

    #[test]
    fn detects_lvmconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.blocks, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.entries, 5);
        assert_eq!(c.known_keys, 5);
    }

    #[test]
    fn rejects_generic_ini() {
        let ini = b"[a]\nx = 1\ny = 2\n";
        assert!(!detect(ini));
    }
}
