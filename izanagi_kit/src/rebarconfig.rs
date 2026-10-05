//! Erlang `rebar.config` の検出と構造カウント。
//!
//! `{key, value}.` Erlang ターム行(`deps`/`erl_opts`/`profiles`/`relx`/
//! `plugins`/`ct_opts`/`edoc_opts`/`dialyzer`/`xref_checks`/`provider_hooks` 等)を
//! 識別する。コメントは `%`。
//!
//! ```
//! let c = izanagi_kit::rebarconfig::parse(
//!     b"{erl_opts, [debug_info]}.\n{deps, [{cowboy, \"2.9\"}]}.\n{profiles, [{test, [{deps, []}]}]}.\n").unwrap();
//! assert_eq!(c.entries, 3);
//! assert!(izanagi_kit::rebarconfig::detect(
//!     b"{deps, []}.\n{relx, []}.\n"));
//! ```

/// 既知タームキー。
const KEYS: &[&str] = &[
    "alias",
    "checkouts_dir",
    "clean_files",
    "compiler_options_format",
    "cover_enabled",
    "cover_export_enabled",
    "cover_print_enabled",
    "ct_opts",
    "deps",
    "deps_dir",
    "deps_names",
    "dialyzer",
    "dialyzer_plt_apps",
    "edoc_opts",
    "erl_first_files",
    "erl_opts",
    "escript_comment",
    "escript_emu_args",
    "escript_incl_apps",
    "escript_incl_extra",
    "escript_main_app",
    "escript_name",
    "eunit_first_files",
    "eunit_opts",
    "hex_core_opts",
    "lib_dirs",
    "minimum_otp_vsn",
    "mib_first_files",
    "overrides",
    "plugins",
    "post_hooks",
    "pre_hooks",
    "profiles",
    "project_app_dirs",
    "project_plugins",
    "provider_hooks",
    "relx",
    "shell",
    "sub_dirs",
    "validate_app_modules",
    "xrl_opts",
    "xref_checks",
    "xref_queries",
    "yrl_opts",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `{key, ...}` ターム開始行数(既知キー)。
    pub entries: usize,
    /// `%` コメント行数。
    pub comments: usize,
    /// その他の行数(ターム継続行等)。
    pub misc: usize,
}

fn term_key(t: &str) -> Option<&str> {
    if !t.starts_with('{') {
        return None;
    }
    let inner = t[1..].trim_start();
    let end = inner.find(|c: char| c == ',' || c.is_whitespace())?;
    let k = &inner[..end];
    KEYS.contains(&k).then_some(k)
}

/// `rebar.config` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('%') {
            continue;
        }
        if term_key(t).is_some() {
            hits += 1;
            if hits >= 2 {
                return true;
            }
        }
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let mut c = Counts {
        entries: 0,
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
        if term_key(t).is_some() {
            c.entries += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"% rebar.config\n{erl_opts, [debug_info, warn_export_all]}.\n{deps, [\n  {cowboy, \"2.9.0\"},\n  {jsx, \"3.1.0\"}\n]}.\n{relx, [{release, {app, \"1.0\"}, [app]}]}.\n{profiles, [\n  {test, [{deps, [{meck, \"0.9\"}]}]}\n]}.\n{ct_opts, [{sys_config, \"test.cfg\"}]}.\n{dialyzer, [{warnings, [unmatched_returns]}]}.\n";

    #[test]
    fn rebarconfig() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 6);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 5);
    }

    #[test]
    fn not_rebarconfig() {
        assert!(!detect(b"{a, 1}.\n{b, 2}.\n"));
        assert!(parse(b"text\n").is_none());
    }
}
