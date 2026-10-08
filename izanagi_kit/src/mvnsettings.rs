//! Maven `settings.xml`(`.m2/settings.xml`)の検出と構造カウント。
//!
//! `<settings>` ルートと `<localRepository>`/`<interactiveMode>`/
//! `<usePluginRegistry>`/`<offline>` フラット設定、`<proxies>`/`<servers>`/
//! `<mirrors>`/`<profiles>`/`<activeProfiles>`/`<pluginGroups>` コンテナ、
//! その内部の `<proxy>`/`<server>`/`<mirror>`/`<profile>` ブロックと
//! `id`/`host`/`port`/`username`/`mirrorOf`/`url`/`activation`/`properties`/
//! `repositories` 等のフィールドを識別する。
//!
//! ```
//! let c = izanagi_kit::mvnsettings::parse(
//!     b"<settings>\n  <localRepository>/m2</localRepository>\n  <servers>\n    <server><id>a</id></server>\n  </servers>\n</settings>\n").unwrap();
//! assert_eq!(c.containers, 3);
//! assert_eq!(c.fields, 2);
//! assert!(izanagi_kit::mvnsettings::detect(b"<settings>\n<mirrors>\n<mirror>\n</mirror>\n</mirrors>\n</settings>\n"));
//! ```

/// コンテナ要素名(ルート + グループ + ブロック)。
const CONTAINERS: &[&str] = &[
    "activeProfile",
    "activeProfiles",
    "mirror",
    "mirrors",
    "pluginGroup",
    "pluginGroups",
    "profile",
    "profiles",
    "proxies",
    "proxy",
    "server",
    "servers",
    "settings",
];

/// フィールド要素名(単純値 + ネスト設定タグ)。
const FIELDS: &[&str] = &[
    "activation",
    "activeByDefault",
    "file",
    "host",
    "id",
    "interactiveMode",
    "jdk",
    "layout",
    "localRepository",
    "mirrorOf",
    "name",
    "nonProxyHosts",
    "offline",
    "os",
    "password",
    "passphrase",
    "port",
    "privateKey",
    "properties",
    "protocol",
    "releases",
    "repositories",
    "repository",
    "pluginRepositories",
    "pluginRepository",
    "snapshots",
    "url",
    "usePluginRegistry",
    "username",
    "value",
];

/// settings.xml 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// コンテナ要素の開始タグ(`<settings>`/`<servers>`/`<server>`/…)。
    pub containers: usize,
    /// フィールド要素タグ(`<id>`/`<host>`/`<url>`/…)。
    pub fields: usize,
    /// `<!-- -->` コメント行。
    pub comments: usize,
    /// 未知タグ/分類不能行。
    pub misc: usize,
}

/// 行内の開始/自己完結タグ名を列挙(閉じタグ `</name>` は除外)。
fn tags_in_line<'a>(t: &'a str, out: &mut Vec<&'a str>) {
    let b = t.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'<' && b[i + 1].is_ascii_alphabetic() {
            let start = i + 1;
            let mut j = start;
            while j < b.len()
                && (b[j].is_ascii_alphanumeric() || b[j] == b'.' || b[j] == b'-' || b[j] == b'_')
            {
                j += 1;
            }
            if j > start {
                out.push(&t[start..j]);
            }
            i = j;
        } else {
            i += 1;
        }
    }
}

fn strip_comments(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// b が settings.xml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = strip_comments(core::str::from_utf8(b).unwrap_or(""));
    let mut hits = 0;
    for line in text.lines() {
        let mut tags = Vec::new();
        tags_in_line(line.trim(), &mut tags);
        hits += tags
            .iter()
            .filter(|k| CONTAINERS.contains(k) || FIELDS.contains(k))
            .count();
    }
    hits >= 3
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = strip_comments(core::str::from_utf8(b).ok()?);
    let mut c = Counts {
        containers: 0,
        fields: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("<!--") {
            c.comments += 1;
            continue;
        }
        // 閉じタグのみの行は構造行。
        if t.starts_with("</") {
            continue;
        }
        let mut tags = Vec::new();
        tags_in_line(t, &mut tags);
        if tags.is_empty() {
            if t.chars().any(|ch| ch.is_alphanumeric()) && !t.starts_with("<?xml") {
                c.misc += 1;
            }
            continue;
        }
        for tag in tags {
            if CONTAINERS.contains(&tag) {
                c.containers += 1;
            } else if FIELDS.contains(&tag) {
                c.fields += 1;
            } else if tag != "?xml" {
                c.misc += 1;
            }
        }
    }
    (c.containers >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\"?>\n<settings>\n  <localRepository>/repo</localRepository>\n  <interactiveMode>true</interactiveMode>\n  <offline>false</offline>\n  <proxies>\n    <proxy>\n      <id>corporate</id>\n      <host>proxy.local</host>\n      <port>8080</port>\n      <nonProxyHosts>localhost</nonProxyHosts>\n    </proxy>\n  </proxies>\n  <servers>\n    <server>\n      <id>releases</id>\n      <username>me</username>\n      <password>x</password>\n    </server>\n  </servers>\n  <mirrors>\n    <mirror>\n      <id>central</id>\n      <mirrorOf>*</mirrorOf>\n      <url>https://repo.local/</url>\n    </mirror>\n  </mirrors>\n</settings>\n";

    #[test]
    fn mvnsettings() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.containers, 7);
        assert_eq!(c.fields, 13);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_settings() {
        assert!(!detect(b"<project><name>x</name></project>\n"));
        assert!(!detect(b"key=value\n"));
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}
