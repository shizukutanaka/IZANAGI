//! RHEL Cluster Suite (rgmanager/cman) `cluster.conf` XML の解析。
//!
//! `<cluster name="" config_version="">` ルートと `<clusternodes>`/
//! `<clusternode name="" nodeid="" votes="">`/`<fence>`/`<fencedevices>`/
//! `<fencedevice agent="">`/`<cman>`/`<totem>`/`<rm>`/`<failoverdomains>`/
//! `<failoverdomain>`/`<failoverdomainnode>`/`<resources>`/`<ip>`/`<fs>`/
//! `<script>`/`<service>`/`<netfs>`/`<nfs>`/`<nfsexport>`/`<nfsclient>`/
//! `<clusterfs>`/`<apache>`/`<mysql>`/`<samba>`/`<openvpn>`/`<oracledb>`/
//! `<postgres>`/`<tomcat>`/`<lvm>`/`<logging>` 等を検出し、
//! ノード・フェンス・フェイルオーバ・リソースの数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::clusterconf;
//!
//! let text = br#"<cluster name="mycluster" config_version="1">
//!   <clusternodes>
//!     <clusternode name="node1" nodeid="1" votes="1"/>
//!   </clusternodes>
//! </cluster>
//! "#;
//!
//! assert!(clusterconf::detect(text));
//! let c = clusterconf::parse(text).unwrap();
//! assert_eq!(c.nodes, 1);
//! ```

/// cluster.conf の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<cluster` ルート要素数。
    pub root: usize,
    /// `<clusternode` 宣言数 (`<clusternodes>` 自体は除外)。
    pub nodes: usize,
    /// `<fencedevice` 宣言数 (`<fencedevices>` 自体は除外)。
    pub fencedevices: usize,
    /// `<failoverdomain` 宣言数 (`<failoverdomains>` 自体は除外)。
    pub failoverdomains: usize,
    /// `<failoverdomainnode` 宣言数。
    pub failoverdomainnodes: usize,
    /// `<service`/`apache`/`mysql`/`postgres`/`tomcat`/`samba`/`openvpn`/`oracledb`/`named`/`nfsserver`/`postfix`/`sendmail`/`squid`/`xen`/`vm`/`virtual`/`lvm`/`HA` サービス系要素数。
    pub services: usize,
    /// `<ip`/`<fs`/`<netfs`/`<nfs`/`<nfsexport`/`<nfsclient`/`<clusterfs`/`<script`/`<smb`/`<mount` リソース要素数。
    pub resources: usize,
    /// `<cman`/`<totem`/`<fence`/`logging`/`<rm`/`<multicast`/`<key`/`quorum`/`dlm`/`<group`/`compat`/`gfs_controld`/`fence_daemon`/`fence_xvmd`/`event` 構造・設定要素数。
    pub structural: usize,
}

fn count_tag(line: &str, tag: &str) -> usize {
    line.matches(tag).count()
}

/// `b` が cluster.conf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.root >= 1 && (c.nodes >= 1 || c.fencedevices >= 1 || c.failoverdomains >= 1)
}

