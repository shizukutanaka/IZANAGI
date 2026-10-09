//! slrn ニュースリーダ設定(.slrnrc)の検出と構造カウント。
//!
//! `set`/`unset` 変数代入・`setkey`/`undefkey` キー・`color`/`mono` 配色・
//! `group`/`server`/`newsgroup` ディレクティブ・`%%` コメントを分類する。
//!
//! ```
//! let c = izanagi_kit::slrnconf::parse(
//!     b"set username \"joe\"\nset hostname \"host\"\ncolor article blue black\n").unwrap();
//! assert_eq!(c.sets, 2);
//! assert_eq!(c.colors, 1);
//! assert!(izanagi_kit::slrnconf::detect(b"set sorting_method 9\nset display_score 0\n"));
//! ```

use crate::textutil::strip_bom;
/// `set`/`unset` に使える既知変数。
const VARS: &[&str] = &[
    "abort_unmodified_edits",
    "art_help_line",
    "art_status_line",
    "art_viewed_status",
    "author_display",
    "auto_mark_article_as_read",
    "auto_reconnect",
    "beep",
    "blinker",
    "broken_refs",
    "cancel_warn",
    "cc_self",
    "charset",
    "check_new_groups",
    "color_by_score",
    "color_date",
    "confirm_print_job",
    "confirm_save_all",
    "custom_headers",
    "custom_headers_order",
    "custom_postponed_headers",
    "custom_rejections",
    "custom_sort_order",
    "date_format",
    "decode_directory",
    "decode_directory_mode",
    "descend_order",
    "display_cursor_bar",
    "display_score",
    "drag_bars",
    "draw_graphic_char",
    "editor_printer",
    "editor_shell_command",
    "emphasis_mask",
    "fetch_numbers",
    "followup",
    "followup_strip_signature",
    "followup_to",
    "generate_message_id",
    "getlimit",
    "group_descriptions",
    "group_help_line",
    "group_status_line",
    "group_title_format",
    "group_update_hook",
    "header_display_line",
    "header_status_line",
    "header_title_format",
    "hide_pgp_signature",
    "hide_signature",
    "hide_quotes",
    "highlight_unread_subjects",
    "highlight_urls",
    "hostname",
    "iconify",
    "ignore_signature",
    "ispell_display_mode",
    "keymap",
    "lines",
    "list_max",
    "mark_read_until_date",
    "meta_mail",
    "mime_charset",
    "mono",
    "mouse",
    "new_subject_breaks_threads",
    "newsubject_keywords",
    "nonascii_browser",
    "obscure_keyword",
    "post_editor_command",
    "post_file",
    "post_object",
    "posting_host",
    "prefer_head",
    "print_cmd",
    "printer_name",
    "query_cut_group_scores",
    "query_cut_scores",
    "query_min_group_score",
    "query_min_high_score",
    "read_active",
    "refetch_speed",
    "reply_string",
    "replyto",
    "save_directory",
    "save_posts",
    "save_replies",
    "score",
    "scorefile",
    "scroll_by_page",
    "sendmail_command",
    "server",
    "setkey",
    "show_article",
    "show_description",
    "show_thread_subject",
    "signature",
    "signature_file",
    "simkey_sendmap",
    "slant",
    "smiley_patterns",
    "sorting_method",
    "spoof",
    "spoof_comment",
    "spool_directory",
    "spool_inn_root",
    "spool_root",
    "subject_compare_limit",
    "subject_keyword",
    "subscriptions",
    "top_status_line",
    "uncollapse_threads",
    "unread_color",
    "use_netrc",
    "use_tmpdir",
    "use_tilde",
    "user_posts_update",
    "username",
    "verbose",
    "wrap_method",
    "wrap_strikes",
    "write_newsrc_flags",
    "xeditor",
    "xorg",
    "org",
];
/// トップレベル命令語。
const DIREC: &[&str] = &[
    "autobaud",
    "compatible",
    "define",
    "group",
    "if",
    "ignore",
    "include",
    "interpret",
    "newsgroup",
    "server",
    "spool",
    "unread",
    "visible",
    "hook",
];
/// `setkey`/`undefkey`/`setcolor`/`color`/`mono` 以外のキー・配色語。
const KEYS: &[&str] = &["setkey", "undefkey"];

/// slrnrc 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `set`/`unset` 行。
    pub sets: usize,
    /// `setkey`/`undefkey` 行。
    pub keys: usize,
    /// `color`/`setcolor`/`mono` 行。
    pub colors: usize,
    /// `group`/`server`/`newsgroup` 等のディレクティブ行。
    pub directives: usize,
    /// `%%`/`%` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が .slrnrc かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut sig = 0;
    for line in text.lines() {
        let t = line.trim();
        let h = t.split_whitespace().next().unwrap_or("");
        if (h == "set" || h == "unset")
            && t.split_whitespace()
                .nth(1)
                .is_some_and(|v| VARS.contains(&v))
        {
            sig += 2;
        } else if KEYS.contains(&h)
            || h == "color"
            || h == "setcolor"
            || h == "mono"
            || t.starts_with("%%")
        {
            sig += 1;
        }
    }
    sig >= 2
}

/// .slrnrc の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sets: 0,
        keys: 0,
        colors: 0,
        directives: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('%') {
            c.comments += 1;
            continue;
        }
        let h = t.split_whitespace().next().unwrap_or("");
        if h == "set" || h == "unset" {
            c.sets += 1;
        } else if KEYS.contains(&h) {
            c.keys += 1;
        } else if h == "color" || h == "setcolor" || h == "mono" {
            c.colors += 1;
        } else if DIREC.contains(&h) {
            c.directives += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.sets + c.keys + c.colors + c.directives >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"%% slrnrc\nset username \"joe\"\nset hostname \"example.com\"\nset signature \".signature\"\nset sorting_method 9\nset display_score 1\nset color_by_score 1\nunset use_netrc\n\n%% keys\nsetkey article next \"n\"\nsetkey article prev \"p\"\n\ncolor article blue black\ncolor headername cyan black\nmono art_subject bold\n\ngroup \"comp.lang.rust\" 1\nserver \"news.example.org\" \".jnewsrc\"\n\nrandom text\n";

    #[test]
    fn slrnconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sets, 7);
        assert_eq!(c.keys, 2);
        assert_eq!(c.colors, 3);
        assert_eq!(c.directives, 2);
        assert_eq!(c.comments, 2);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_slrn() {
        assert!(!detect(b"key = value\n[section]\n"));
        assert!(!detect(b"hello world\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
