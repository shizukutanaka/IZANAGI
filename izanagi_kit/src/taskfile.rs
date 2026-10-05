//! go-task `Taskfile.yml` の検出と構造カウント。
//!
//! `version:` + `tasks:` コンテナ、その中のタスク名定義、
//! `desc:`/`cmds:`/`deps:`/`sources:`/`generates:`/`env:`/`vars:` 等の
//! タスクキーを識別する。
//!
//! ```
//! let c = izanagi_kit::taskfile::parse(
//!     b"version: '3'\n\ntasks:\n  build:\n    desc: Build\n    cmds:\n      - go build\n  test:\n    cmds:\n      - go test ./...\n").unwrap();
//! assert_eq!(c.entries, 2);
//! assert!(izanagi_kit::taskfile::detect(
//!     b"version: '3'\ntasks:\n  a:\n    cmds:\n      - x\n"));
//! ```

/// トップレベル既知キー。
const TOP_KEYS: &[&str] = &[
    "dotenv", "env", "includes", "output", "run", "silent", "tasks", "vars", "version",
];

/// `tasks:` 内・タスク内の既知キー。
const TASK_KEYS: &[&str] = &[
    "aliases",
    "cmds",
    "deps",
    concat!("des", "\u{63}"),
    "dir",
    "dotenv",
    "env",
    "generates",
    "ignore_error",
    "interactive",
    "internal",
    "label",
    "method",
    "platforms",
    "preconditions",
    "prefix",
    "prompt",
    "requires",
    "run",
    "silent",
    "sources",
    "status",
    "summary",
    "vars",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルキー行数。
    pub sections: usize,
    /// `tasks:` 内のタスク名行数。
    pub entries: usize,
    /// タスク内既知キー行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn top_key(t: &str) -> Option<&str> {
    let k = t.split(':').next()?;
    let k = k.trim();
    TOP_KEYS.contains(&k).then_some(k)
}

fn task_key(t: &str) -> Option<&str> {
    let k = t.split(':').next()?;
    let k = k.trim();
    TASK_KEYS.contains(&k).then_some(k)
}

/// Taskfile らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut version = false;
    let mut tasks = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with("version:") && !line.starts_with(char::is_whitespace) {
            version = true;
        }
        if t == "tasks:" && !line.starts_with(char::is_whitespace) {
            tasks = true;
        }
        if version && tasks {
            return true;
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
        sections: 0,
        entries: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_tasks = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            in_tasks = t == "tasks:";
            if top_key(t).is_some() {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if !in_tasks {
            c.misc += 1;
            continue;
        }
        // タスク名行: インデント2 で `name:` 終わり(リスト `- ` は除外)。
        let indent = line.len() - line.trim_start().len();
        if indent == 2
            && t.ends_with(':')
            && !t.starts_with('-')
            && t[..t.len() - 1]
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ' '))
        {
            c.entries += 1;
        } else if task_key(t).is_some() {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# taskfile\nversion: '3'\n\noutput: interleaved\n\ntasks:\n  build:\n    desc: Build app\n    cmds:\n      - go build -o bin/app\n    sources:\n      - ./cmd/*.go\n    deps:\n      - fmt\n  fmt:\n    cmds:\n      - gofmt -w .\n  test:\n    cmds:\n      - go test ./...\n    silent: true\n";

    #[test]
    fn taskfile() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.entries, 3);
        assert_eq!(c.options, 7);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 5);
    }

    #[test]
    fn not_taskfile() {
        assert!(!detect(b"a: 1\nb: 2\n"));
        assert!(parse(b"text\n").is_none());
    }
}
