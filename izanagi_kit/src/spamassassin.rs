//! SpamAssassin `local.cf`/`*.cf` census.
//!
//! `key value` settings (`required_score`, `report_safe`,
//! `rewrite_header`, `add_header`, `clear_headers`, `use_bayes`,
//! `bayes_auto_learn*`, `bayes_path`, `bayes_file_mode`,
//! `use_auto_whitelist`, `auto_whitelist_path`,
//! `trusted_networks`, `internal_networks`,
//! `allow_user_rules`, `dns_available`, `skip_rbl_checks`,
//! `use_dcc`, `use_pyzor`, `use_razor2`, `use_spf`,
//! `use_dmarc`, `use_arc`, `ok_locales`, `ok_languages`,
//! `normalize_charset`, `fold_headers`, `envelope_sender_header`,
//! `whitelist_from`, `whitelist_to`, `blacklist_from`,
//! `blacklist_to`, `whitelist_rcvd`, `def_whitelist_*`,
//! `def_blacklist_*`, `unwhitelist_*`, `unblacklist_*`,
//! `score SYMBOLIC n`, `header NAME =~ /re/`, `body`, `uri`,
//! `rawbody`, `full`, `meta`, `describe`, `tflags`, `priority`,
//! `required_hits`, `report_charset`, `report`,
//! `trusted_header_path`, `loadplugin`, `ifplugin`, `plugin`,
//! `user_scores_ldap_username`, `user_scores_dsn`,
//! `bayes_sql_dsn`, `whitelist_rcvd`).
//!
//! ```rust
//! let l = "required_score 5.0\nscore FOO_TEST 2.5\nheader X_BAD =~ /viagra/i\nwhitelist_from *@example.org\n";
//! let c = izanagi_kit::spamassassin::Spamassassin::parse(l.as_bytes()).unwrap();
//! assert_eq!(c.rules, 1);
//! assert_eq!(c.scores, 1);
//! ```

/// SpamAssassin cf census.
#[derive(Debug, Clone)]
pub struct Spamassassin {
    /// `header`/`body`/`uri`/`rawbody`/`full`/`meta` test definitions.
    pub rules: usize,
    /// `score TEST n` lines.
    pub scores: usize,
    /// whitelist/blacklist directives.
    pub lists: usize,
    /// Other `key value` settings.
    pub settings: usize,
    /// Recognised setting names.
    pub named: usize,
    /// `loadplugin`/`ifplugin`/`plugin` directives.
    pub plugins: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const RULE_HEADS: &[&str] = &[
    "header",
    "body",
    "uri",
    "rawbody",
    "full",
    "meta",
    "mimeheader",
    "eval",
    "describe",
    "tflags",
    "priority",
];

const LIST_HEADS: &[&str] = &[
    "whitelist_from",
    "whitelist_to",
    "whitelist_rcvd",
    "whitelist_allows_relays",
    "blacklist_from",
    "blacklist_to",
    "blacklist_rcvd",
    "def_whitelist_from_rcvd",
    "def_blacklist_from_rcvd",
    "unwhitelist_from",
    "unblacklist_from",
    "unwhitelist_from_rcvd",
    "enlist_uri_host",
    "delist_uri_host",
    "whitelist_auth",
    "whitelist_from_rcvd",
];

const KEYS: &[&str] = &[
    "required_score",
    "required_hits",
    "report_safe",
    "report_charset",
    "report",
    "rewrite_header",
    "add_header",
    "remove_header",
    "clear_headers",
    "use_bayes",
    "bayes_auto_learn",
    "bayes_auto_learn_threshold_nonspam",
    "bayes_auto_learn_threshold_spam",
    "bayes_path",
    "bayes_file_mode",
    "bayes_ignore_headers",
    "bayes_sql_dsn",
    "bayes_expiry_max_db_size",
    "use_auto_whitelist",
    "auto_whitelist_path",
    "auto_whitelist_file_mode",
    "trusted_networks",
    "internal_networks",
    "trusted_header_path",
    "allow_user_rules",
    "dns_available",
    "skip_rbl_checks",
    "skip_uribl_checks",
    "use_dcc",
    "dcc_timeout",
    "dcc_body_max",
    "dcc_fuz1_max",
    "dcc_fuz2_max",
    "use_pyzor",
    "pyzor_timeout",
    "pyzor_max",
    "use_razor2",
    "razor_timeout",
    "use_spf",
    "use_dmarc",
    "use_arc",
    "ok_locales",
    "ok_languages",
    "normalize_charset",
    "fold_headers",
    "envelope_sender_header",
    "user_scores_ldap_username",
    "user_scores_dsn",
    "user_scores_sql_username",
    "user_scores_sql_password",
    "user_scores_sql_field",
    "username",
    "timelog",
    "use_txrep",
    "txrep_factory",
    "bayes_min_ham_num",
    "bayes_min_spam_num",
    "bayes_learn_to_journal",
    "bayes_use_hapaxes",
    "bayes_journal_max_size",
    "bayes_expiry_max_db_size",
    "bayes_auto_expire",
    "bayes_store_module",
    "awl_sql_dsn",
    "awl_sql_username",
    "awl_sql_table",
    "rbl_timeout",
    "razor_config",
    "dcc_options",
    "pyzor_options",
    "spamd_allow_rules",
    "channel_updater_binary",
    "channel_timeout",
    "gpg_binary",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect SpamAssassin cf content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        let head = s.split_whitespace().next().unwrap_or("");
        if head == "score"
            || RULE_HEADS.contains(&head)
            || LIST_HEADS.contains(&head)
            || KEYS.contains(&head)
        {
            hits += 1;
        }
    }
    hits >= 2
}

