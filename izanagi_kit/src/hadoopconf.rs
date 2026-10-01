//! Hadoop `*-site.xml` (`core-site.xml`/`hdfs-site.xml`/`mapred-site.xml`/`yarn-site.xml`)
//! パーサ。`<property>`/`<name>`/`<value>` と `fs.defaultFS`/`dfs.*`/`mapreduce.*`/
//! `yarn.*`/`hadoop.*` 既知プロパティを計数する。
//!
//! ```
//! use izanagi_kit::hadoopconf;
//! let x = b"<?xml version=\"1.0\"?>\n<configuration>\n<property>\n<name>fs.defaultFS</name>\n<value>hdfs://nn:9000</value>\n</property>\n<property>\n<name>dfs.replication</name>\n<value>3</value>\n</property>\n</configuration>\n";
//! assert!(hadoopconf::detect(x));
//! let c = hadoopconf::parse(x).unwrap();
//! assert_eq!(c.properties, 2);
//! assert_eq!(c.known_prefixes, 2);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<property>` ブロック数。
    pub properties: usize,
    /// `<name>` タグ数。
    pub names: usize,
    /// 既知プロパティ接頭辞数。
    pub known_prefixes: usize,
    /// `<value>` タグ数。
    pub values: usize,
}

const KNOWN_PREFIXES: &[&str] = &[
    "fs.default",
    "dfs.",
    "mapred.",
    "mapreduce.",
    "yarn.",
    "hadoop.",
    "ha.zookeeper.",
    "ipc.",
    "net.topology.",
    "io.compression.",
    "fs.trash",
    "fs.s3",
    "fs.s3a",
    "fs.azure",
    "fs.gs",
    "ssl.",
    "security.",
];

/// `*-site.xml` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => c.properties >= 2 && c.known_prefixes >= 1,
        None => false,
    }
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        properties: 0,
        names: 0,
        known_prefixes: 0,
        values: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.contains("<property") {
            c.properties += 1;
        }
        if let Some(rest) = t.find("<name>").map(|i| &t[i + 6..]) {
            c.names += 1;
            let name = rest.split('<').next().unwrap_or("").trim();
            if KNOWN_PREFIXES.iter().any(|p| name.starts_with(p)) {
                c.known_prefixes += 1;
            }
        }
        if t.contains("<value") {
            c.values += 1;
        }
    }
    (c.properties > 0 || c.names > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<configuration>\n<property>\n<name>fs.defaultFS</name>\n<value>hdfs://nn:9000</value>\n</property>\n<property>\n<name>dfs.replication</name>\n<value>3</value>\n</property>\n<property>\n<name>yarn.resourcemanager.hostname</name>\n<value>rm1</value>\n</property>\n</configuration>\n";

    #[test]
    fn detects_site_xml() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.properties, 3);
        assert_eq!(c.names, 3);
        assert_eq!(c.known_prefixes, 3);
        assert_eq!(c.values, 3);
    }

    #[test]
    fn rejects_generic_xml() {
        let x = b"<configuration>\n<property>\n<name>foo</name>\n<value>1</value>\n</property>\n<property>\n<name>bar</name>\n<value>2</value>\n</property>\n</configuration>\n";
        assert!(!detect(x));
    }
}
