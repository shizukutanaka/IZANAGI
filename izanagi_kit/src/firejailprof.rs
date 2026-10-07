//! Firejail `.profile` 検出モジュール。
//!
//! Firejail プロファイルは1行1ディレクティブで、`include`/
//! `blacklist`/`noblacklist`/`whitelist`/`nowhitelist`/`read-only`/
//! `read-write`/`noexec`/`private`/`private-*`/`net`/`caps.*`/
//! `seccomp`/`seccomp.*`/`nonewprivs`/`noroot`/`protocol`/`tracelog`/
//! `rlimit-*`/`env`/`shell`/`disable-mnt`/`apparmor`/`ipc-namespace`/
//! `ignore`/`mkdir`/`mkfile`/`bind`/`tmpfs`/`netfilter`/`hostname`/
//! `dns`/`name`/`description`/`version`/`join-or-start`/`netns`/
//! `x11`/`dbus-*`/`nou2f`/`novideo`/`nodvd`/`notv`/`nosound`/
//! `no3d`/`quiet`/`allow-debuggers`/`tracelog`/`writable-*`/`noautopulse`
//! 等のディレクティブで構成される。
//!
//! ```
//! let b = b"include globals.local\n\
//!           noblacklist ${HOME}/.config\n\
//!           whitelist ${HOME}/Documents\n\
//!           net none\n\
//!           nonewprivs\n\
//!           seccomp\n";
//! let c = izanagi_kit::firejailprof::parse(b);
//! assert!(izanagi_kit::firejailprof::detect(b));
//! assert_eq!(c.directives, 6);
//! ```

const DIRS: &[&str] = &[
    "allow-debuggers",
    "allusers",
    "apparmor",
    "apparmor-replace",
    "apparmor-round-trip",
    "bind",
    "blacklist",
    "caps.drop",
    "caps.keep",
    "chroot",
    "cpu",
    "dbus-log",
    "dbus-policy",
    "dbus-user",
    "dbus-user.own",
    "dbus-user.talk",
    "dbus-user.none",
    "dbus-user.broadcast",
    "dbus-user.call",
    "dbus-user.filter",
    "dbus-system",
    "deterministic-shutdown",
    "disable-mnt",
    "dns",
    "env",
    "hostname",
    "hosts-file",
    "ignore",
    "include",
    "ipc-namespace",
    "join",
    "join-filesystem",
    "join-namespace",
    "join-or-start",
    "keep-assistant",
    "keep-config-pulse",
    "keep-dev-shm",
    "keep-var-tmp",
    "machine-id",
    "mkdir",
    "mkfile",
    "name",
    "net",
    "netfilter",
    "netfilter6",
    "netns",
    "no3d",
    "noautopulse",
    "noblacklist",
    "nodbus",
    "nodvd",
    "noexec",
    "nogroups",
    "noinput",
    "nonewprivs",
    "noprinters",
    "noroot",
    "nosound",
    "notv",
    "nou2f",
    "novideo",
    "nowhitelist",
    "overlay-tmpfs",
    "private",
    "private-bin",
    "private-cache",
    "private-cwd",
    "private-dev",
    "private-etc",
    "private-home",
    "private-kde",
    "private-lib",
    "private-local",
    "private-opt",
    "private-srv",
    "private-tmp",
    "protocol",
    "quiet",
    "read-only",
    "read-write",
    "rmenv",
    "seccomp",
    "seccomp.block-secondary",
    "seccomp.drop",
    "seccomp.keep",
    "seccomp-error-action",
    "shell",
    "timeout",
    "tmpfs",
    "tracelog",
    "veth-name",
    "whitelist",
    "writable-etc",
    "writable-home",
    "writable-run-user",
    "writable-tmp",
    "writable-var",
    "writable-var-log",
    "writable-var-tmp",
    "x11",
    "xephyr-screen",
];

fn is_directive(t: &str) -> bool {
    let w = t.split_whitespace().next().unwrap_or("");
    DIRS.contains(&w) || w.starts_with("rlimit-")
}

/// `b` が firejail プロファイルに見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut dirs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_directive(tr) {
            dirs += 1;
        }
    }
    dirs >= 3
}

/// firejail プロファイルの統計。
#[derive(Debug, Default, Clone)]
pub struct FirejailProf {
    /// 既知ディレクティブ行数。
    pub directives: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を firejail プロファイルとして統計する。
pub fn parse(b: &[u8]) -> FirejailProf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = FirejailProf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_directive(tr) {
            c.directives += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"blacklist /x\nwhitelist /y\nnet none\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.directives, 3);
    }

    #[test]
    fn rlimit() {
        let b = b"rlimit-nproc 100\nrlimit-fsize 1g\nrlimit-nofile 200\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"blacklist /x\nwhitelist /y\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.directives, 0);
    }
}
