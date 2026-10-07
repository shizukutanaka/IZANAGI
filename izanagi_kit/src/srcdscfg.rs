//! srcds `server.cfg`(Source Engine: CS:S/CS:GO/TF2/L4D2 等)検出モジュール。
//!
//! srcds の設定は `key "value"` 形式の convar 並びで、`hostname`/
//! `rcon_password`/`sv_password`/`sv_region`/`sv_lan`/`sv_cheats`/
//! `sv_consistency`/`sv_contact`/`sv_downloadurl`/`sv_allowupload`/
//! `sv_allowdownload`/`sv_alltalk`/`sv_voiceenable`/`sv_maxrate`/
//! `sv_minrate`/`sv_maxupdaterate`/`sv_minupdaterate`/`sv_maxcmdrate`/
//! `sv_mincmdrate`/`sv_pure`/`sv_pausable`/`sv_tags`/`sv_log*`/
//! `mp_timelimit`/`mp_maxrounds`/`mp_winlimit`/`mp_fraglimit`/
//! `mp_friendlyfire`/`mp_autoteambalance`/`mp_limitteams`/
//! `mp_freezetime`/`mp_roundtime`/`mp_buytime`/`mp_startmoney`/
//! `mp_c4timer`/`mp_teamplay`/`mp_footsteps`/`mp_flashlight`/
//! `mp_autocrosshair`/`mp_forcerespawn`/`mp_logmessages`/`mp_match*`/
//! `mapcyclefile`/`motdfile`/`map`/`exec`/`heartbeat`/`log`/`rcon`/
//! `tv_enable`/`tv_name`/`tv_port`/`tv_password`/`tv_maxclients` 等で
//! 構成される。
//!
//! ```
//! let b = b"hostname \"My Server\"\n\
//!           rcon_password \"secret\"\n\
//!           sv_maxrate 0\n\
//!           sv_minrate 100000\n\
//!           mp_timelimit 30\n\
//!           mp_friendlyfire 0\n";
//! let c = izanagi_kit::srcdscfg::parse(b);
//! assert!(izanagi_kit::srcdscfg::detect(b));
//! assert_eq!(c.cvars, 6);
//! ```

const KEYS: &[&str] = &[
    "bot_quota",
    "bot_quota_mode",
    "exec",
    "heartbeat",
    "hostname",
    "log",
    "map",
    "mapcyclefile",
    "motdfile",
    "rcon_password",
    "sv_allowdownload",
    "sv_allowupload",
    "sv_alltalk",
    "sv_cheats",
    "sv_consistency",
    "sv_contact",
    "sv_deadtalk",
    "sv_disablefreezecam",
    "sv_downloadurl",
    "sv_enablevoice",
    "sv_gravity",
    "sv_hibernate_when_empty",
    "sv_lan",
    "sv_log",
    "sv_log_onefile",
    "sv_logbans",
    "sv_logblocks",
    "sv_logdownloadlist",
    "sv_logdownloads",
    "sv_logecho",
    "sv_logfile",
    "sv_logflush",
    "sv_logsdir",
    "sv_max_queries_sec",
    "sv_max_queries_sec_global",
    "sv_max_queries_window",
    "sv_maxcmdrate",
    "sv_maxrate",
    "sv_maxreplay",
    "sv_maxupdaterate",
    "sv_mincmdrate",
    "sv_minrate",
    "sv_minupdaterate",
    "sv_noclipaccelerate",
    "sv_pausable",
    "sv_password",
    "sv_pure",
    "sv_pvp",
    "sv_rcon_banpenalty",
    "sv_rcon_log",
    "sv_rcon_maxfailures",
    "sv_rcon_minfailures",
    "sv_rcon_minfailuretime",
    "sv_region",
    "sv_registration_message",
    "sv_registration_successful",
    "sv_search_key",
    "sv_showimpacts",
    "sv_showimpacts_time",
    "sv_showladder",
    "sv_skyname",
    "sv_stats",
    "sv_steamgroup",
    "sv_tags",
    "sv_timeout",
    "sv_turbophysics",
    "sv_voiceenable",
    "sys_error_log",
    "tv_allow_camera_man",
    "tv_allow_static_shots",
    "tv_autorecord",
    "tv_autoretry",
    "tv_chatgroupsize",
    "tv_chattimelimit",
    "tv_clients",
    "tv_delay",
    "tv_deltacache",
    "tv_dispatchmode",
    "tv_enable",
    "tv_maxclients",
    "tv_maxrate",
    "tv_name",
    "tv_nochat",
    "tv_overridemaster",
    "tv_password",
    "tv_port",
    "tv_relaypassword",
    "tv_relayvoice",
    "tv_snapshotrate",
    "tv_timeout",
    "tv_title",
    "tv_transmitall",
    "writeid",
    "writeip",
];

fn is_cvar(t: &str) -> bool {
    let mut it = t.split_whitespace();
    let w = it.next().unwrap_or("");
    // 設定行は `cvar value` の2要素。単独の cvar 名は問い合わせであり
    // 設定ではないので除外する。
    if it.next().is_none() {
        return false;
    }
    KEYS.contains(&w)
        || w.starts_with("sv_")
        || w.starts_with("mp_")
        || w.starts_with("tv_")
        || w.starts_with("bot_")
        || w.starts_with("cs_")
        || w.starts_with("sm_")
}

/// `b` が srcds server.cfg に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut cvars = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with("//") || tr.starts_with('#') {
            continue;
        }
        if is_cvar(tr) {
            cvars += 1;
        }
    }
    cvars >= 4
}

/// srcds server.cfg の統計。
#[derive(Debug, Default, Clone)]
pub struct SrcdsCfg {
    /// convar 行数。
    pub cvars: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を srcds server.cfg として統計する。
pub fn parse(b: &[u8]) -> SrcdsCfg {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = SrcdsCfg::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with("//") || tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_cvar(tr) {
            c.cvars += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"hostname \"x\"\nsv_maxrate 0\nmp_timelimit 30\nmp_friendlyfire 0\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.cvars, 4);
    }

    #[test]
    fn prefix_cvars() {
        let b = b"sv_x 1\nmp_y 2\ntv_z 3\nbot_w 4\n";
        assert!(detect(b));
    }

    #[test]
    fn bare_cvar_names_do_not_count() {
        let b = b"sv_maxrate\nsv_minrate\nmp_timelimit\nmp_friendlyfire\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.cvars, 0);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"hostname \"x\"\nsv_maxrate 0\nmp_timelimit 30\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.cvars, 0);
    }
}
