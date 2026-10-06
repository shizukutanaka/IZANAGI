//! nfpm 設定(`nfpm.yaml`)の検出と構造カウント。
//!
//! `contents:` の `- src:`/`- dst:`/`type:` 項目、`overrides:` の
//! `deb:`/`rpm:`/`apk:`/`archlinux:` サブセクション、`scripts:` の
//! `preinstall`/`postinstall`/`preremove`/`postremove`、メタキー
//! (`name`/`arch`/`platform`/`version`/`section`/`priority`/`maintainer`/
//! `description`/`vendor`/`homepage`/`license`/`bindir`/`epoch`/`release`/
//! `prerelease`/`version_metadata`)を識別する。
//!
//! ```
//! let c = izanagi_kit::nfpm::parse(
//!     b"name: myapp\narch: amd64\nplatform: linux\nversion: v1.2.3\nmaintainer: Me <m@x>\ndescription: app\ncontents:\n  - src: ./bin/app\n    dst: /usr/bin/app\noverrides:\n  deb:\n    depends: [libc6]\n").unwrap();
//! assert_eq!(c.contents, 1);
//! assert!(izanagi_kit::nfpm::detect(
//!     b"contents:\n  - src: ./a\n    dst: /opt/a\n"));
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

/// `nfpm.yaml` の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// `contents:` の `- src:` 項目数。
    pub contents: usize,
    /// `overrides:` の `deb:`/`rpm:`/`apk:`/`archlinux:` 行数。
    pub overrides: usize,
    /// `scripts:` フックキー行数。
    pub script_keys: usize,
    /// メタキー行数。
    pub meta_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// メタキー。
const META_KEYS: &[&str] = &[
    "arch",
    "bindir",
    "description",
    "epoch",
    "homepage",
    "license",
    "maintainer",
    "name",
    "platform",
    "prerelease",
    "priority",
    "release",
    "section",
    "vendor",
    "version",
    "version_metadata",
];

/// `b` が `nfpm.yaml` に見えるかを判定する。
///
/// `contents:` と `- src:`/`- dst:` 項目ペア、または `overrides:` と
/// `deb:`/`rpm:`/`apk:` サブキーを要求する — メタキーは汎用語のため
/// 単独では判定しない。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let contents = has_key(t, "contents")
        && t.lines().any(|l| is_item(l.trim(), "src"))
        && t.lines().any(|l| is_item(l.trim(), "dst"));
    let overrides = has_key(t, "overrides")
        && ["deb", "rpm", "apk", "archlinux"]
            .iter()
            .any(|k| has_key(t, k));
    contents || overrides
}

/// `b` を `nfpm.yaml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        contents: 0,
        overrides: 0,
        script_keys: 0,
        meta_keys: 0,
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
        if is_item(tr, "src") {
            c.contents += 1;
            continue;
        }
        if ["deb", "rpm", "apk", "archlinux", "ipk"]
            .iter()
            .any(|k| is_key(tr, k))
        {
            c.overrides += 1;
            continue;
        }
        if [
            "preinstall",
            "postinstall",
            "preremove",
            "postremove",
            "preupgrade",
            "postupgrade",
        ]
        .iter()
        .any(|k| is_key(tr, k))
        {
            c.script_keys += 1;
            continue;
        }
        if META_KEYS.iter().any(|k| is_key(tr, k)) {
            c.meta_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"name: myapp\narch: amd64\nplatform: linux\nversion: v1.2.3\nsection: utils\npriority: extra\nmaintainer: Me <m@x>\ndescription: An app\nvendor: Corp\nhomepage: https://x\nlicense: MIT\ncontents:\n  - src: ./bin/app\n    dst: /usr/bin/app\n    type: file\n  - src: ./etc/app.conf\n    dst: /etc/app.conf\nscripts:\n  postinstall: ./scripts/post.sh\noverrides:\n  deb:\n    depends:\n      - libc6\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(b"contents:\n  - src: ./a\n    dst: /opt/a\n"));
        assert!(detect(b"overrides:\n  rpm:\n    group: apps\n"));
        assert!(!detect(b"# contents:\n#   - src: ./a\n#     dst: /opt/a\n"));
        assert!(!detect(b"name: x\nversion: 1\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.contents, 2);
        assert_eq!(c.overrides, 1);
        assert_eq!(c.script_keys, 1);
        assert!(c.meta_keys >= 8);
        assert!(parse(b"name: x\n").is_none());
    }
}
