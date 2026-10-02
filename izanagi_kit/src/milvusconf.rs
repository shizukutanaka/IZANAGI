//! Milvus `milvus.yaml` の検出と構造カウント。
//!
//! `etcd`/`minio`/`mq`/`pulsar`/`rootCoord`/`proxy`/`queryCoord`/`dataCoord`/
//! `indexCoord`/`common` 等の既知トップキーと配下キーをインデント走査で分類する。
//!
//! ```
//! let c = izanagi_kit::milvusconf::parse(
//!     b"etcd:\n  endpoints: localhost:2379\nminio:\n  address: localhost\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert!(izanagi_kit::milvusconf::detect(b"etcd:\n  endpoints: x\ncommon:\n  channel:\n"));
//! ```

/// 既知トップレベルセクション。
const SECTIONS: &[&str] = &[
    "common",
    "dataCoord",
    "dataNode",
    "etcd",
    "gpu",
    "indexCoord",
    "indexNode",
    "kafka",
    "localStorage",
    "log",
    "metastore",
    "metrics",
    "minio",
    "mixCoord",
    "mq",
    "natsmq",
    "objectStorage",
    "proxy",
    "pulsar",
    "queryCoord",
    "queryNode",
    "rocksmq",
    "rootCoord",
    "tls",
    "trace",
    "woodpecker",
];
/// 配下の既知キー。
const SUBKEYS: &[&str] = &[
    "accessKeyID",
    "address",
    "bucketName",
    "caPemPath",
    "channel",
    "clientPemPath",
    "clientPemPath",
    "cloudProvider",
    "common",
    "compact",
    "configType",
    "defaultPartitionNum",
    "enabled",
    "endpoints",
    "etcd",
    "gracefulTime",
    "healthz",
    "host",
    "io",
    "ip",
    "level",
    "listen",
    "maxAddressLength",
    "maxChannelNum",
    "mem",
    "path",
    "pemPemPath",
    "port",
    "privacyPolicyEnabled",
    "requestTimeoutMs",
    "restful",
    "retry",
    "rootPath",
    "serverMode",
    "ssl",
    "standby",
    "subscriptionPos",
    "timeout",
    "tlsMode",
    "topicPrefix",
    "useSSL",
    "useVirtualHost",
    "useIAM",
    "writeBufferSize",
];

/// Milvus 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップセクション行。
    pub sections: usize,
    /// セクション配下の既知キー。
    pub options: usize,
    /// `- ` リスト項目。
    pub items: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が milvus.yaml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| {
            let t = l.trim();
            t.find(':')
                .is_some_and(|p| SECTIONS.contains(&t[..p].trim()))
        })
        .count()
        >= 2
}

/// milvus.yaml の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        items: 0,
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
        if t.starts_with('-') {
            c.items += 1;
            continue;
        }
        let Some(colon) = t.find(':') else {
            c.misc += 1;
            continue;
        };
        let key = t[..colon].trim();
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            if SECTIONS.contains(&key) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
        } else if SUBKEYS.contains(&key) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# milvus\netcd:\n  endpoints: localhost:2379\n  rootPath: by-dev\n  ssl:\n    enabled: false\nminio:\n  address: localhost\n  port: 9000\n  bucketName: milvus-bucket\nmq:\n  type: pulsar\nproxy:\n  port: 19530\ncommon:\n  channel:\n    prefix: by-dev\nlog:\n  level: info\n";

    #[test]
    fn milvusconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 6);
        assert!(c.options >= 8);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_milvus() {
        assert!(!detect(b"key: value\nother: 1\n"));
        assert!(!detect(b"hello\n"));
    }
}
