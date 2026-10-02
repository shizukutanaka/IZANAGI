//! Rspamd `rspamd.conf` / `local.d/*.conf`(UCL/HCL 風)の検出と構造カウント。
//!
//! `worker`/`logging`/`options`/`modules`/`metric`/`classifier` 等の既知セクション +
//! `key = value;` または `key { ... }` ブロック形式を分類する。
//!
//! ```
//! let c = izanagi_kit::rspamdconf::parse(
//!     b"options {\n  filters = \"spf,dkim,dmarc\";\n}\nworker {\n  type = \"normal\";\n}\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert!(izanagi_kit::rspamdconf::detect(b"logging {\n  level = \"warning\";\n}\n"));
//! ```

/// 既知トップ/サブセクション名。
const SECTIONS: &[&str] = &[
    "actions",
    "antivirus",
    "arc",
    "asn",
    "bayes_expiry",
    "chartable",
    "clickhouse",
    "composite",
    "composites",
    "conf",
    "controller",
    "dcc",
    "dkim",
    "dkim_signing",
    "dmarc",
    "dynamic_conf",
    "elastic",
    "emails",
    "external_services",
    "fann_redis",
    "fuzzy_check",
    "fuzzy_storage",
    "greylist",
    "groups",
    "history_redis",
    "hfilter",
    "ip_score",
    "lang_detect",
    "logging",
    "lua",
    "maillist",
    "map",
    "metadata_exporter",
    "metric",
    "milter_headers",
    "mime_types",
    "modules",
    "monitoring",
    "multimap",
    "mx_check",
    "neural",
    "normalization",
    "options",
    "outbound",
    "password",
    "phishing",
    "plugin",
    "plugins",
    "ratelimit",
    "regexp",
    "reputation",
    "rspamd_proxy",
    "rbl",
    "rfc822",
    "selector",
    "settings",
    "sigh",
    "spamassassin",
    "spf",
    "statistics",
    "subject_rewrites",
    "surbl",
    "symbols",
    "trie",
    "url_reputation",
    "user",
    "whitelist",
    "worker",
];
/// 既知キー(UCL 代入形式)。
const KEYS: &[&str] = &[
    "actions",
    "allow_cdb",
    "backend",
    "bind_socket",
    "cache_expire",
    "check_editor",
    "classify_headers",
    "code",
    "condition",
    "control_socket",
    "count",
    "description",
    "disabled",
    "dns",
    "elapsed",
    "enable_password",
    "enabled",
    "expire",
    "expression",
    "extra_keys",
    "filename",
    "filters",
    "flag",
    "from",
    "group",
    "grow_factor",
    "hash",
    "history_rows",
    "hostname",
    "hs",
    "ignore_map",
    "inflight",
    "ip",
    "key",
    "learn_condition",
    "level",
    "log_format",
    "log_re_cache",
    "map",
    "max_associations",
    "max_ttl",
    "message",
    "min_bytes",
    "minimize_output",
    "min_length",
    "modules_disabled",
    "monitoring_interval",
    "name",
    "nrows",
    "nsfw",
    "one_shot",
    "password",
    "path",
    "pattern",
    "priority",
    "private_key",
    "pubkey",
    "rate",
    "read_only",
    "reject_score",
    "relay",
    "rewrite",
    "score",
    "secure_ip",
    "servers",
    "sign_local",
    "sign_networks",
    "symbol_prefix",
    "sysctl",
    "tempdir",
    "threshold",
    "timeout",
    "type",
    "upstream",
    "url",
    "use_own",
    "use_redis",
    "use_static",
    "watch_interval",
    "weight",
    "workers",
    "whitelisted",
    "nameserver",
];

/// Rspamd 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `name {`/`worker {...}` ブロック開始。
    pub sections: usize,
    /// `key = value;` 既知代入。
    pub options: usize,
    /// `#`/`//` コメント行。
    pub comments: usize,
    /// `}` 閉じブロック。
    pub closes: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行が `name {` / `name "arg" {` ブロック開始かどうか。先頭トークンを返す。
fn block_name(t: &str) -> Option<&str> {
    let pos = t.find('{')?;
    let name = t[..pos].trim();
    if name.is_empty() {
        return None;
    }
    let mut it = name.split_whitespace();
    let first = it.next()?;
    // `worker "normal" {` のように後続は引用符引数のみ許可。
    if it.all(|w| w.starts_with('"')) {
        Some(first)
    } else {
        None
    }
}

/// b が rspamd.conf かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut secs = 0;
    let mut opts = 0;
    for line in text.lines() {
        let t = line.trim();
        if let Some(n) = block_name(t) {
            if SECTIONS.contains(&n) || n == "worker" {
                secs += 1;
            }
        } else if t.ends_with(';')
            && t[..t.len() - 1]
                .find('=')
                .is_some_and(|p| KEYS.contains(&t[..p].trim()))
        {
            opts += 1;
        }
    }
    secs + opts >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        closes: 0,
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
        if t == "}" || t == "};" || t == "}," || t.starts_with("}.") {
            c.closes += 1;
            continue;
        }
        if let Some(name) = block_name(t) {
            if SECTIONS.contains(&name) || !name.contains(char::is_whitespace) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if t.ends_with(';') || t.ends_with(',') {
            let body = &t[..t.len() - 1];
            if body
                .find('=')
                .is_some_and(|p| KEYS.contains(&body[..p].trim()))
            {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# rspamd\noptions {\n  filters = \"spf,dkim,dmarc\";\n  dns {\n    nameserver = [\"127.0.0.1\"];\n  }\n}\nlogging {\n  type = \"file\";\n  filename = \"/var/log/rspamd/rspamd.log\";\n  level = \"warning\";\n}\nworker \"normal\" {\n  bind_socket = \"localhost:11333\";\n}\nmodules {\n  path = \"${PLUGINSDIR}\";\n}\n";

    #[test]
    fn rspamdconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.options, 7);
        assert_eq!(c.closes, 5);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_rspamd() {
        assert!(!detect(b"[section]\nkey = value\n"));
        assert!(!detect(b"hello\n"));
    }
}
