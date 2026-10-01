//! Ceph `ceph.conf` パーサ。
//!
//! `[global]`/`[mon]`/`[osd]`/`[mds]`/`[mgr]`/`[client]`/`[osd.0]` 系セクションと
//! `fsid`/`mon host`/`public network`/`osd journal` 等のアンダースコア系既知キーを計数する。
//!
//! ```
//! use izanagi_kit::cephconf;
//! let conf = b"[global]\nfsid = 5e3f2b7e-0000-4b00-a111-2222aaaabbbb\nmon host = 10.0.0.1\npublic_network = 10.0.0.0/24\n";
//! assert!(cephconf::detect(conf));
//! let c = cephconf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 1);
//! assert_eq!(c.known_keys, 3);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション数。
    pub sections: usize,
    /// 既知セクション数 (daemon 系も含む)。
    pub known_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_SECTIONS: &[&str] = &[
    "global",
    "mon",
    "osd",
    "mds",
    "mgr",
    "client",
    "radosgw",
    "rgw",
    "mdsfs",
    "mds_cache",
];

const KNOWN_KEYS: &[&str] = &[
    "fsid",
    "mon_initial_members",
    "mon_host",
    "mon host",
    "public_network",
    "cluster_network",
    "auth_cluster_required",
    "auth_service_required",
    "auth_client_required",
    "osd_pool_default_size",
    "osd_pool_default_min_size",
    "osd_pool_default_pg_num",
    "osd_pool_default_pgp_num",
    "osd_crush_chooseleaf_type",
    "osd_journal",
    "osd_journal_size",
    "osd_max_object_name_len",
    "osd_max_object_namespace_len",
    "osd_memory_target",
    "osd_mkfs_type",
    "osd_op_threads",
    "mon_osd_min_down_reporters",
    "mon_osd_report_timeout",
    "mon_data_avail_warn",
    "ms_bind_ipv6",
    "keyring",
    "key",
    "admin_socket",
    "chdir",
    "log_file",
    "max_open_files",
    "rbd_cache",
    "rbd_default_features",
    "rgw_frontends",
    "rgw_dns_name",
    "rgw_socket_path",
    "debug_ms",
    "debug_osd",
    "debug_mon",
    "debug_rgw",
    "mgr_modules",
    "mon_pg_warn_max_per_osd",
    "bluestore_block_size",
    "bluefs_buffered_io",
];

/// `ceph.conf` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => (c.known_sections >= 1 && c.known_keys >= 2) || c.known_keys >= 3,
        None => false,
    }
}

/// セクション名が既知 (daemon 指定 `osd.0`/`mon.a`/`client.foo`/`rgw.zone` 含む) か。
fn known_section(name: &str) -> bool {
    let base = name.split(&['.', ' ', '\t'][..]).next().unwrap_or(name);
    KNOWN_SECTIONS.contains(&base) || name.starts_with("client ")
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            c.sections += 1;
            let name = t[1..t.len() - 1].trim().to_ascii_lowercase();
            if known_section(&name) {
                c.known_sections += 1;
            }
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim().to_ascii_lowercase();
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b' ' | b'.'))
        {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key.as_str()) {
            c.known_keys += 1;
        }
    }
    (c.entries > 0 || c.sections > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[global]\nfsid = 5e3f2b7e-0000-4b00-a111-2222aaaabbbb\nmon_initial_members = a,b,c\nmon_host = 10.0.0.1,10.0.0.2\npublic_network = 10.0.0.0/24\nauth_cluster_required = cephx\n[osd]\nosd_journal_size = 512\nosd_pool_default_size = 3\n[mon.a]\nmon addr = 10.0.0.1:6789\n";

    #[test]
    fn detects_cephconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.entries, 8);
        assert_eq!(c.known_keys, 7);
    }

    #[test]
    fn rejects_generic_ini() {
        let ini = b"[a]\nx = 1\ny = 2\n[b]\nz = 3\nw = 4\n";
        assert!(!detect(ini));
    }
}
