//! Earthly `Earthfile` の検出と構造カウント。
//!
//! 先頭の `VERSION` ディレクティブ、`target:` ターゲット定義、
//! `FROM`/`COPY`/`RUN`/`SAVE ARTIFACT`/`ARG`/`WITH DOCKER`/`DO`/`BUILD` 等の
//! Earthfile コマンドを識別する。
//!
//! ```
//! let c = izanagi_kit::earthly::parse(
//!     b"VERSION 0.7\nFROM alpine\n\nbuild:\n    RUN echo hi\n    SAVE ARTIFACT out\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert!(izanagi_kit::earthly::detect(
//!     b"VERSION 0.7\nFROM scratch\n"));
//! ```

use crate::textutil::strip_bom;
/// Earthfile コマンド(行頭語)。複数語は `|` 連結しない。
const CMDS: &[&str] = &[
    "ARG",
    "BUILD",
    "CACHE",
    "CMD",
    "COPY",
    "DO",
    "ELSE",
    "END",
    "ENTRYPOINT",
    "ENV",
    "EXPOSE",
    "FOR",
    "FROM",
    "GIT",
    "HEALTHCHECK",
    "HOST",
    "IF",
    "IMPORT",
    "LABEL",
    "LET",
    "LOCALLY",
    "ONBUILD",
    "PIPELINE",
    "PROJECT",
    "PUSH",
    "RUN",
    "SAVE",
    "SET",
    "SHELL",
    "STOPSIGNAL",
    "TRIGGER",
    "TRY",
    "USER",
    "VERSION",
    "VOLUME",
    "WAIT",
    "WITH",
    "WORKDIR",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ターゲット定義行数(`name:`)。
    pub sections: usize,
    /// コマンド行数(`VERSION` 含む)。
    pub entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn cmd_of(t: &str) -> Option<&str> {
    let w = t.split_whitespace().next()?;
    CMDS.contains(&w).then_some(w)
}

fn is_target(t: &str) -> bool {
    t.ends_with(':')
        && t[..t.len() - 1]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
}

/// Earthfile らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        return t == "VERSION"
            || t.starts_with("VERSION ")
            || (t.starts_with("FROM ") && is_target_line_follows(text));
    }
    false
}

fn is_target_line_follows(text: &str) -> bool {
    text.lines().any(|l| is_target(l.trim()))
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        entries: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if !line.starts_with(char::is_whitespace) && is_target(t) {
            c.sections += 1;
        } else if cmd_of(t).is_some() {
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

    const SAMPLE: &[u8] = b"VERSION 0.7\nFROM alpine:3\nWORKDIR /app\n\nbuild:\n    COPY . .\n    RUN cargo build --release\n    SAVE ARTIFACT target/release/app AS LOCAL out\n\ntest:\n    RUN cargo test\n    WITH DOCKER\n        RUN docker compose up\n    END\n";

    #[test]
    fn earthly() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.entries, 10);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_earthly() {
        assert!(!detect(b"FROM python\nRUN pip install x\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
