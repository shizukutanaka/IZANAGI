//! xmake build file (`xmake.lua`) parser.
//!
//! Detects xmake.lua scripts by their characteristic calls (`set_project`/
//! `add_rules`/`add_requires`/`target()`/`set_kind`/`add_files`/
//! `add_includedirs`/`set_languages`/`option()`/`add_defines`/`add_links`/
//! `includes(`/`set_configvar` …) and counts occurrences by category.
//!
//! ```
//! let b = b"set_project(\"app\")\nadd_rules(\"mode.debug\")\ntarget(\"app\")\n    set_kind(\"binary\")\n    add_files(\"src/*.cpp\")\n    set_languages(\"cxx17\")\n";
//! assert!(izanagi_kit::xmakeconf::detect(b));
//! let c = izanagi_kit::xmakeconf::Xmake::parse(b).unwrap();
//! assert!(c.keys >= 5);
//! ```

/// Parsed xmake.lua summary.
#[derive(Debug, Clone)]
pub struct Xmake {
    /// Recognized call occurrences.
    pub keys: usize,
    /// Target-declaration calls (`target`/`target_end`/`option`/`option_end`/`rule`/`rule_end`/`task`/`task_end`/`package`/`package_end`/`toolchain`/`toolchain_end`/`includes`/`add_subdirs`/`add_subfiles`).
    pub decl_keys: usize,
    /// Target attribute calls (`set_kind`/`add_files`/`add_defines`/`add_includedirs`/`add_links`/`add_syslinks`/`add_packages`/`add_deps`/`set_languages`/`set_targetdir`/`set_basename`/`set_runtimes`/`set_filename`/`add_headerfiles`/`set_values`/`set_configvar`/`add_cflags`/`add_cxflags`/`add_cxxflags`/`add_ldflags`/`add_arflags`/`add_shflags`/`add_mflags`/`add_scflags`/`add_installfiles`/`add_extrafiles`/`set_warnings`/`set_optimize`/`set_symbols`/`set_strip`/`set_policy`/`set_group`/`set_default`/`set_enabled`/`set_rundir`/`set_runargs`/`set_license`/`set_description`/`set_homepage`/`set_version`/`add_rules`/`set_project`/`add_requires`/`add_requireconfs`/`set_xmakever`).
    pub attr_keys: usize,
    /// Hook/lifecycle calls (`on_build`/`before_build`/`after_build`/`on_install`/`before_install`/`after_install`/`on_package`/`on_test`/`on_load`/`on_config`/`on_clean`/`on_run`/`on_check`/`on_download`/`on_prepare`/`on_link`/`before_link`/`after_link`/`on_source`/`on_package_source`/`main`/`print`/`raise`/`catch`/`try`/`finally`/`import`/`inherit`/`if`/`os`/`path`/`io`/`net`/`utils`).
    pub hook_keys: usize,
    /// `#`/`--` comment lines.
    pub comments: usize,
}

/// Declaration calls.
const DECL_KEYS: &[&str] = &[
    "target",
    "target_end",
    "option",
    "option_end",
    "rule",
    "rule_end",
    "task",
    "task_end",
    "package",
    "package_end",
    "toolchain",
    "toolchain_end",
    "includes",
    "add_subdirs",
    "add_subfiles",
];

/// Attribute calls.
const ATTR_KEYS: &[&str] = &[
    "set_kind",
    "add_files",
    "add_defines",
    "add_includedirs",
    "add_links",
    "add_syslinks",
    "add_packages",
    "add_deps",
    "set_languages",
    "set_targetdir",
    "set_basename",
    "set_runtimes",
    "set_filename",
    "add_headerfiles",
    "set_values",
    "set_configvar",
    "add_cflags",
    "add_cxflags",
    "add_cxxflags",
    "add_ldflags",
    "add_arflags",
    "add_shflags",
    "add_installfiles",
    "add_extrafiles",
    "set_warnings",
    "set_optimize",
    "set_symbols",
    "set_strip",
    "set_policy",
    "set_group",
    "set_default",
    "set_enabled",
    "set_rundir",
    "set_runargs",
    "set_license",
    "set_description",
    "set_homepage",
    "set_version",
    "add_rules",
    "set_project",
    "add_requires",
    "add_requireconfs",
    "set_xmakever",
];

/// Hook/lifecycle calls.
const HOOK_KEYS: &[&str] = &[
    "on_build",
    "before_build",
    "after_build",
    "on_install",
    "before_install",
    "after_install",
    "on_package",
    "on_test",
    "on_load",
    "on_config",
    "on_clean",
    "on_run",
    "on_check",
    "on_download",
    "on_prepare",
    "on_link",
    "before_link",
    "after_link",
    "on_source",
    "on_package_source",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["xmake", "target(", "add_rules", "set_kind"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "set_project",
    "add_rules",
    "add_requires",
    "target",
    "set_kind",
    "add_files",
    "add_includedirs",
    "set_languages",
    "option",
    "add_defines",
    "add_links",
    "includes",
    "set_configvar",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("{k}(")) || t.contains(&format!("\"{k}\"")) || t.contains(&format!("'{k}'"))
}

/// Detect an xmake.lua file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Xmake {
    /// Count call categories in an xmake.lua file. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            decl_keys: 0,
            attr_keys: 0,
            hook_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("--") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in DECL_KEYS {
            c.decl_keys += t.matches(&format!("{k}(")).count();
        }
        for k in ATTR_KEYS {
            c.attr_keys += t.matches(&format!("{k}(")).count();
        }
        for k in HOOK_KEYS {
            c.hook_keys += t.matches(&format!("{k}(")).count();
        }
        c.keys = c.decl_keys + c.attr_keys + c.hook_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"-- xmake\nset_project(\"app\")\nset_version(\"1.0\")\nadd_rules(\"mode.debug\", \"mode.release\")\nadd_requires(\"zlib\")\ntarget(\"app\")\n    set_kind(\"binary\")\n    add_files(\"src/*.cpp\")\n    add_includedirs(\"include\")\n    set_languages(\"cxx17\")\n    add_defines(\"USE_X\")\n    add_links(\"z\")\n    on_install(function (target) end)\ntarget_end()\n";
        assert!(detect(b));
        let c = Xmake::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.decl_keys >= 2);
        assert!(c.attr_keys >= 7);
        assert!(c.hook_keys >= 1);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_random_lua() {
        assert!(!detect(b"local x = 1\nprint(x)\n"));
        assert!(Xmake::parse(b"a = b\n").is_none());
    }
}
