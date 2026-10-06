//! `kanshi` 設定ファイル検出モジュール。
//!
//! kanshi (Wayland 出力マネージャ) の設定は `profile { ... }`
//! ブロック内に `output <desc> <props>`、`exec <cmd>` 等の
//! ディレクティブを持つ形式。
//!
//! ```
//! let b = br#"profile {
//!     output "eDP-1" enable position 0,0
//!     output "HDMI-A-1" enable position 1920,0
//! }
//! profile mobile {
//!     output "eDP-1" enable
//!     exec waybar
//! }
//! "#;
//! let c = izanagi_kit::kanshi::parse(b);
//! assert!(izanagi_kit::kanshi::detect(b));
//! assert_eq!(c.profiles, 2);
//! ```

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn profile_line(t: &str) -> bool {
    t == "profile" || t.starts_with("profile ") || t.starts_with("profile{")
}

fn directive_line(t: &str) -> bool {
    t.starts_with("output ") || t.starts_with("exec ") || t == "output" || t == "exec"
}

/// `b` が kanshi 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut profiles = 0usize;
    let mut dirs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if profile_line(tr) {
            profiles += 1;
        } else if directive_line(tr) {
            dirs += 1;
        }
    }
    profiles >= 1 && dirs >= 1
}

/// kanshi 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct KanshiConf {
    /// `profile` ブロック数。
    pub profiles: usize,
    /// `output` 行数。
    pub outputs: usize,
    /// `exec` 行数。
    pub execs: usize,
    /// ブレース `{`/`}` 行数。
    pub braces: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を kanshi 設定として統計する。
pub fn parse(b: &[u8]) -> KanshiConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = KanshiConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if profile_line(tr) {
            c.profiles += 1;
            if tr.contains('{') {
                c.braces += 1;
            }
            continue;
        }
        if tr == "{" || tr == "}" || tr == "};" {
            c.braces += 1;
            continue;
        }
        if tr.starts_with("output ") || tr == "output" {
            c.outputs += 1;
        } else if tr.starts_with("exec ") || tr == "exec" {
            c.execs += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"profile {
    output "eDP-1" enable
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.profiles, 1);
        assert_eq!(c.outputs, 1);
    }

    #[test]
    fn detects_multi_profile() {
        let b = br#"profile docked {
    output "HDMI-A-1" enable mode 1920x1080
    exec swaymsg workspace 1
}
profile mobile {
    output "eDP-1" enable
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.profiles, 2);
        assert_eq!(c.outputs, 2);
        assert_eq!(c.execs, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"profile {\n}\n"));
        assert!(!detect(b"output foo\nexec bar\n"));
        assert!(!detect(b"{ \"a\": 1 }\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.profiles, 0);
    }
}
