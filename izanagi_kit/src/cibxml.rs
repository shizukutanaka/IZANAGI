//! Pacemaker CIB (Cluster Information Base) XML の解析。
//!
//! `<cib>`/`<configuration>`/`<crm_config>`/`<nodes>`/`<node>`/`<resources>`/
//! `<primitive>`/`<group>`/`<clone>`/`<master>`/`<constraints>`/
//! `<rsc_location>`/`<rsc_colocation>`/`<rsc_order>`/`<nvpair>`/`<op>`/
//! `<meta_attributes>`/`<instance_attributes>`/`<rsc_defaults>`/
//! `<op_defaults>`/`<acls>`/`<alerts>` 要素を検出し、リソース・制約・
//! 属性の数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::cibxml;
//!
//! let text = br#"<cib>
//!   <configuration>
//!     <resources>
//!       <primitive id="vip" class="ocf" provider="heartbeat" type="IPaddr2"/>
//!     </resources>
//!   </configuration>
//! </cib>
//! "#;
//!
//! assert!(cibxml::detect(text));
//! let c = cibxml::parse(text).unwrap();
//! assert_eq!(c.primitives, 1);
//! ```

/// CIB XML の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<cib>` ルート要素数。
    pub cib: usize,
    /// `<primitive …>` リソース定義数。
    pub primitives: usize,
    /// `<group>`/`<clone>`/`<master>`/`<bundle>` コンテナリソース数。
    pub containers: usize,
    /// `<node` 要素数 (`<nodes>` 自体は除外)。
    pub nodes: usize,
    /// `<rsc_location>`/`<rsc_colocation>`/`<rsc_order>`/`<rsc_ticket>` 制約数。
    pub constraints: usize,
    /// `<nvpair` 属性ペア数。
    pub nvpairs: usize,
    /// `<op ` 監視/操作定義数 (`<op_defaults` 等は除外)。
    pub ops: usize,
    /// `<meta_attributes>`/`<instance_attributes>` 属性コンテナ数。
    pub attribute_blocks: usize,
}

/// `b` が CIB XML らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.cib >= 1 && (c.primitives >= 1 || c.nodes >= 1 || c.containers >= 1)
}

fn count_tag(line: &str, tag: &str) -> usize {
    line.matches(tag).count()
}

/// CIB XML を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        cib: 0,
        primitives: 0,
        containers: 0,
        nodes: 0,
        constraints: 0,
        nvpairs: 0,
        ops: 0,
        attribute_blocks: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with("<!--")
            || line.starts_with("<?xml")
            || line.starts_with("<!")
        {
            continue;
        }
        counts.cib += count_tag(line, "<cib");
        // `<node ` / `<node>` / `<node/>` は数えるが `<nodes>` は除外。
        let node_tags =
            count_tag(line, "<node ") + count_tag(line, "<node>") + count_tag(line, "<node/");
        counts.nodes += node_tags;
        counts.primitives += count_tag(line, "<primitive");
        counts.containers += count_tag(line, "<group")
            + count_tag(line, "<clone")
            + count_tag(line, "<master")
            + count_tag(line, "<bundle");
        counts.constraints += count_tag(line, "<rsc_location")
            + count_tag(line, "<rsc_colocation")
            + count_tag(line, "<rsc_order")
            + count_tag(line, "<rsc_ticket");
        counts.nvpairs += count_tag(line, "<nvpair");
        counts.ops += count_tag(line, "<op ") + count_tag(line, "<op>");
        counts.attribute_blocks +=
            count_tag(line, "<meta_attributes") + count_tag(line, "<instance_attributes");
        if counts.cib > 0
            || counts.primitives > 0
            || counts.nodes > 0
            || counts.containers > 0
            || counts.constraints > 0
            || counts.nvpairs > 0
            || counts.ops > 0
            || line.starts_with("<configuration")
            || line.starts_with("<crm_config")
            || line.starts_with("<resources")
            || line.starts_with("<constraints")
        {
            saw_any = true;
        }
    }
    if !saw_any {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"<?xml version="1.0"?>
<cib crm_feature_set="3.19.0" validate-with="pacemaker-3.9" epoch="12" num_updates="4">
  <configuration>
    <crm_config>
      <cluster_property_set id="cib-bootstrap-options">
        <nvpair name="stonith-enabled" value="false" id="cib-bootstrap-options-stonith-enabled"/>
        <nvpair name="no-quorum-policy" value="ignore" id="cib-bootstrap-options-no-quorum-policy"/>
      </cluster_property_set>
    </crm_config>
    <nodes>
      <node id="1" uname="node1"/>
      <node id="2" uname="node2"/>
    </nodes>
    <resources>
      <primitive id="vip" class="ocf" provider="heartbeat" type="IPaddr2">
        <instance_attributes id="vip-instance_attributes">
          <nvpair name="ip" value="192.168.1.100" id="vip-instance_attributes-ip"/>
        </instance_attributes>
        <operations>
          <op name="monitor" interval="30s" id="vip-monitor-30s"/>
        </operations>
      </primitive>
      <primitive id="web" class="systemd" type="httpd"/>
      <clone id="ping-clone">
        <primitive id="ping" class="ocf" provider="pacemaker" type="ping"/>
      </clone>
    </resources>
    <constraints>
      <rsc_location id="loc-vip-node1" rsc="vip" node="node1" score="200"/>
      <rsc_colocation id="colo-web-vip" rsc="web" with-rsc="vip" score="INFINITY"/>
      <rsc_order id="ord-vip-web" first="vip" then="web"/>
    </constraints>
  </configuration>
</cib>
"#;

    #[test]
    fn detects_cib() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.cib, 1);
        assert_eq!(c.nodes, 2);
        assert_eq!(c.primitives, 3);
        assert_eq!(c.containers, 1);
        assert_eq!(c.constraints, 3);
        assert_eq!(c.nvpairs, 3);
        assert_eq!(c.ops, 1);
        assert_eq!(c.attribute_blocks, 1);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(b"<html><body/></html>"));
        assert!(!detect(b"plain text"));
    }
}
