//! SELinux `.te` (type enforcement) 検出モジュール。
//!
//! .te ポリシーファイルは `policy_module(name, ver)` 宣言、
//! `require { ... }`/`gen_require(...)`/`optional_policy(...)`/
//! `tunable_policy(...)`/`ifdef(...)`/`ifndef(...)` ブロック、
//! `type`/`typealias`/`attribute`/`attribute_role`/`class`/
//! `bool`/`role`/`sensitivity`/`category`/`dominance`/`level`/
//! `user`/`sid`/`range_transition` 宣言、`allow`/`auditallow`/
//! `dontaudit`/`neverallow`/`type_transition`/`type_change`/
//! `type_member` ルール、`allow src tgt:class { perm1 perm2 };`
//! 形式のパーミッション集合で構成される。
//!
//! ```
//! let b = b"policy_module(myapp, 1.0.0)\n\
//!           require {\n    type httpd_t;\n    class file read;\n}\n\
//!           type myapp_t;\n\
//!           type myapp_exec_t;\n\
//!           allow myapp_t httpd_t:file read;\n\
//!           type_transition myapp_t httpd_t:file myapp_t;\n";
//! let c = izanagi_kit::selinuxte::parse(b);
//! assert!(izanagi_kit::selinuxte::detect(b));
//! assert!(c.rules >= 2);
//! ```

fn is_anchor(t: &str) -> bool {
    t.starts_with("policy_module(")
        || t.starts_with("module ")
        || t.starts_with("gen_require(")
        || t.starts_with("optional_policy(")
        || t.starts_with("tunable_policy(")
}

fn is_rule(t: &str) -> bool {
    let w = t.split_whitespace().next().unwrap_or("");
    matches!(
        w,
        "allow"
            | "auditallow"
            | "dontaudit"
            | "neverallow"
            | "type_transition"
            | "type_change"
            | "type_member"
            | "role_transition"
            | "range_transition"
    )
}

fn is_decl(t: &str) -> bool {
    let w = t.split_whitespace().next().unwrap_or("");
    matches!(
        w,
        "type"
            | "typealias"
            | "attribute"
            | "attribute_role"
            | "class"
            | "common"
            | "bool"
            | "role"
            | "sensitivity"
            | "category"
            | "dominance"
            | "level"
            | "user"
            | "sid"
            | "require"
            | "interface"
            | "template"
            | "permissive"
            | "alias"
            | "typeattribute"
            | "expandattribute"
    )
}

/// `b` が .te ファイルに見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut anchors = 0usize;
    let mut rules = 0usize;
    let mut decls = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with("//") {
            continue;
        }
        if is_anchor(tr) {
            anchors += 1;
        }
        if is_rule(tr) {
            rules += 1;
        }
        if is_decl(tr) {
            decls += 1;
        }
    }
    anchors >= 1 || (rules >= 2 && decls >= 1) || (rules + decls >= 4 && rules >= 1)
}

/// .te ファイルの統計。
#[derive(Debug, Default, Clone)]
pub struct SelinuxTe {
    /// policy_module/gen_require 等のアンカー行数。
    pub anchors: usize,
    /// allow/auditallow/type_transition 等のルール行数。
    pub rules: usize,
    /// type/attribute/class 等の宣言行数。
    pub decls: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .te ファイルとして統計する。
pub fn parse(b: &[u8]) -> SelinuxTe {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = SelinuxTe::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if is_anchor(tr) {
            c.anchors += 1;
        }
        if is_rule(tr) {
            c.rules += 1;
        }
        if is_decl(tr) {
            c.decls += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"policy_module(x, 1.0)\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.anchors, 1);
    }

    #[test]
    fn detects_rules() {
        let b = b"type a_t;\nallow a_t b_t:file read;\nallow a_t c_t:dir write;\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"type foo;\n"));
        assert!(!detect(b"key=value\nfoo=bar\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
        assert!(!detect(b"allow me to explain\nallow me to go\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.rules, 0);
    }
}
