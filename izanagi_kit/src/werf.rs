//! werf(`werf.yaml`)の検出と構造カウント。
//!
//! werf の主設定は `project: <name>` + `configVersion: 1` の
//! 組合せが必須で、極めて特異的なマーカーになる。
//! イメージ定義キー(`image`/`from`/`fromImage`/`dockerfile`/`context`/
//! `git`/`shell`/`ansible`/`mount`/`import`/`stageDependencies`/
//! `args`/`target`/`workdir`)も数える。
//!
//! ```
//! let b = b"project: myapp\nconfigVersion: 1\n---\nimage: frontend\ncontext: frontend\ndockerfile: Dockerfile\n";
//! assert!(izanagi_kit::werf::detect(b));
//! let c = izanagi_kit::werf::parse(b).unwrap();
//! assert_eq!(c.project.as_str(), "myapp");
//! assert!(c.image_keys >= 3);
//! ```

use crate::textutil::yaml_val;
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// werf イメージ定義で使われるキー。
const IMAGE_KEYS: &[&str] = &[
    "add",
    "ansible",
    "args",
    "assets",
    "context",
    "contextAddFiles",
    "dependencies",
    "dockerfile",
    "from",
    "fromArtifact",
    "fromImage",
    "git",
    "image",
    "imageSpec",
    "import",
    "mount",
    "mounts",
    "setup",
    "shell",
    "stageDependencies",
    "stapel",
    "target",
    "to",
    "workdir",
];

/// `werf.yaml` の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// `project:` の値(非検出時は空文字列)。
    pub project: String,
    /// `image:`/`fromImage:`/`dockerfile:` 等のイメージ定義キー行数。
    pub image_keys: usize,
    /// `---` ドキュメント区切り数。
    pub documents: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が `werf.yaml` に見えるかを判定する。
///
/// `project:` キーと `configVersion: 1` の両方を要求する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let has_project = t
        .lines()
        .any(|l| yaml_val(l.trim(), "project").is_some_and(|v| !v.is_empty()));
    let has_cv = t
        .lines()
        .any(|l| yaml_val(l.trim(), "configVersion") == Some("1"));
    has_project && has_cv
}

/// `b` を `werf.yaml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        project: t
            .lines()
            .find_map(|l| yaml_val(l.trim(), "project"))
            .unwrap_or("")
            .to_string(),
        image_keys: 0,
        documents: 0,
        comments: 0,
        misc: 0,
    };
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if tr == "---" {
            c.documents += 1;
            continue;
        }
        if IMAGE_KEYS.iter().any(|k| is_key(tr, k)) {
            c.image_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"project: myapp\nconfigVersion: 1\n---\nimage: frontend\ncontext: frontend\ndockerfile: Dockerfile\ntarget: builder\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(b"configVersion: 1\nproject: app\n"));
        // project だけ / configVersion だけでは検出しない。
        assert!(!detect(b"project: myapp\n"));
        assert!(!detect(b"configVersion: 1\n"));
        assert!(!detect(b"name: x\nversion: 1\n"));
        // コメント内の言及だけでは検出しない。
        assert!(!detect(b"# project: myapp\n# configVersion: 1\nfoo: bar\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.project.as_str(), "myapp");
        assert_eq!(c.documents, 1);
        assert!(c.image_keys >= 3);
        assert!(parse(b"project: myapp\n").is_none());
    }
}
