//! `.Rprofile` 検出モジュール。
//!
//! R 起動時に評価されるスクリプト。`options(...)`、`library(...)`/
//! `require(...)`、`.First`/`.Last` フック、`local({...})`/
//! `Sys.setenv(...)`、`<-` 代入が特徴。
//!
//! ```
//! let b = br#"options(repos = c(CRAN = "https://cloud.r-project.org"))
//! options(width = 120)
//! library(stats)
//! .First <- function() {
//!   cat("Welcome\n")
//! }
//! Sys.setenv(TZ = "UTC")
//! "#;
//! let c = izanagi_kit::rprofile::parse(b);
//! assert!(izanagi_kit::rprofile::detect(b));
//! assert!(c.option_calls >= 2);
//! ```

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn is_r_line(t: &str) -> bool {
    t.starts_with("options(")
        || t.starts_with("option(")
        || t.starts_with("library(")
        || t.starts_with("require(")
        || t.starts_with("suppressMessages(")
        || t.starts_with("suppressWarnings(")
        || t.starts_with("Sys.setenv(")
        || t.starts_with("Sys.setlocale(")
        || t.starts_with("Sys.setLanguage(")
        || t.starts_with("setwd(")
        || t.starts_with("setHook(")
        || t.starts_with("message(")
        || t.starts_with("cat(")
        || t.starts_with("local(")
        || t.starts_with(".First")
        || t.starts_with(".Last")
        || t.contains("<-")
        || t.contains("->")
}

/// `b` が .Rprofile に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut r_lines = 0usize;
    let mut opts = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if tr.starts_with("options(") {
            opts += 1;
            r_lines += 1;
        } else if is_r_line(tr) {
            r_lines += 1;
        }
    }
    (opts >= 1 && r_lines >= 3) || r_lines >= 5
}

/// .Rprofile の統計。
#[derive(Debug, Default, Clone)]
pub struct RProfile {
    /// R らしい行の総数。
    pub r_lines: usize,
    /// `options(`/`option(` 行数。
    pub option_calls: usize,
    /// `library(`/`require(` 行数。
    pub library_calls: usize,
    /// `<-`/`->` 代入を含む行数。
    pub assignments: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .Rprofile として統計する。
pub fn parse(b: &[u8]) -> RProfile {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = RProfile::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tr.starts_with("options(") || tr.starts_with("option(") {
            c.option_calls += 1;
        }
        if tr.starts_with("library(") || tr.starts_with("require(") {
            c.library_calls += 1;
        }
        if tr.contains("<-") || tr.contains("->") {
            c.assignments += 1;
        }
        if is_r_line(tr) {
            c.r_lines += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"options(repos = c(CRAN = "https://cloud.r-project.org"))
options(width = 120)
library(stats)
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.option_calls, 2);
        assert_eq!(c.library_calls, 1);
    }

    #[test]
    fn detects_hooks() {
        let b = br#".First <- function() {
  options(digits = 4)
}
.Last <- function() {
  cat("bye\n")
}
local({
  x <- 1
})
"#;
        assert!(detect(b));
        let c = parse(b);
        assert!(c.assignments >= 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"options(x = 1)\n"));
        assert!(!detect(b"foo()\nbar()\nbaz()\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.r_lines, 0);
    }
}
