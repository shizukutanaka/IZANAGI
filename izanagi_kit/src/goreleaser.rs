//! GoReleaser 設定(`.goreleaser.yml`/`goreleaser.yaml`)の検出と構造カウント。
//!
//! `project_name:`/`before:`/`builds:`(`goos:`/`goarch:`/`goarm:`/`targets:`/
//! `main:`/`ldflags:`/`flags:`/`env:`)/`archives:`/`dockers:`/`nfpms:`/`brews:`/
//! `scoops:`/`snapshots`/`snapshot:`/`changelog:`/`release:`/`checksum:`/
//! `signs:`/`sboms:`/`universal_binaries:`/`ko:`/`aurs:`/`source:` キーを識別する。
//!
//! ```
//! let c = izanagi_kit::goreleaser::parse(
//!     b"project_name: myapp\nbuilds:\n  - id: myapp\n    goos: [linux, darwin]\n    goarch: [amd64, arm64]\narchives:\n  - id: tgz\n    format: tar.gz\nchecksum:\n  name_template: checksums.txt\n").unwrap();
//! assert_eq!(c.build_items, 2);
//! assert!(c.top_keys >= 4);
//! assert!(izanagi_kit::goreleaser::detect(
//!     b"builds:\n  - goos: [linux]\n    goarch: [amd64]\n"));
//! ```

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| is_key(l.trim(), key))
}

/// `- key:` リスト項目行かどうか。
fn is_item(tr: &str, key: &str) -> bool {
    let t = tr.trim_start_matches('-').trim_start();
    is_key(t, key)
}

/// ビルド関連キー。
const BUILD_KEYS: &[&str] = &[
    "binary", "env", "flags", "goarch", "goarm", "gomips", "goos", "ldflags", "main", "targets",
];

/// GoReleaser 固有のトップレベルキー。
const TOP_KEYS: &[&str] = &[
    "announce",
    "archives",
    "aurs",
    "before",
    "blobs",
    "brews",
    "builds",
    "changelog",
    "checksum",
    "chocolateys",
    "dockers",
    "docker_manifests",
    "envs",
    "flatpaks",
    "gomod",
    "ko",
    "metadata",
    "milestones",
    "nfpm",
    "nfpms",
    "nightly",
    "project_name",
    "release",
    "sboms",
    "scoop",
    "scoops",
    "signs",
    "snapcrafts",
    "snapshot",
    "source",
    "universal_binaries",
    "uploads",
    "winget",
];

/// `.goreleaser.yml` の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// `- id:`/`goos:` 等のビルド項目数。
    pub build_items: usize,
    /// 既知トップレベルキー行数。
    pub top_keys: usize,
    /// ビルド関連キー行数。
    pub build_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が `.goreleaser.yml` に見えるかを判定する。
///
/// `builds:`+go ターゲットキー、`project_name:`+`builds:`、または
/// `archives:`/`nfpm(s):`/`brews:`/`checksum:` の GoReleaser 固有組合せを要求する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let has = |k: &str| has_key(t, k);
    (has("builds") && (has("goos") || has("goarch") || has("targets")))
        || (has("project_name") && has("builds"))
        || (has("archives") && (has("checksum") || has("nfpm") || has("nfpms") || has("brews")))
}

/// `b` を `.goreleaser.yml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        build_items: 0,
        top_keys: 0,
        build_keys: 0,
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
        if is_item(tr, "id") {
            c.build_items += 1;
            continue;
        }
        if BUILD_KEYS.iter().any(|k| is_key(tr, k)) {
            c.build_keys += 1;
            continue;
        }
        if TOP_KEYS.iter().any(|k| is_key(tr, k)) {
            c.top_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"project_name: myapp\nbefore:\n  hooks:\n    - go mod tidy\nbuilds:\n  - id: myapp\n    main: ./cmd/app\n    goos: [linux, darwin]\n    goarch: [amd64, arm64]\n    env:\n      - CGO_ENABLED=0\narchives:\n  - id: tgz\n    format: tar.gz\nnfpms:\n  - id: myapp\nchecksum:\n  name_template: checksums.txt\nchangelog:\n  sort: asc\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(b"builds:\n  - goos: [linux]\n    goarch: [amd64]\n"));
        assert!(!detect(
            b"# builds:\n#   - goos: [linux]\n#     goarch: [amd64]\n"
        ));
        assert!(!detect(b"steps:\n  build:\n    image: rust\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.build_items, 3);
        assert!(c.top_keys >= 5);
        assert!(c.build_keys >= 4);
        assert!(parse(b"name: x\n").is_none());
    }
}
