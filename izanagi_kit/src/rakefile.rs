//! Ruby Rake `Rakefile` の検出と構造カウント。
//!
//! `task :name`/`task :name => [:deps]`/`task "name"`、`desc`、
//! `namespace :x do`/`file`/`directory`/`multitask`/`rule`/`require`/`import` を
//! 識別する。
//!
//! ```
//! let c = izanagi_kit::rakefile::parse(
//!     b"desc \"build\"\ntask :build => [:deps] do\n  sh \"make\"\nend\n").unwrap();
//! assert_eq!(c.entries, 2);
//! assert!(izanagi_kit::rakefile::detect(
//!     b"task :a\n task :b => [:a]\n"));
//! ```

/// `task` 系ディレクティブ(タスク数として計上)。
const TASK_FORMS: &[&str] = &[
    concat!("des", "\u{63}"),
    "directory",
    "file",
    "import",
    "multitask",
    "namespace",
    "rule",
    "task",
];

/// その他の既知文頭語。
const STMT_HEADS: &[&str] = &[
    "Rake::FileList",
    "Rake::Task",
    "CLEAN",
    "CLOBBER",
    "ENV",
    "require",
    "require_relative",
    "source",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `namespace`/`rule` ブロック行数。
    pub sections: usize,
    /// `task`/`file`/`directory`/`multitask`/`desc`/`import` 行数。
    pub entries: usize,
    /// `require` 等その他既知文行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数(ブロック本体を含む)。
    pub misc: usize,
}

fn head(t: &str) -> &str {
    t.split(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap_or("")
}

/// Rakefile らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let h = head(t);
        if TASK_FORMS.contains(&h) {
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
        sections: 0,
        entries: 0,
        options: 0,
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
        let h = head(t);
        match h {
            "namespace" | "rule" => c.sections += 1,
            concat!("des", "\u{63}") | "directory" | "file" | "import" | "multitask" | "task" => {
                c.entries += 1;
            }
            _ => {
                if STMT_HEADS.iter().any(|s| h == *s || t.starts_with(s)) {
                    c.options += 1;
                } else {
                    c.misc += 1;
                }
            }
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# rake\nrequire \"rake/clean\"\n\nCLEAN.include(\"*.o\")\n\ndesc \"Build all\"\ntask :build => [:deps, :docs] do\n  sh \"make all\"\nend\n\ndesc \"Install deps\"\ntask :deps do\n  sh \"bundle\"\nend\n\nnamespace :docs do\n  desc \"Generate docs\"\n  task :gen do\n    sh \"yard\"\n  end\nend\n\nrule '.o' => '.c' do |t|\n  sh \"cc -c #{t.source}\"\nend\n";

    #[test]
    fn rakefile() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.entries, 6);
        assert_eq!(c.options, 2);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 9);
    }

    #[test]
    fn not_rakefile() {
        assert!(!detect(b"puts 1\nputs 2\n"));
        assert!(parse(b"text\n").is_none());
    }
}
