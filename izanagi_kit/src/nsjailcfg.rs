//! `nsjail.cfg` (nsjail, protobuf text) 検出モジュール。
//!
//! nsjail の設定は protobuf テキスト形式で、`name:`/`description:`/
//! `mode:`/`chroot:`/`hostname:`/`cwd:`/`time_limit:`/`daemon:`/
//! `verbose:`/`keep_env:`/`keep_caps:`/`user:`/`group:`/`uid:`/`gid:`/
//! `exec_bin { path: ... arg: ... }`/`uidmap { inside_id: ...
//! outside_id: ... count: ... }`/`gidmap`/`mount { src: dst: ... }`/
//! `cap:`/`rlimit_as:`/`rlimit_core:`/`rlimit_cpu:`/`rlimit_fsize:`/
//! `rlimit_nofile:`/`rlimit_nproc:`/`rlimit_stack:`/`cgroup_*:`/
//! `clone_new*`/`iface_no_lo`/`disable_proc`/`disable_no_new_privs`/
//! `seccomp_string`/`seccomp_log`/`kafel_file`/`log_file`/`log_level`/
//! `envar`/`skip_readdir`/`safe_env`/`bind_proc`/`detect_sandboxing`/
//! `forward_signals`/`max_cpus`/`nice_level`/`max_conns_per_ip`/
//! `mount_proc`/`tmpfsmount`/`is_root`/`pivot_root_only`
//! 等のフィールドで構成される。
//!
//! ```
//! let b = b"name: \"test\"\n\
//!           mode: ONCE\n\
//!           chroot: \"/chroot\"\n\
//!           exec_bin {\n  path: \"/bin/sh\"\n  arg: \"-c\"\n  arg: \"id\"\n}\n\
//!           uidmap { inside_id: \"0\" outside_id: \"1000\" count: 1 }\n\
//!           rlimit_as: 512\n";
//! let c = izanagi_kit::nsjailcfg::parse(b);
//! assert!(izanagi_kit::nsjailcfg::detect(b));
//! assert_eq!(c.blocks, 2);
//! ```

const KEYS: &[&str] = &[
    "bind_proc",
    "cap",
    "cgroup_cpu_max",
    "cgroup_mem_max",
    "cgroup_net_cls_classid",
    "cgroup_pids_max",
    "chroot",
    "clone_newcgroup",
    "clone_newipc",
    "clone_newnet",
    "clone_newns",
    "clone_newpid",
    "clone_newtime",
    "clone_newuser",
    "clone_newuts",
    "cwd",
    "daemon",
    "detect_sandboxing",
    "description",
    "disable_no_new_privs",
    "disable_proc",
    "disable_sandboxing",
    "envar",
    "exec_bin",
    "forward_signals",
    "gid",
    "gidmap",
    "hostname",
    "iface_no_lo",
    "is_root",
    "kafel_file",
    "keep_caps",
    "keep_env",
    "log_file",
    "log_level",
    "max_conns_per_ip",
    "max_cpus",
    "mode",
    "mount",
    "mount_proc",
    "name",
    "nice_level",
    "pivot_root_only",
    "process_caps",
    "rlimit_as",
    "rlimit_as_type",
    "rlimit_core",
    "rlimit_core_type",
    "rlimit_cpu",
    "rlimit_cpu_type",
    "rlimit_fsize",
    "rlimit_fsize_type",
    "rlimit_nofile",
    "rlimit_nofile_type",
    "rlimit_nproc",
    "rlimit_nproc_type",
    "rlimit_stack",
    "rlimit_stack_type",
    "safe_env",
    "seccomp_log",
    "seccomp_string",
    "skip_readdir",
    "time_limit",
    "tmpfsmount",
    "uid",
    "uidmap",
    "user",
    "verbose",
];

const BLOCKS: &[&str] = &["exec_bin", "mount", "uidmap", "gidmap", "iface", "mac"];

fn key_of(t: &str) -> &str {
    t.split([':', ' ', '{']).next().unwrap_or("")
}

/// `b` が nsjail.cfg に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut blocks = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr == "}" {
            continue;
        }
        let k = key_of(tr);
        if KEYS.contains(&k) {
            keys += 1;
        }
        if BLOCKS.contains(&k) && tr.contains('{') {
            blocks += 1;
        }
    }
    blocks >= 1 || keys >= 4
}

/// nsjail.cfg の統計。
#[derive(Debug, Default, Clone)]
pub struct NsjailCfg {
    /// 既知フィールド行数。
    pub keys: usize,
    /// ブロック開始行数 (exec_bin/mount/uidmap 等)。
    pub blocks: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を nsjail.cfg として統計する。
pub fn parse(b: &[u8]) -> NsjailCfg {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = NsjailCfg::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr == "}" {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let k = key_of(tr);
        if KEYS.contains(&k) {
            c.keys += 1;
        }
        if BLOCKS.contains(&k) && tr.contains('{') {
            c.blocks += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"exec_bin {\n  path: \"/bin/sh\"\n}\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.blocks, 1);
    }

    #[test]
    fn detects_keys() {
        let b = b"name: \"x\"\nmode: ONCE\nchroot: \"/c\"\ntime_limit: 60\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"name: \"x\"\nmode: ONCE\n"));
        assert!(!detect(b"key=value\nfoo=bar\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
        assert!(!detect(
            b"package foo\noption go_package = \"x\"\nsyntax = \"proto3\"\n"
        ));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
