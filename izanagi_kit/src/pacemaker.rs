//! Pacemaker CIB XML (`cib.xml`) 検出モジュール。
//!
//! Pacemaker のクラスタ設定(CIB)は `<cib>` ルート + `<configuration>` +
//! `<crm_config>`/`<nodes>`/`<resources>`/`<constraints>`/
//! `<op_defaults>`/`<rsc_defaults>`/`<acls>`/`<fencing-level>`/
//! `<alerts>`/`<status>` セクション、リソース要素 `primitive`/
//! `clone`/`group`/`master`/`bundle`、制約要素 `rsc_location`/
//! `rsc_order`/`rsc_colocation`/`rsc_ticket`/`lrm_rsc_op`、
//! メタ要素 `nvpair`/`meta_attributes`/`instance_attributes`/
//! `utilization`/`rule`/`expression`/`date_expression`/`op`/
//! `operations`、クラスタプロパティ `cluster_property_set` で
//! 構成される。
//!
//! ```
//! let b = b"<cib crm_feature_set=\"3.9\" validate-with=\"pacemaker-3.9\">\n\
//!           <configuration>\n\
//!           <crm_config><cluster_property_set id=\"cib-bootstrap-options\"/>\n\
//!           </crm_config>\n\
//!           <nodes><node id=\"1\" uname=\"node1\"/></nodes>\n\
//!           <resources><primitive id=\"ip\" class=\"ocf\" provider=\"heartbeat\"\n\
//!           type=\"IPaddr2\"/></resources>\n\
//!           <constraints><rsc_location id=\"l1\" rsc=\"ip\" node=\"node1\" score=\"100\"/></constraints>\n\
//!           </configuration>\n\
//!           </cib>\n";
//! let c = izanagi_kit::pacemaker::parse(b);
//! assert!(izanagi_kit::pacemaker::detect(b));
//! assert!(c.elems >= 4);
//! ```

const ELEMS: &[&str] = &[
    "acls",
    "alerts",
    "attributes",
    "bundle",
    "cib",
    "cluster_property_set",
    "configuration",
    "constraints",
    "crm_config",
    "date_expression",
    "expression",
    "fencing-level",
    "group",
    "instance_attributes",
    "lrm_rsc_op",
    "meta_attributes",
    "node",
    "nodes",
    "nvpair",
    "op",
    "op_defaults",
    "operations",
    "primitive",
    "recipient",
    "replication",
    "resource_set",
    "resources",
    "role",
    "rsc_colocation",
    "rsc_defaults",
    "rsc_location",
    "rsc_order",
    "rsc_ticket",
    "rule",
    "status",
    "tag",
    "target",
    "utilization",
];

fn elem_name(t: &str) -> &str {
    let t = t.trim_start_matches('<');
    let t = t.strip_prefix('/').unwrap_or(t);
    t.split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("")
}

/// `b` が Pacemaker CIB に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    if !t.contains("<cib") && !t.contains("<configuration") {
        return false;
    }
    let mut elems = 0usize;
    for l in t.lines() {
        let mut s = l.trim_start();
        while let Some(pos) = s.find('<') {
            let name = elem_name(&s[pos + 1..]);
            if ELEMS.contains(&name) {
                elems += 1;
            }
            s = &s[pos + 1..];
        }
    }
    elems >= 3
}

/// CIB の統計。
#[derive(Debug, Default, Clone)]
pub struct Pacemaker {
    /// 既知 CIB 要素数。
    pub elems: usize,
}

/// `b` を CIB XML として統計する。
pub fn parse(b: &[u8]) -> Pacemaker {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Pacemaker::default();
    for l in t.lines() {
        let mut s = l.trim_start();
        while let Some(pos) = s.find('<') {
            let name = elem_name(&s[pos + 1..]);
            if ELEMS.contains(&name) {
                c.elems += 1;
            }
            s = &s[pos + 1..];
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"<cib>\n<configuration>\n<nodes><node/></nodes>\n</configuration>\n</cib>\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.elems, 7);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"<html><body><cib/></body></html>\n"));
        assert!(!detect(b"key=value\nfoo=bar\n"));
        assert!(!detect(
            b"<configuration>\n<item/>\n<item/>\n<item/>\n</configuration>\n"
        ));
    }

    #[test]
    fn comment_lines_ok() {
        let b = b"<!-- cib -->\n<cib>\n<configuration>\n<nodes/>\n</configuration>\n</cib>\n";
        assert!(detect(b));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.elems, 0);
    }
}
