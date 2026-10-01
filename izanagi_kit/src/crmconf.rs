//! Pacemaker `crm`/`crmsh` シェルスクリプトの解析。
//!
//! `configure`/`cib`/`primitive <id> <class>:<provider>:<type>`/`group`/`clone`/
//! `ms`/`master`/`location`/`colocation`/`order`/`property`/`rsc_defaults`/
//! `op_defaults`/`node`/`monitor`/`rsc_template`/`commit`/`end`/`verify` 等の
//! コマンド行を検出し、各文種別の数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::crmconf;
//!
//! let text = br#"primitive vip IPaddr2 params ip=192.168.1.10 \
//!   op monitor interval=30s
//! location loc-vip vip 200: node1
//! "#;
//!
//! assert!(crmconf::detect(text));
//! let c = crmconf::parse(text).unwrap();
//! assert_eq!(c.primitives, 1);
//! ```

/// crm 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `primitive <id> …` 文数。
    pub primitives: usize,
    /// `group`/`clone`/`ms`/`master`/`rsc_template`/`bundle` 文数。
    pub containers: usize,
    /// `location`/`colocation`/`order`/`rsc_ticket` 制約文数。
    pub constraints: usize,
    /// `property`/`rsc_defaults`/`op_defaults`/`node`/`fencing_topology` 文数。
    pub properties: usize,
    /// `op <type>`/文内 ` op ` 操作句の数。
    pub ops: usize,
    /// `configure`/`cib`/`commit`/`end`/`verify`/`show`/`erase`/`xml`/`status` 等管理文数。
    pub management: usize,
    /// 継続行 (`\` 終わり) 数。
    pub continuations: usize,
}

/// `b` が crmsh スクリプトらしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    (c.primitives >= 1 || c.constraints >= 1) && c.ops + c.properties >= 1
        || c.primitives >= 1 && c.management >= 1
}

/// crmsh スクリプトを解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        primitives: 0,
        containers: 0,
        constraints: 0,
        properties: 0,
        ops: 0,
        management: 0,
        continuations: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.as_bytes().last() == Some(&92) {
            counts.continuations += 1;
        }
        let mut words = line.split_whitespace();
        let Some(first) = words.next() else {
            continue;
        };
        // `configure <文>`/`cib <文>` は第2語で再ディスパッチ。
        let eff = if first == "configure" {
            words.next().unwrap_or("")
        } else {
            first
        };
        match eff {
            "primitive" => {
                counts.primitives += 1;
                saw_any = true;
            }
            "group" | "clone" | "ms" | "master" | "rsc_template" | "bundle" => {
                counts.containers += 1;
                saw_any = true;
            }
            "location" | "colocation" | "order" | "rsc_ticket" => {
                counts.constraints += 1;
                saw_any = true;
            }
            "property" | "rsc_defaults" | "op_defaults" | "node" | "fencing_topology" | "rsc"
            | "acl" | "acl_target" | "acl_group" | "role" | "user" | "tag" | "alert" => {
                counts.properties += 1;
                saw_any = true;
            }
            "op" => {
                counts.ops += 1;
                saw_any = true;
            }
            "configure" | "cib" | "commit" | "end" | "verify" | "show" | "erase" | "xml"
            | "status" | "cd" | "quit" | "exit" | "bye" | "help" | "modgroup" | "modsubnet"
            | "up" | "down" | "resource" | "constraint" | "options" | "assistance"
            | "maintenance" | "ra" | "report" | "script" | "site" | "corosync" | "cluster"
            | "history" => {
                counts.management += 1;
                saw_any = true;
            }
            _ => {}
        }
        // 行内の ` op <type>` 句を追加計上 (primitive 文の後半)。
        if eff != "op" && line.contains(" op ") {
            counts.ops += line.matches(" op ").count();
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

    const SAMPLE: &[u8] = br#"# crmsh script
cib new production
configure erase
configure primitive vip ocf:heartbeat:IPaddr2 \
    params ip=192.168.1.100 cidr_netmask=24 \
    op monitor interval=30s timeout=60s
configure primitive web systemd:httpd \
    op start interval=0 timeout=90 \
    op monitor interval=10
configure clone ping-clone ping
configure group webgroup vip web
configure location loc-vip-node1 vip 200: node1
configure colocation colo-web-vip inf: web vip
configure order ord-vip-web Mandatory: vip web
configure property stonith-enabled=false
configure rsc_defaults resource-stickiness=200
configure op_defaults timeout=120
commit
"#;

    #[test]
    fn detects_crm() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.primitives, 2);
        assert_eq!(c.containers, 2);
        assert_eq!(c.constraints, 3);
        assert_eq!(c.properties, 3);
        assert_eq!(c.ops, 3);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"section { key = value }"));
    }
}
