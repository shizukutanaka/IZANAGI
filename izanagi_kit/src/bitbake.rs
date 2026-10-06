//! BitBake/Yocto `.bb`/`.bbappend`/`*.conf` レシピ形式の検出と
//! 構造カウント。
//!
//! `VAR = "v"`/`VAR:append = ...`/`VAR ?= ...`/`inherit x`/
//! `include`/`require`/`BB_*`/`IMAGE_*`/`MACHINE`/`DISTRO`/
//! `PACKAGECONFIG`/`SRC_URI`/`LICENSE`/`do_*` タスク、`${...}`/
//! `${@...}` 展開、Python `def`/`python` ブロックを識別する。
//!
//! ```
//! let b = b"SUMMARY = \"demo\"\nLICENSE = \"MIT\"\nSRC_URI = \"git://x/repo\"\ninherit cmake\n";
//! assert!(izanagi_kit::bitbake::detect(b));
//! let c = izanagi_kit::bitbake::BitBake::parse(b).unwrap();
//! assert_eq!(c.assignments, 3);
//! ```

/// Parsed BitBake recipe summary.
#[derive(Debug, Clone)]
pub struct BitBake {
    /// `VAR =`/`VAR:op =`/`?=`/`??=` assignment lines.
    pub assignments: usize,
    /// `inherit`/`include`/`require`/`inherit_defer` lines.
    pub directives: usize,
    /// `do_*` task lines.
    pub tasks: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Distinctive BitBake variable prefixes.
const VAR_PREFIXES: &[&str] = &[
    "ALLOW_EMPTY",
    "AUTOREV",
    "BAD_RECOMMENDATIONS",
    "BB_",
    "BBCLASSEXTEND",
    "BBLAYERS",
    "BBMASK",
    "BBPATH",
    "BUILDDIR",
    "COMPATIBLE_",
    "CONFFILES",
    "CORE_IMAGE_",
    "DEPENDS",
    "DESCRIPTION",
    "AUTHOR",
    "BUGTRACKER",
    "DISTRO",
    "DL_DIR",
    "EXTRA_OECONF",
    "EXTRA_OEMAKE",
    "FEATURE_PACKAGES",
    "FILESEXTRAPATHS",
    "FILESPATH",
    "HOMEPAGE",
    "IMAGE_",
    "INHIBIT_",
    "INITSCRIPT_",
    "INSANE_SKIP",
    "KERNEL_",
    "LAYERDEPENDS",
    "LAYERSERIES_COMPAT",
    "LIC_FILES_CHKSUM",
    "LICENSE",
    "MACHINE",
    "MIRRORS",
    "PACKAGECONFIG",
    "PACKAGES",
    "PACKAGE_ARCH",
    "PARALLEL_MAKE",
    "PE",
    "PKG",
    "PN",
    "PR",
    "PREMIRRORS",
    "PROVIDES",
    "PV",
    "RCONFLICTS",
    "RDEPENDS",
    "RECOMMENDS",
    "REQUIRED_DISTRO_FEATURES",
    "RPROVIDES",
    "RRECOMMENDS",
    "RSUGGESTS",
    "S",
    "SECTION",
    "SRCBRANCH",
    "SRCREV",
    "SRC_URI",
    "STAGING_",
    "SUMMARY",
    "SYSTEMD_",
    "TARGET_",
    "TCLIBC",
    "TOOLCHAIN_",
    "WORKDIR",
];

fn var_line(tr: &str) -> bool {
    if tr.starts_with('#') {
        return false;
    }
    let name_part = tr.split('=').next().unwrap_or("");
    let name = name_part.trim_end_matches(['?', '+', ':', ' ', '\t', '.']);
    let base = name.split(':').next().unwrap_or("");
    VAR_PREFIXES.iter().any(|p| base.starts_with(p)) && tr.contains('=')
}

fn directive_line(tr: &str) -> bool {
    tr.starts_with("inherit ")
        || tr.starts_with("include ")
        || tr.starts_with("require ")
        || tr.starts_with("inherit_defer ")
        || tr.starts_with("export_")
        || tr.starts_with("addtask ")
        || tr.starts_with("deltask ")
        || tr.starts_with("EXPORT_")
        || tr.starts_with("INC_DIR")
}

fn task_line(tr: &str) -> bool {
    tr.starts_with("do_")
        && (tr.ends_with('{')
            || tr.contains(':')
            || tr.starts_with("do_compile")
            || tr.starts_with("do_install")
            || tr.starts_with("do_configure")
            || tr.starts_with("do_fetch")
            || tr.starts_with("do_unpack")
            || tr.starts_with("do_patch")
            || tr.starts_with("do_package"))
}

/// Detect a BitBake recipe.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut assigns = 0usize;
    let mut dirs = 0usize;
    let mut tasks = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with('#') {
            continue;
        }
        if var_line(tr) {
            assigns += 1;
        } else if directive_line(tr) {
            dirs += 1;
        } else if task_line(tr) {
            tasks += 1;
        }
    }
    (assigns >= 2 && (dirs >= 1 || tasks >= 1)) || assigns >= 4
}

impl BitBake {
    /// Count categories. Returns `None` when the input does not look like
    /// a BitBake recipe.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            assignments: 0,
            directives: 0,
            tasks: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if var_line(tr) {
                c.assignments += 1;
            } else if directive_line(tr) {
                c.directives += 1;
            } else if task_line(tr) {
                c.tasks += 1;
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`BitBake::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<BitBake> {
    BitBake::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# demo recipe\nSUMMARY = \"demo app\"\nDESCRIPTION = \"long\"\nHOMEPAGE = \"https://x\"\nLICENSE = \"MIT\"\nLIC_FILES_CHKSUM = \"file://COPYING;md5=abc\"\nSRC_URI = \"git://x/repo;branch=main\"\nSRCREV = \"${AUTOREV}\"\nPV = \"1.0+git${SRCPV}\"\nDEPENDS = \"zlib openssl\"\nRDEPENDS_${PN} += \"busybox\"\nPACKAGECONFIG ?= \"feature\"\ninherit cmake pkgconfig\ninherit_defer native\nrequire common.inc\ndo_compile() {\n    oe_runmake\n}\ndo_install() {\n    oe_runmake install DESTDIR=${D}\n}\n";
        assert!(detect(b));
        let c = BitBake::parse(b).unwrap();
        assert!(c.assignments >= 11);
        assert_eq!(c.directives, 3);
        assert_eq!(c.tasks, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_makefile() {
        assert!(!detect(b"CC=gcc\nall: build\ninstall:\n\tcp x y\n"));
        assert!(!detect(b"name = value\nfoo = bar\n"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"# SRC_URI = \"x\"\n# inherit cmake\n# do_compile() {}\n"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(BitBake::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"SRC_URI");
        assert!(!detect(&b));
    }
}