/// cluster.conf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        root: 0,
        nodes: 0,
        fencedevices: 0,
        failoverdomains: 0,
        failoverdomainnodes: 0,
        services: 0,
        resources: 0,
        structural: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with("<?xml")
            || line.starts_with("<!")
            || line.starts_with("<!--")
        {
            continue;
        }
        counts.root += count_tag(line, "<cluster ");
        counts.nodes += count_tag(line, "<clusternode ");
        // `<clusternodes>`(複数形ブロック)は `<clusternode ` で除外済み。
        counts.fencedevices += count_tag(line, "<fencedevice ");
        counts.failoverdomains += count_tag(line, "<failoverdomain ");
        counts.failoverdomainnodes += count_tag(line, "<failoverdomainnode");
        counts.services += count_tag(line, "<service")
            + count_tag(line, "<apache")
            + count_tag(line, "<mysql")
            + count_tag(line, "<postgres")
            + count_tag(line, "<tomcat")
            + count_tag(line, "<samba")
            + count_tag(line, "<openvpn")
            + count_tag(line, "<oracledb")
            + count_tag(line, "<named")
            + count_tag(line, "<xen")
            + count_tag(line, "<vm ")
            + count_tag(line, "<vm>");
        counts.resources += count_tag(line, "<ip ")
            + count_tag(line, "<fs ")
            + count_tag(line, "<netfs")
            + count_tag(line, "<nfs ")
            + count_tag(line, "<nfs>")
            + count_tag(line, "<nfsexport")
            + count_tag(line, "<nfsclient")
            + count_tag(line, "<clusterfs")
            + count_tag(line, "<script")
            + count_tag(line, "<smb")
            + count_tag(line, "<mount");
        counts.structural += count_tag(line, "<cman")
            + count_tag(line, "<totem")
            + count_tag(line, "<fence")
            + count_tag(line, "<fence_daemon")
            + count_tag(line, "<fence_xvmd")
            + count_tag(line, "<logging")
            + count_tag(line, "<rm")
            + count_tag(line, "<multicast")
            + count_tag(line, "<key")
            + count_tag(line, "<quorum")
            + count_tag(line, "<dlm")
            + count_tag(line, "<group")
            + count_tag(line, "<event")
            + count_tag(line, "<clusternodes")
            + count_tag(line, "<fencedevices")
            + count_tag(line, "<failoverdomains")
            + count_tag(line, "<resources")
            + count_tag(line, "<gfs_controld")
            + count_tag(line, "<fencepost")
            + count_tag(line, "<fence_xvm");
        if line.contains('<') {
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
<cluster name="production" config_version="23">
  <fence_daemon post_fail_delay="0" post_join_delay="3" clean_start="0"/>
  <clusternodes>
    <clusternode name="node1" nodeid="1" votes="1">
      <fence>
        <method name="1">
          <device name="ipmi1" ipaddr="192.168.1.11"/>
        </method>
      </fence>
    </clusternode>
    <clusternode name="node2" nodeid="2" votes="1">
      <fence>
        <method name="1">
          <device name="ipmi2" ipaddr="192.168.1.12"/>
        </method>
      </fence>
    </clusternode>
  </clusternodes>
  <cman expected_votes="1" two_node="1"/>
  <fencedevices>
    <fencedevice agent="fence_ipmilan" name="ipmi1" ipaddr="192.168.1.11" login="admin" passwd="secret"/>
    <fencedevice agent="fence_ipmilan" name="ipmi2" ipaddr="192.168.1.12" login="admin" passwd="secret"/>
  </fencedevices>
  <rm>
    <failoverdomains>
      <failoverdomain name="dom1" ordered="1" restricted="1">
        <failoverdomainnode name="node1" priority="1"/>
        <failoverdomainnode name="node2" priority="2"/>
      </failoverdomain>
    </failoverdomains>
    <resources>
      <ip address="192.168.1.100" monitor_link="1"/>
      <fs device="/dev/sda2" mountpoint="/data" fstype="ext3"/>
      <script file="/etc/init.d/httpd" name="web"/>
    </resources>
    <service name="websvc" domain="dom1" autostart="1">
      <ip ref="192.168.1.100"/>
      <fs ref="webdata"/>
      <script ref="web"/>
    </service>
  </rm>
  <logging debug="off"/>
</cluster>
"#;

    #[test]
    fn detects_clusterconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.root, 1);
        assert_eq!(c.nodes, 2);
        assert_eq!(c.fencedevices, 2);
        assert_eq!(c.failoverdomains, 1);
        assert_eq!(c.failoverdomainnodes, 2);
        assert_eq!(c.resources, 6);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(b"<html><body/></html>"));
        assert!(!detect(b"plain text"));
    }
}
