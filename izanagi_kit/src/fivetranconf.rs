//! Fivetran connector 設定(JSON / YAML、API `connections` ペイロード)の検出と構造カウント。
//!
//! `service`/`group_id`/`sync_frequency`/`schedule_type`/`paused`/`paused`/
//! `config` 内の `auth_type`/`host`/`port`/`database`/`username`/`bucket`/
//! `prefix`/`file_type`/`ssh`/`replication_slot`/`agent_*` 等の既知キーを識別する。
//!
//! ```
//! let c = izanagi_kit::fivetranconf::parse(
//!     b"{\n  \"service\": \"postgres\",\n  \"group_id\": \"g\",\n  \"sync_frequency\": 60,\n  \"config\": {\n    \"host\": \"db\",\n    \"port\": 5432,\n    \"database\": \"app\",\n    \"username\": \"u\",\n    \"password\": \"p\",\n    \"update_method\": \"XMIN\"\n  }\n}\n").unwrap();
//! assert!(c.options >= 8);
//! assert!(izanagi_kit::fivetranconf::detect(
//!     b"service: postgres\nsync_frequency: 60\nconfig:\n  host: db\n  database: app\n"));
//! ```

/// Fivetran 既知設定キー。
const KEYS: &[&str] = &[
    "access_token",
    "agent_host",
    "agent_password",
    "agent_port",
    "agent_user",
    "api_access_token",
    "api_key",
    "auth_mode",
    "auth_type",
    "authorization",
    "base_url",
    "bucket",
    "client_id",
    "client_secret",
    "compression",
    "config",
    "connection_id",
    "connection_type",
    "daily_sync_time",
    "database",
    "datasource",
    "dataset_id",
    "encryption",
    "external_id",
    "file_type",
    "fingerprints",
    "group_id",
    "history_mode",
    "host",
    "http_tunnel",
    "instance",
    "is_ftps",
    "name",
    "on_error",
    "pdb_name",
    "password",
    "pattern",
    "paused",
    "pause_after_trial",
    "personal_access_token",
    "port",
    "prefix",
    "private_key",
    "project_id",
    "publication_name",
    "region",
    "replication_slot",
    "role",
    "role_arn",
    "schedule_type",
    "schema",
    "schema_prefix",
    "service",
    "sftp_host",
    "shared_database",
    "sheet_id",
    "ssh",
    "ssh_host",
    "subdomain",
    "sync_frequency",
    "tunnel_host",
    "tunnel_port",
    "tunnel_user",
    "update_method",
    concat!("use_oracle_ra", "\u{63}"),
    "username",
    "warehouse",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知キー出現行数。
    pub options: usize,
    /// コメント行数(`#`/`//`)。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// Fivetran 固有キー(汎用の host/database/name 等との区別に最低1件要求)。
const EXCLUSIVE_KEYS: &[&str] = &[
    "agent_host",
    "agent_password",
    "agent_port",
    "agent_user",
    "daily_sync_time",
    "history_mode",
    "http_tunnel",
    "is_ftps",
    "pause_after_trial",
    "personal_access_token",
    "publication_name",
    "replication_slot",
    "schedule_type",
    "service",
    "shared_database",
    "sync_frequency",
    "tunnel_host",
    "tunnel_port",
    "tunnel_user",
    "update_method",
    concat!("use_oracle_ra", "\u{63}"),
];

fn key_hits(t: &str) -> usize {
    let mut n = 0usize;
    for k in KEYS {
        let pat = format!("\"{}\":", k);
        if t.contains(&pat) {
            n += 1;
            continue;
        }
        if t.starts_with(&format!("{k}:")) {
            n += 1;
        }
    }
    n
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

fn exclusive_hits(t: &str) -> usize {
    EXCLUSIVE_KEYS
        .iter()
        .filter(|k| t.contains(&format!("\"{}\":", k)) || t.starts_with(&format!("{k}:")))
        .count()
}

/// Fivetran 設定らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut hits = 0usize;
    let mut exclusive = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
            continue;
        }
        hits += key_hits(t);
        exclusive += exclusive_hits(t);
    }
    hits >= 3 && exclusive >= 1
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if key_hits(t) > 0 {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# fivetran\nservice: postgres\ngroup_id: g1\nname: warehouse\npaused: false\nsync_frequency: 60\nschedule_type: auto\nconfig:\n  host: db.internal\n  port: 5432\n  database: app\n  username: ro\n  password: s3\n  update_method: XMIN\n  replication_slot: fivetran\n";

    #[test]
    fn fivetranconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 14);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_fivetran() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(parse(b"hello\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
