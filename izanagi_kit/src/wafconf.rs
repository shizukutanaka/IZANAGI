//! waf build file (`wscript`) parser.
//!
//! Detects waf wscripts by their characteristic structure (`def options`/
//! `def configure`/`def build`, `ctx.`/`conf.`/`bld.` receivers,
//! `conf.check_cc`/`conf.define`/`conf.write_config_header`,
//! `bld(features=…)`/`bld.program`/`bld.shlib`/`bld.stlib`,
//! `source=`/`target=`/`use=`/`includes=`/`uselib=` …) and counts
//! occurrences by category.
//!
//! ```
//! let b = b"def options(ctx):\n    ctx.load('compiler_c')\n\ndef configure(conf):\n    conf.load('compiler_c')\n\ndef build(bld):\n    bld.program(source='main.c', target='app')\n";
//! assert!(izanagi_kit::wafconf::detect(b));
//! let c = izanagi_kit::wafconf::Waf::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed wscript summary.
#[derive(Debug, Clone)]
pub struct Waf {
    /// Recognized call occurrences.
    pub keys: usize,
    /// Lifecycle `def`s and receiver calls (`def options`/`def configure`/`def build`/`def init`/`def shutdown`/`def clean`/`def dist`/`def distclean`/`def distcheck`/`def check`/`ctx.*`/`conf.*`/`bld.*`).
    pub hook_keys: usize,
    /// `conf.`/`ctx.` check/load calls (`conf.load`/`conf.check`/`conf.check_cc`/`conf.check_cxx`/`conf.check_cfg`/`conf.check_cfg_package`/`conf.check_header`/`conf.check_lib`/`conf.check_tool`/`conf.find_program`/`conf.define`/`conf.undefine`/`conf.write_config_header`/`conf.env`/`conf.recurse`/`conf.check_large_file`/`conf.check_endianness`/`conf.multicheck`/`ctx.load`/`ctx.recurse`/`ctx.options`/`ctx.add_option`/`ctx.find_program`/`ctx.path`/`ctx.srcnode`/`ctx.bldnode`/`ctx.fatal`/`ctx.to_include`/`ctx.check_waf_version`).
    pub check_keys: usize,
    /// `bld.` build calls and task attributes (`bld.program`/`bld.shlib`/`bld.stlib`/`bld.objects`/`bld(features=`/`bld.add_group`/`bld.add_subdirs`/`bld.recurse`/`bld.install_files`/`bld.install_as`/`bld.symlink_as`/`bld.env`/`bld.path`/`source=`/`target=`/`use=`/`uselib=`/`uselib_local=`/`includes=`/`defines=`/`cflags=`/`cxxflags=`/`linkflags=`/`lib=`/`libpath=`/`stlib=`/`stlibpath=`/`rpath=`/`vnum=`/`install_path=`/`name=`/`export_includes=`/`export_defines=`/`features=`/`rule=`/`before=`/`after=`/`always=`/`on_results=`/`ext_in=`/`ext_out=`/`shell=`/`scan=`/`deps=`/`copy=`/`answer=`/`meths=`/`install_from=`/`install_to=`/`chmod=`/`env=`).
    pub bld_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Lifecycle defs + receiver prefixes.
const HOOK_KEYS: &[&str] = &[
    "def options",
    "def configure",
    "def build",
    "def init",
    "def shutdown",
    "def clean",
    "def dist",
    "def distclean",
    "def distcheck",
    "def check",
    "ctx.",
    "conf.",
    "bld.",
];

/// conf./ctx. check/load calls.
const CHECK_KEYS: &[&str] = &[
    "conf.load",
    "conf.check",
    "conf.check_cc",
    "conf.check_cxx",
    "conf.check_cfg",
    "conf.check_cfg_package",
    "conf.check_header",
    "conf.check_lib",
    "conf.check_tool",
    "conf.find_program",
    "conf.define",
    "conf.undefine",
    "conf.write_config_header",
    "conf.env",
    "conf.recurse",
    "conf.check_large_file",
    "conf.check_endianness",
    "conf.multicheck",
    "ctx.load",
    "ctx.recurse",
    "ctx.options",
    "ctx.add_option",
    "ctx.find_program",
    "ctx.fatal",
    "ctx.check_waf_version",
];

/// bld. calls + task attribute kwargs.
const BLD_KEYS: &[&str] = &[
    "bld.program",
    "bld.shlib",
    "bld.stlib",
    "bld.objects",
    "bld(features",
    "bld.add_group",
    "bld.add_subdirs",
    "bld.recurse",
    "bld.install_files",
    "bld.install_as",
    "bld.symlink_as",
    "bld.env",
    "source=",
    "target=",
    "use=",
    "uselib=",
    "uselib_local=",
    "includes=",
    "defines=",
    "cflags=",
    "cxxflags=",
    "linkflags=",
    "lib=",
    "libpath=",
    "stlib=",
    "stlibpath=",
    "rpath=",
    "vnum=",
    "install_path=",
    "export_includes=",
    "export_defines=",
    "features=",
    "rule=",
    "deps=",
    "install_from=",
    "install_to=",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["def configure", "bld", "conf.", "waf"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "def options",
    "def configure",
    "def build",
    "conf.",
    "bld.",
    "ctx.",
    "conf.check_cc",
    "conf.define",
    "bld.program",
    "features=",
    "use=",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a wscript file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 3 && anchor
}

impl Waf {
    /// Count call categories in a wscript. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            hook_keys: 0,
            check_keys: 0,
            bld_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in HOOK_KEYS {
            c.hook_keys += t.matches(k).count();
        }
        for k in CHECK_KEYS {
            c.check_keys += t.matches(k).count();
        }
        for k in BLD_KEYS {
            c.bld_keys += t.matches(k).count();
        }
        c.keys = c.hook_keys + c.check_keys + c.bld_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# waf\ndef options(ctx):\n    ctx.load('compiler_c')\n\ndef configure(conf):\n    conf.load('compiler_c')\n    conf.check_cc(header_name='stdio.h')\n    conf.define('HAVE_X', 1)\n    conf.write_config_header('config.h')\n\ndef build(bld):\n    bld.program(features='c', source='main.c', target='app', use='X', includes='inc')\n    bld.install_files('${PREFIX}/bin', 'app')\n";
        assert!(detect(b));
        let c = Waf::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.hook_keys >= 3);
        assert!(c.check_keys >= 3);
        assert!(c.bld_keys >= 4);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_random_py() {
        assert!(!detect(b"x = 1\nprint(x)\n"));
        assert!(Waf::parse(b"a = b\n").is_none());
    }
}
