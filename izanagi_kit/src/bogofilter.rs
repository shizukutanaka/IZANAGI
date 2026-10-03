//! Bogofilter 設定(bogofilter.cf)の検出と構造カウント。
//!
//! `key = value` 行を既知オプション・`!`/`#` コメントに分類する。
//!
//! ```
//! let c = izanagi_kit::bogofilter::parse(
//!     b"block_on_subnets = yes\nspam_header_name = X-Bogosity\n").unwrap();
//! assert_eq!(c.options, 2);
//! assert!(izanagi_kit::bogofilter::detect(b"stats_in_header = yes\nblock_on_subnets = yes\n"));
//! ```

/// 既知 bogofilter オプション。
const KEYS: &[&str] = &[
    "algorithm",
    "block_on_subnets",
    "bogofilter_dir",
    "charset_default",
    "db_cachesize",
    "db_checkpoint",
    "db_log_autoremove",
    "db_private",
    "db_prune_pagesize",
    "db_recover_harder",
    "db_txn_durable",
    "encoding",
    "format_header",
    "format_footer",
    "format_spamicity",
    "ham_header_format",
    "header_degen",
    "header_format",
    "header_line_markup",
    "ignore_case",
    "min_dev",
    "ns_esf",
    "replace_nonascii_characters",
    "robobs",
    "robx",
    "sp_esf",
    "spam_header_name",
    "spam_header_format",
    "spam_subject_tag",
    "spamicity_formats",
    "spamicity_tags",
    "stats_in_header",
    "terse",
    "terse_format",
    "thresh_index",
    "thresh_rtable",
    "thresh_update",
    "timestamp",
    "timestamp_tokens",
    "unsure_header_format",
    "unsure_subject_tag",
    "updatestamp_min_delay",
    "wordlist",
    "user_config_file",
];

/// Bogofilter 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知オプション行。
    pub options: usize,
    /// `#`/`!` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が bogofilter.cf かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| {
            let t = l.trim();
            t.find('=')
                .map(|p| KEYS.contains(&t[..p].trim()))
                .unwrap_or(false)
        })
        .count()
        >= 2
}

/// bogofilter.cf の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
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
        if t.starts_with('#') || t.starts_with('!') {
            c.comments += 1;
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        if KEYS.contains(&t[..pos].trim()) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# bogofilter.cf\nblock_on_subnets = yes\ncharset_default = us-ascii\nreplace_nonascii_characters = yes\nstats_in_header = yes\nthresh_update = 0.0\nspam_header_name = X-Bogosity\nspam_subject_tag = ***SPAM***\nunsure_subject_tag = ???\n! inline comment\nwordlist = r,user,bogofilter,list.db,2\nbogus_option = 1\n";

    #[test]
    fn bogofilter() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 9);
        assert_eq!(c.comments, 2);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_bogofilter() {
        assert!(!detect(b"key = value\nother = 1\n"));
        assert!(!detect(b"hello\n"));
    }
}
