//! NFS `/etc/exports` パーサ。
//!
//! `/path host(オプション)` 形式のエントリを計数し、`rw`/`sync`/`no_subtree_check`/
//! `root_squash`/`all_squash`/`fsid`/`sec=` 等既知オプションを数える。
//!
//! ```
//! use izanagi_kit::nfsexports;
//! let exp = b"/srv/nfs 192.168.0.0/24(rw,sync,no_subtree_check)\n/data *(ro,root_squash,fsid=0)\n";
//! assert!(nfsexports::detect(exp));
//! let c = nfsexports::parse(exp).unwrap();
//! assert_eq!(c.exports, 2);
//! assert_eq!(c.known_options, 6);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// エクスポート行数。
    pub exports: usize,
    /// `(...)` オプション群を持つ行数。
    pub option_blocks: usize,
    /// 既知オプション数 (合計)。
    pub known_options: usize,
    /// CIDR/`*.`/`@group`/`host` 指定を持つホスト数。
    pub hosts: usize,
}

const KNOWN_OPTIONS: &[&str] = &[
    "rw",
    "ro",
    "sync",
    "async",
    "secure",
    "insecure",
    "wdelay",
    "no_wdelay",
    "hide",
    "nohide",
    "crossmnt",
    "subtree_check",
    "no_subtree_check",
    "insecure_locks",
    "mountpoint",
    "mp",
    "fsid",
    "refer",
    "replicas",
    "root_squash",
    "no_root_squash",
    "all_squash",
    "no_all_squash",
    "anonuid",
    "anongid",
    "sec",
    "pnfs",
    "no_pnfs",
    "security_label",
    "no_acl",
    "mountpoint",
];

/// `/etc/exports` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => c.exports >= 1 && c.known_options >= 1,
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
        exports: 0,
        option_blocks: 0,
        known_options: 0,
        hosts: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let t = t.trim_end_matches('\\').trim_end();
        // パスは絶対パス必須
        let Some(sp) = t.find([' ', '\t']) else {
            continue;
        };
        let path = t[..sp].trim();
        if !path.starts_with('/') && !path.starts_with('"') {
            continue;
        }
        c.exports += 1;
        let rest = t[sp..].trim();
        // ホスト(オプション) 群
        for tok in rest.split_whitespace() {
            let (host, opts) = match tok.split_once('(') {
                Some((h, o)) => (h, o),
                None => (tok, ""),
            };
            let opts = opts.trim_end_matches(')');
            if host.is_empty() {
                continue;
            }
            c.hosts += 1;
            if !opts.is_empty() {
                c.option_blocks += 1;
                for opt in opts.split(',') {
                    let opt = opt.trim();
                    let name = opt.split('=').next().unwrap_or(opt);
                    if KNOWN_OPTIONS.contains(&name) {
                        c.known_options += 1;
                    }
                }
            }
        }
    }
    (c.exports > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# exports\n/srv/nfs 192.168.0.0/24(rw,sync,no_subtree_check)\n/data *(ro,root_squash,fsid=0)\n/home bob(rw,sync) alice(ro,root_squash)\n";

    #[test]
    fn detects_exports() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.exports, 3);
        assert_eq!(c.hosts, 4);
        assert_eq!(c.option_blocks, 4);
        assert_eq!(c.known_options, 10);
    }

    #[test]
    fn detects_single_known_option() {
        // `/srv host(ro)` is a valid minimal exports entry; the previous
        // `known_options >= 2` gate missed it.
        assert!(detect(b"/srv host(ro)\n"));
    }

    #[test]
    fn rejects_etc_fstab() {
        let fstab = b"/dev/sda1 / ext4 defaults 0 1\nproc /proc proc defaults 0 0\n";
        assert!(!detect(fstab));
    }
}
