//! Internal text-scanning helpers shared by the config/manifest detector
//! modules. Not part of the public API — every item is `pub(crate)`, so the
//! pinned public surface is unchanged.
//!
//! These replace identical private copies that were duplicated across
//! hundreds of modules (`strip_bom` ×222, `value_of`/`kind_val` ×64 each,
//! `strip_xml_comments` ×43, `yaml_val` ×19). One definition, one fix point.

/// Drop a leading UTF-8 BOM (`U+FEFF`) if present.
pub(crate) fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Remove `<!-- ... -->` comments, tolerating an unterminated final comment.
pub(crate) fn strip_xml_comments(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The scalar after `:` on a `key: value` line, quotes stripped (`""` if none).
pub(crate) fn value_of(s: &str) -> &str {
    match s.find(':') {
        Some(i) => s[i + 1..].trim().trim_matches('"').trim_matches('\''),
        None => "",
    }
}

/// Value of `key` on a YAML `key: value` line (quoted or bare key, scalar
/// may be quoted).
pub(crate) fn yaml_val<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let l = line.trim_start_matches(['"', '\'']);
    let r = l
        .strip_prefix(key)?
        .trim_start_matches(['"', '\''])
        .trim_start();
    r.strip_prefix(':')
        .map(|v| v.trim().trim_matches('"').trim_matches('\''))
}

/// First `kind:` value in a Kubernetes-style manifest (comment lines and
/// non-`kind` lines skipped).
pub(crate) fn kind_val(t: &str) -> Option<String> {
    t.lines().find_map(|l| {
        let s = l.trim();
        if s.starts_with('#') || !s.starts_with("kind") {
            return None;
        }
        let v = value_of(s);
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    })
}
