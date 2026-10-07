//! Factorio `server-settings.json`/`map-settings.json`/
//! `map-gen-settings.json` 検出モジュール。
//!
//! Factorio 設定は JSON で、server-settings は
//! `"name"`/`"description"`/`"tags"`/`"max_players"`/`"visibility"`/
//! `"username"`/`"token"`/`"game_password"`/`"verify_user_identity"`/
//! `"allow_commands"`/`"autosave_interval"`/`"autosave_slots"`/
//! `"afk_autokick_interval"`/`"non_blocking_saving"`/
//! `"minimum_segment_size"`/`"ignore_player_limit_for_returning_players"`/
//! `"allow_debug_settings"`/`"only_admins_can_pause_the_game"`/
//! `"autosave_only_on_server"`/`"require_user_verification"`/
//! `"max_upload_in_kilobytes_per_second"`/`"max_upload_slots"`/
//! `"max_heartbeats_per_second"`/`"server_query"`/`"public"`/`"lan"`、
//! map-settings は `"pollution"`/`"enemy_evolution"`/
//! `"enemy_expansion"`/`"unit_group"`/`"steering"`/`"path_finder"`/
//! `"max_failed_behavior_count"`/`"difficulty_settings"`/
//! `"spoiling_time_penalty"`、map-gen-settings は `"seed"`/
//! `"width"`/`"height"`/`"starting_area"`/`"peaceful_mode"`/
//! `"autoplace_controls"`/`"cliff_settings"`/`"property_expression_names"`
//! 等のキーで構成される。
//!
//! ```
//! let b = b"{\n  \"name\": \"My Game\",\n  \"description\": \"desc\",\n  \"max_players\": 0,\n  \"visibility\": {\"public\": true, \"lan\": true},\n  \"autosave_interval\": 10\n}\n";
//! let c = izanagi_kit::factoriosettings::parse(b);
//! assert!(izanagi_kit::factoriosettings::detect(b));
//! assert_eq!(c.keys, 7);
//! ```

const KEYS: &[&str] = &[
    "afk_autokick_interval",
    "allow_commands",
    "allow_debug_settings",
    "autoplace_controls",
    "autosave_interval",
    "autosave_only_on_server",
    "autosave_slots",
    "cliff_settings",
    "default_enable_all_autoplace_controls",
    "description",
    "difficulty_settings",
    "enemy_evolution",
    "enemy_expansion",
    "game_password",
    "height",
    "ignore_player_limit_for_returning_players",
    "lan",
    "map_settings",
    "max_failed_behavior_count",
    "max_heartbeats_per_second",
    "max_players",
    "max_upload_in_kilobytes_per_second",
    "max_upload_slots",
    "minimum_segment_size",
    "name",
    "non_blocking_saving",
    "only_admins_can_pause_the_game",
    "path_finder",
    "peaceful_mode",
    "pollution",
    "property_expression_names",
    "public",
    "require_user_verification",
    "seed",
    "server_query",
    "spoiling_time_penalty",
    "starting_area",
    "steering",
    "tags",
    "token",
    "unit_group",
    "username",
    "verify_user_identity",
    "visibility",
    "width",
];

const SIGNATURE: &[&str] = &[
    "afk_autokick_interval",
    "allow_commands",
    "allow_debug_settings",
    "autoplace_controls",
    "autosave_interval",
    "autosave_only_on_server",
    "autosave_slots",
    "cliff_settings",
    "default_enable_all_autoplace_controls",
    "difficulty_settings",
    "enemy_evolution",
    "enemy_expansion",
    "game_password",
    "ignore_player_limit_for_returning_players",
    "map_settings",
    "max_failed_behavior_count",
    "max_heartbeats_per_second",
    "max_players",
    "max_upload_in_kilobytes_per_second",
    "max_upload_slots",
    "minimum_segment_size",
    "non_blocking_saving",
    "only_admins_can_pause_the_game",
    "path_finder",
    "peaceful_mode",
    "pollution",
    "property_expression_names",
    "require_user_verification",
    "server_query",
    "spoiling_time_penalty",
    "starting_area",
    "steering",
    "unit_group",
    "verify_user_identity",
    "visibility",
];

fn count_keys(t: &str) -> usize {
    let mut n = 0usize;
    for k in KEYS {
        n += t.matches(&format!("\"{k}\"")).count();
    }
    n
}

fn has_signature(t: &str) -> bool {
    SIGNATURE.iter().any(|k| t.contains(&format!("\"{k}\"")))
}

/// `b` が Factorio 設定 JSON に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    // `"name"` は server-settings 専用で map/map-gen 系には無いため、
    // Factorio 固有キーの出現で署名する。1行圧縮 JSON でも数えられるよう
    // 行ではなく出現回数で判定する。
    t.trim_start().starts_with('{') && has_signature(t) && count_keys(t) >= 3
}

/// Factorio 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct FactorioSettings {
    /// 既知キーの出現数。
    pub keys: usize,
}

/// `b` を Factorio 設定として統計する。
pub fn parse(b: &[u8]) -> FactorioSettings {
    let t = std::str::from_utf8(b).unwrap_or("");
    FactorioSettings {
        keys: count_keys(t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"{\n\"name\": \"x\",\n\"max_players\": 0,\n\"visibility\": {}\n}\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"{\n\"name\": \"x\",\n\"max_players\": 0\n}\n"));
        assert!(!detect(b"key=value\nfoo=bar\n"));
        assert!(!detect(b"{\n\"key\": 1,\n\"other\": 2,\n\"name2\": 3\n}\n"));
    }

    #[test]
    fn detects_compact_one_line_json() {
        let b = b"{\"name\": \"x\", \"max_players\": 0, \"visibility\": {\"public\": true}}";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 4);
    }

    #[test]
    fn detects_map_settings_without_name() {
        let b = b"{\n\"pollution\": {},\n\"enemy_evolution\": {},\n\"enemy_expansion\": {}\n}\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_map_gen_settings_without_name() {
        let b = b"{\n\"seed\": 1,\n\"width\": 0,\n\"autoplace_controls\": {},\n\"starting_area\": 1.0\n}\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_generic_json_without_signature() {
        assert!(!detect(
            b"{\n\"name\": \"x\",\n\"description\": \"y\",\n\"tags\": []\n}\n"
        ));
    }

    #[test]
    fn not_json_rejected() {
        assert!(!detect(
            b"\"name\": \"x\"\n\"max_players\": 0\n\"visibility\": {}\n"
        ));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
