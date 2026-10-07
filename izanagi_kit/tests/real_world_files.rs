//! Real-world corpus regression test.
//!
//! Unit fixtures are authored by the same person who wrote each detector, so
//! they attest only that the detector matches what its author imagined. This
//! test instead runs every registered detector over configuration files
//! taken from real projects (nginx, git, mathiasbynens' dotfiles, unbound,
//! BIND-adjacent tooling, OpenWrt, …) and asserts two invariants:
//!
//! 1. recall on real data: the file's own format must be detected, and
//! 2. precision on real data: the number of *foreign* detectors firing on
//!    each file stays at or below the recorded ceiling.
//!
//! The ceilings are snapshots of the current counts — tightening detection
//! lowers them freely; a regression trips the assert.

use izanagi_kit::DETECTORS;

/// One corpus entry: (slug, human label, bytes, detector that must hit,
/// foreign-hit ceiling). `expected` is `None` for files that legitimately
/// match nothing (e.g. a comments-only `exports` file).
type Case = (
    &'static str,
    &'static str,
    &'static [u8],
    Option<&'static str>,
    usize,
);

const FILES: &[Case] = &[
    (
        "nginx",
        "nginx.conf",
        include_bytes!("../realfiles/nginx"),
        Some("nginx"),
        20,
    ),
    (
        "gitignore",
        ".gitignore (python)",
        include_bytes!("../realfiles/gitignore"),
        Some("gitignore"),
        13,
    ),
    (
        "gitattributes",
        ".gitattributes (linux)",
        include_bytes!("../realfiles/gitattributes"),
        Some("gitattributes"),
        8,
    ),
    (
        "gitconfig",
        ".gitconfig (mathiasbynens)",
        include_bytes!("../realfiles/gitconfig"),
        Some("gitconfig"),
        32,
    ),
    (
        "vimrc",
        ".vimrc (mathiasbynens)",
        include_bytes!("../realfiles/vimrc"),
        Some("vimrc"),
        33,
    ),
    (
        "bashrc",
        ".bashrc delegation one-liner",
        include_bytes!("../realfiles/bashrc"),
        Some("bashrc"),
        4,
    ),
    (
        "zshrc",
        "ohmyzsh zshrc template",
        include_bytes!("../realfiles/zshrc"),
        Some("zshrc"),
        15,
    ),
    (
        "requirements",
        "requirements-dev.txt (requests)",
        include_bytes!("../realfiles/requirements"),
        Some("requirements"),
        3,
    ),
    (
        "editorconfig",
        ".editorconfig (editorconfig-vim)",
        include_bytes!("../realfiles/editorconfig"),
        Some("editorconfig"),
        10,
    ),
    (
        "apacheconf",
        "httpd.conf template",
        include_bytes!("../realfiles/apacheconf"),
        Some("apacheconf"),
        25,
    ),
    (
        "unbound",
        "unbound example.conf",
        include_bytes!("../realfiles/unbound"),
        Some("unbound"),
        43,
    ),
    (
        "chronyconf",
        "chrony.conf example",
        include_bytes!("../realfiles/chronyconf"),
        Some("chronyconf"),
        8,
    ),
    (
        "krb5conf",
        "krb5.conf",
        include_bytes!("../realfiles/krb5conf"),
        Some("krb5conf"),
        14,
    ),
    (
        "haproxy",
        "haproxy content-switch sample",
        include_bytes!("../realfiles/haproxy"),
        Some("haproxy"),
        35,
    ),
    (
        "sudoers",
        "sudoers template",
        include_bytes!("../realfiles/sudoers"),
        Some("sudoers"),
        20,
    ),
    (
        "sysctlconf",
        "systemd 50-default.conf",
        include_bytes!("../realfiles/sysctlconf"),
        Some("sysctlconf"),
        12,
    ),
    (
        "rsyslogd",
        "rsyslog.conf",
        include_bytes!("../realfiles/rsyslogd"),
        Some("rsyslogd"),
        12,
    ),
    (
        "lighttpd",
        "lighttpd.conf",
        include_bytes!("../realfiles/lighttpd"),
        Some("lighttpd"),
        16,
    ),
    (
        "wpasupplicant",
        "wpa_supplicant.conf",
        include_bytes!("../realfiles/wpasupplicant"),
        Some("wpasupplicant"),
        5,
    ),
    (
        "nfsexports_comments",
        "openwrt exports (comments only)",
        include_bytes!("../realfiles/nfsexports_comments"),
        None,
        5,
    ),
];

#[test]
fn real_files_hit_their_own_detector() {
    for (slug, label, bytes, expected, _) in FILES {
        let hits: Vec<&str> = DETECTORS
            .iter()
            .filter(|(_, detect)| detect(bytes))
            .map(|(name, _)| *name)
            .collect();
        if let Some(exp) = expected {
            assert!(
                hits.contains(exp),
                "{label}: intended detector `{exp}` did not fire; hits = {hits:?}"
            );
        }
        let _ = slug;
    }
}

#[test]
fn foreign_hits_stay_under_ceiling() {
    for (_, label, bytes, expected, ceiling) in FILES {
        let foreign: Vec<&str> = DETECTORS
            .iter()
            .filter(|(_, detect)| detect(bytes))
            .map(|(name, _)| *name)
            .filter(|name| Some(*name) != *expected)
            .collect();
        assert!(
            foreign.len() <= *ceiling,
            "{label}: {} foreign hits (ceiling {ceiling}): {foreign:?}",
            foreign.len()
        );
    }
}