impl Spamassassin {
    /// Census a SpamAssassin cf buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            rules: 0,
            scores: 0,
            lists: 0,
            settings: 0,
            named: 0,
            plugins: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if head == "score" {
                c.scores += 1;
                continue;
            }
            if RULE_HEADS.contains(&head) {
                c.rules += 1;
                continue;
            }
            if LIST_HEADS.contains(&head) {
                c.lists += 1;
                continue;
            }
            if head == "loadplugin" || head == "ifplugin" || head == "plugin" || head == "endif" {
                c.plugins += 1;
                continue;
            }
            c.settings += 1;
            if KEYS.contains(&head) {
                c.named += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cf() {
        let b = b"required_score 5.0\nscore FOO 2.0\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_cf() {
        let b = concat!(
            "# spamassassin local.cf\n",
            "required_score 5.0\n",
            "report_safe 1\n",
            "rewrite_header Subject [SPAM]\n",
            "use_bayes 1\n",
            "bayes_auto_learn 1\n",
            "trusted_networks 192.168.\n",
            "skip_rbl_checks 0\n",
            "score FOO_TEST 2.5\n",
            "score RCVD_IN_MSPIKE -1.0\n",
            "header X_BAD_SUBJ Subject =~ /free money/i\n",
            "body BAD_BODY /viagra/i\n",
            "uri BAD_URI /cheap-pills/\n",
            "meta BIG_COMBO (X_BAD_SUBJ + BAD_BODY > 1)\n",
            "describe X_BAD_SUBJ Spammy subject\n",
            "whitelist_from *@example.org\n",
            "blacklist_from spammer@bad.tld\n",
            "loadplugin Mail::SpamAssassin::Plugin::DKIM\n",
        );
        let c = Spamassassin::parse(b.as_bytes()).unwrap();
        assert_eq!(c.rules, 5);
        assert_eq!(c.scores, 2);
        assert_eq!(c.lists, 2);
        assert_eq!(c.plugins, 1);
        assert!(c.settings >= 6);
        assert!(c.named >= 6);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
