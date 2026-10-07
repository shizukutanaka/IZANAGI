//! `amanda.conf` (AMANDA backup) 検出モジュール。
//!
//! amanda.conf は `key "value"`/`key value` 形式のグローバル設定と
//! `define <type> <name> { ... }` ブロック(dumptype/tapetype/
//! changer/interface/application-tool/holdingdisk/script 等)で
//! 構成される。主要キー: `org`/`dumpuser`/`mailto`/`dumpcycle`/
//! `runspercycle`/`tapecycle`/`runtapes`/`tapedev`/`tpchanger`/
//! `changerfile`/`changerdev`/`infofile`/`logdir`/`indexdir`/
//! `tapelist`/`disklist`/`bumpsize`/`bumpdays`/`bumpmult`/
//! `bumppercent`/`netusage`/`inparallel`/`maxdumps`/`dumporder`/
//! `etimeout`/`dtimeout`/`ctimeout`/`uidump`/`label_new_tapes`/
//! `autolabel`/`labelstr`/`amrecover_do_fsf`/`device_property`/
//! `debug_*`/`flush-threshold-*`/`taperflush`/`reserve`/`usetimeout`/
//! `connecttimeout`/`reqtimeout`/`timeout`/`amanda_user`/`amanda_group`
//! 等。
//!
//! ```
//! let b = b"org \"MyConfig\"\n\
//!           dumpuser \"amandabackup\"\n\
//!           mailto \"root\"\n\
//!           dumpcycle 7 days\n\
//!           runspercycle 7\n\
//!           tapecycle 15 tapes\n\
//!           tapetype \"HP-DAT\" { file-pad }\n";
//! let c = izanagi_kit::amandaconf::parse(b);
//! assert!(izanagi_kit::amandaconf::detect(b));
//! assert_eq!(c.keys, 7);
//! ```

const KEYS: &[&str] = &[
    "amanda_group",
    "amanda_user",
    "amrecover_changer",
    "amrecover_check_label",
    "amrecover_do_fsf",
    "auth",
    "autoflush",
    "autolabel",
    "bsu_tcp",
    "bsu_udp",
    "bumpdays",
    "bumpmult",
    "bumppercent",
    "bumpsize",
    "changerdev",
    "changerfile",
    "columnspec",
    "connecttimeout",
    "ctimeout",
    "debug_amidxtaped",
    "debug_auth",
    "debug_backup",
    "debug_chunker",
    "debug_client",
    "debug_dumper",
    "debug_event",
    "debug_holding",
    "debug_planner",
    "debug_script",
    "debug_selfcheck",
    "debug_sendsize",
    "debug_taper",
    "device_output_buffer_size",
    "device_property",
    "diskfile",
    "disklist",
    "displayunit",
    "dtimeout",
    "dumporder",
    "dumpcycle",
    "dumpuser",
    "etimeout",
    "flush-threshold-dumped",
    "flush-threshold-scheduled",
    "holdingdisk",
    "indexdir",
    "infofile",
    "inparallel",
    "interactivity",
    "krb5keytab",
    "krb5principal",
    "label_new_tapes",
    "labelstr",
    "logdir",
    "mailto",
    "maxdumps",
    "netusage",
    "org",
    "plugin",
    "reqtimeout",
    "reserve",
    "reserved-csv",
    "restore-client",
    "runprinter",
    "runspercycle",
    "runtapes",
    "script_timeout",
    "send-amreport",
    "sendbackup",
    "sendbackup-dump",
    "tapecycle",
    "tapedev",
    "tapelist",
    "tapetype",
    "taperflush",
    "timeout",
    "toomanydumps",
    "tpchanger",
    "uidump",
    "unit_divisor",
    "use_changer",
    "usetimeout",
];

const DEFINE_TYPES: &[&str] = &[
    "application-tool",
    "changer",
    "dumptype",
    "holdingdisk",
    "interface",
    "interactivity",
    "script",
    "tapetype",
];

fn is_define(t: &str) -> bool {
    t.starts_with("define ") && DEFINE_TYPES.iter().any(|k| t[7..].starts_with(k))
}

fn is_key(t: &str) -> bool {
    let w = t.split_whitespace().next().unwrap_or("");
    KEYS.contains(&w) || w.starts_with("debug_") || w.starts_with("flush-threshold")
}

/// `b` が amanda.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut defines = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_define(tr) {
            defines += 1;
        } else if is_key(tr) {
            keys += 1;
        }
    }
    defines >= 1 || keys >= 3
}

/// amanda.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct AmandaConf {
    /// 既知キー行数。
    pub keys: usize,
    /// `define` ブロック数。
    pub defines: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を amanda.conf として統計する。
pub fn parse(b: &[u8]) -> AmandaConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = AmandaConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_define(tr) {
            c.defines += 1;
        } else if is_key(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"org \"x\"\ndumpuser \"u\"\nmailto \"m\"\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_define() {
        let b = b"define tapetype DAT { comment \"x\" }\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.defines, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"org x\ndumpuser y\n"));
        assert!(!detect(b"define foo bar {\n}\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
