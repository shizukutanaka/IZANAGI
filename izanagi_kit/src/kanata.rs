//! `kanata.kbd` (Kanata キーボードリマッパ) 検出モジュール。
//!
//! Kanata は S 式設定で、`(defcfg ...)`/`(defsrc ...)`/
//! `(deflayer <name> ...)`/`(defalias ...)`/`(defaliasenvcond ...)`/
//! `(defchordsv2 ...)`/`(deflayermap ...)`/`(defoverrides ...)`/
//! `(defseq ...)`/`(defvirtualkeys ...)`/`(deflocalkeys-*)`/
//! `(defvar ...)`/`(deftemplate ...)` フォームと、defcfg 内の
//! `process-unmapped-keys`/`danger-enable-cmd`/`concurrent-tap-hold`/
//! `rapid-event-delay`/`linux-continue-if-no-devs-found`/`linux-dev`/
//! `linux-dev-names-include`/`log-layer-changes`/`delegate-to-first-layer`
//! オプションで構成される。
//!
//! ```
//! let b = b"(defcfg process-unmapped-keys yes\n  linux-dev /dev/input/event0)\n\
//!           (defsrc caps a s d)\n\
//!           (defalias cap (tap-hold 200 200 esc lctl))\n\
//!           (deflayer base @cap a s d)\n";
//! let c = izanagi_kit::kanata::parse(b);
//! assert!(izanagi_kit::kanata::detect(b));
//! assert_eq!(c.forms, 4);
//! ```

const FORMS: &[&str] = &[
    "(defalias",
    "(defaliasenvcond",
    "(defcfg",
    "(defchords",
    "(deflayermap",
    "(deflayer",
    "(deflocalkeys",
    "(defoverrides",
    "(defseq",
    "(defsrc",
    "(deftemplate",
    "(defvar",
    "(defvirtualkeys",
    "(deffakekeys",
    "(zippy-include",
];

const CFGOPTS: &[&str] = &[
    "concurrent-tap-hold",
    "danger-enable-cmd",
    "delegate-to-first-layer",
    "dynamic-macro-replay",
    "linux-continue-if-no-devs-found",
    "linux-dev",
    "linux-dev-names-exclude",
    "linux-dev-names-include",
    "linux-unicode-sending",
    "log-layer-changes",
    "process-unmapped-keys",
    "rapid-event-delay",
    "sequence-timeout",
];

fn is_form(t: &str) -> bool {
    FORMS.iter().any(|f| t.starts_with(f))
}

fn has_cfgopt(t: &str) -> bool {
    CFGOPTS.iter().any(|o| t.contains(o))
}

/// `b` が kanata.kbd に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut src = 0usize;
    let mut forms = 0usize;
    let mut opts = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with(';') {
            continue;
        }
        if tr.starts_with("(defsrc") {
            src += 1;
            forms += 1;
        } else if is_form(tr) {
            forms += 1;
        }
        if has_cfgopt(tr) {
            opts += 1;
        }
    }
    (src >= 1 && forms >= 3) || forms >= 4 || (forms >= 2 && opts >= 2)
}

/// kanata.kbd の統計。
#[derive(Debug, Default, Clone)]
pub struct Kanata {
    /// 既知フォーム数。
    pub forms: usize,
    /// defcfg 既知オプション言及数。
    pub cfg_options: usize,
    /// コメント行数(`;;`始まり)。
    pub comments: usize,
}

/// `b` を kanata.kbd として統計する。
pub fn parse(b: &[u8]) -> Kanata {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Kanata::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if is_form(tr) {
            c.forms += 1;
        }
        if has_cfgopt(tr) {
            c.cfg_options += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"(defsrc a b c)\n(deflayer base x y z)\n(defalias k lctl)\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.forms, 3);
    }

    #[test]
    fn detects_cfg() {
        let b = b"(defcfg process-unmapped-keys yes danger-enable-cmd yes)\n(defsrc a)\n(deflayer base b)\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"(defsrc a b c)\n"));
        assert!(!detect(b"(foo)\n(bar)\n(baz)\n"));
        assert!(!detect(b"key=value\nfoo=bar\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.forms, 0);
    }
}
