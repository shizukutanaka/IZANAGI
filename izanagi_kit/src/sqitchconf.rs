//! Sqitch `sqitch.conf` の検出・カウント。
//!
//! git-config 風 INI: `[core]`/`[engine "pg"]`/`[target "prod"]`/`[deploy]`/
//! `[add]` 等のセクション + `key = value` エントリ。データベース変更管理ツール
//! Sqitch の階層的設定ファイル。
//!
//! ```
//! let cfg = b"[core]\n\
//!             \tengine = pg\n\
//!             \ttop_dir = db\n\
//!             [engine \"pg\"]\n\
//!             \ttarget = db:pg:app\n\
//!             \tregistry = sqitch\n";
//! assert!(izanagi_kit::sqitchconf::detect(cfg));
//! let c = izanagi_kit::sqitchconf::parse(cfg).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.engine_keys, 3);
//! ```

/// ディレクトリ/ファイル系キー。
const DIR_KEYS: &[&str] = &[
    "deploy_dir",
    "revert_dir",
    "verify_dir",
    "reworked_dir",
    "reworked_deploy_dir",
    "reworked_revert_dir",
    "reworked_verify_dir",
    "top_dir",
    "extension",
    "plan_file",
    "template_dir",
    "etc_dir",
    "script_hash",
];

/// エンジン/DB 系キー。
const ENGINE_KEYS: &[&str] = &[
    "engine",
    "target",
    "uri",
    "registry",
    "client",
    "db_name",
    "host",
    "port",
    "username",
    "password",
    "driver",
    "connect_timeout",
    "query_timeout",
    "sqitch_schema",
    "lock_timeout",
    "warehouse",
    "role",
    "account",
    "service",
];

/// ユーザー/メタ系キー。
const USER_KEYS: &[&str] = &["name", "email", "accept"];

/// 変数/デプロイ制御系キー。
const VAR_KEYS: &[&str] = &[
    "set",
    "variables",
    "verify",
    "mode",
    "log_only",
    "deployed_at",
    "default_target",
    "default_client",
];

/// コマンド系セクション名。
const COMMAND_SECTIONS: &[&str] = &[
    "deploy", "revert", "verify", "rework", "add", "plan", "tag", "checkout", "init", "status",
    "log", "bundle", "rebase", "config", "engine", "target", "show", "help", "upgrade",
];

/// セクション行を `(主名, サブ名)` に分解する。
fn split_section(inner: &str) -> (&str, Option<&str>) {
    let inner = inner.trim();
    if let Some(pos) = inner.find(' ') {
        let (main, rest) = inner.split_at(pos);
        let sub = rest.trim().trim_matches('"');
        (main.trim(), Some(sub))
    } else {
        (inner, None)
    }
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// セクション総数。
    pub sections: usize,
    /// `[core]`/`[core "<eng>"]` セクション数。
    pub core_sections: usize,
    /// `[engine "<x>"]`/`[target "<x>"]` セクション数。
    pub target_sections: usize,
    /// `[deploy]`/`[add]`/`[verify]` 等コマンドセクション数。
    pub command_sections: usize,
    /// その他セクション数 (`[user]` 等)。
    pub other_sections: usize,
    /// `key = value` エントリ総数。
    pub entries: usize,
    /// ディレクトリ/ファイル系キー数。
    pub dir_keys: usize,
    /// エンジン/DB 系キー数。
    pub engine_keys: usize,
    /// ユーザー/メタ系キー数。
    pub user_keys: usize,
    /// 変数/制御系キー数。
    pub var_keys: usize,
    /// その他キー数。
    pub misc_keys: usize,
}

/// `b` が `sqitch.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.dir_keys + c.engine_keys + c.var_keys >= 2)
}

/// `b` を `sqitch.conf` として解析し、セクション・キー種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        core_sections: 0,
        target_sections: 0,
        command_sections: 0,
        other_sections: 0,
        entries: 0,
        dir_keys: 0,
        engine_keys: 0,
        user_keys: 0,
        var_keys: 0,
        misc_keys: 0,
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(inner) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            c.sections += 1;
            let (main, _sub) = split_section(inner);
            if main == "core" {
                c.core_sections += 1;
            } else if main == "engine" || main == "target" {
                c.target_sections += 1;
            } else if COMMAND_SECTIONS.contains(&main) {
                c.command_sections += 1;
            } else {
                c.other_sections += 1;
            }
            continue;
        }
        let Some((key, _val)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        c.entries += 1;
        if DIR_KEYS.contains(&key) {
            c.dir_keys += 1;
        } else if ENGINE_KEYS.contains(&key) {
            c.engine_keys += 1;
        } else if USER_KEYS.contains(&key) {
            c.user_keys += 1;
        } else if VAR_KEYS.contains(&key) {
            c.var_keys += 1;
        } else {
            c.misc_keys += 1;
        }
    }
    if c.dir_keys + c.engine_keys + c.var_keys >= 2 || (c.core_sections >= 1 && c.entries >= 2) {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# Sqitch config\n\
        [core]\n\
        \tengine = pg\n\
        \ttop_dir = db\n\
        \tdeploy_dir = db/deploy\n\
        \trevert_dir = db/revert\n\
        \tverify_dir = db/verify\n\
        \tplan_file = sqitch.plan\n\
        \textension = sql\n\
        [engine \"pg\"]\n\
        \ttarget = db:pg://user@localhost/app\n\
        \tregistry = sqitch\n\
        \tclient = /usr/bin/psql\n\
        [deploy]\n\
        \tverify = true\n\
        \tmode = change\n\
        [user]\n\
        \tname = Dev Team\n\
        \temail = dev@example.com\n";

    #[test]
    fn detects_sqitch() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.core_sections, 1);
        assert_eq!(c.target_sections, 1);
        assert_eq!(c.command_sections, 1);
        assert_eq!(c.entries, 14);
        assert_eq!(c.dir_keys, 6);
        assert_eq!(c.engine_keys, 4);
        assert_eq!(c.var_keys, 2);
        assert_eq!(c.user_keys, 2);
    }

    #[test]
    fn rejects_other_ini() {
        assert!(!detect(b"[foo]\nkey = value\n"));
        assert!(!detect(b"hello world\n"));
    }
}
