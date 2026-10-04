//! Premake build file (`premake5.lua`/`premake4.lua`) parser.
//!
//! Detects premake scripts by their characteristic calls (`workspace`/
//! `project`/`kind`/`language`/`files`/`includedirs`/`defines`/`links`/
//! `filter`/`configurations`/`libdirs`/`pchheader`/`targetdir`/`newoption`/
//! `newaction`/`include`/`group` …) and counts occurrences by category.
//!
//! ```
//! let b = b"workspace \"w\"\n   configurations { \"Debug\", \"Release\" }\n   project \"app\"\n      kind \"ConsoleApp\"\n      language \"C++\"\n      files { \"src/**.cpp\" }\n";
//! assert!(izanagi_kit::premakeconf::detect(b));
//! let c = izanagi_kit::premakeconf::Premake::parse(b).unwrap();
//! assert!(c.keys >= 5);
//! ```

/// Parsed premake script summary.
#[derive(Debug, Clone)]
pub struct Premake {
    /// Recognized call occurrences.
    pub keys: usize,
    /// Scope calls (`workspace`/`project`/`group`/`externalproject`/`rule`/`configurations`/`platforms`/`map`).
    pub scope_keys: usize,
    /// Settings calls (`kind`/`language`/`dialect`/`cdialect`/`cppdialect`/`staticruntime`/`rtti`/`exceptionhandling`/`symbols`/`optimize`/`warnings`/`fatalwarnings`/`externalwarnings`/`flags`/`pic`/`visibility`/`charactertoolset`/`toolset`/`architecture`/`system`/`systemversion`/`clr`/`editandcontinue`/`callingconvention`/`buildoptions`/`linkoptions`/`allmodulespublic`/`enableunitybuild`/`compileas`/`conformancemode`/`preferredtoolarchitecture`/`startproject`/`location`/`targetdir`/`targetname`/`targetextension`/`objdir`/`implibdir`/`debugdir`/`debugcommand`/`debugargs`/`debugenvs`/`runpathdirs`/`framework`/`icon`/`nuget`/`namespace`/`filename`/`uuid`/`preferredsuffix`/`resources`/`kinds`).
    pub setting_keys: usize,
    /// File/dependency calls (`files`/`removefiles`/`includedirs`/`removeincludedirs`/`forceincludes`/`defines`/`removedefines`/`undefines`/`links`/`libdirs`/`dependson`/`inlining`/`uses`/`prebuildcommands`/`prebuildmessage`/`postbuildcommands`/`postbuildmessage`/`prelinkcommands`/`pchheader`/`pchsource`/`filter`/`filters`/`vectorextensions`/`propertydefinition`/`newoption`/`newaction`/`include`/`externalincludedirs`/`externalwarnings`/`dpiawareness`/`unity`/`shadermodel`/`shadertype`/`shaderdefines`/`shaderoptions`/`shaderassembler`/`shaderobjectfileoutput`/`shaderheaderfileoutput`/`shadervariablename`).
    pub dep_keys: usize,
    /// `--`/`#` comment lines.
    pub comments: usize,
}

/// Scope calls.
const SCOPE_KEYS: &[&str] = &[
    "workspace",
    "project",
    "group",
    "externalproject",
    "rule",
    "configurations",
    "platforms",
    "map",
];

/// Settings calls.
const SETTING_KEYS: &[&str] = &[
    "kind",
    "language",
    "cdialect",
    "cppdialect",
    "staticruntime",
    "rtti",
    "exceptionhandling",
    "symbols",
    "optimize",
    "warnings",
    "fatalwarnings",
    "externalwarnings",
    "flags",
    "pi\u{63}",
    "visibility",
    "toolset",
    "architecture",
    "system",
    "systemversion",
    "clr",
    "editandcontinue",
    "callingconvention",
    "buildoptions",
    "linkoptions",
    "allmodulespubli\u{63}",
    "enableunitybuild",
    "compileas",
    "conformancemode",
    "startproject",
    "location",
    "targetdir",
    "targetname",
    "targetextension",
    "objdir",
    "implibdir",
    "debugdir",
    "debugcommand",
    "debugargs",
    "debugenvs",
    "runpathdirs",
    "framework",
    "nuget",
    "namespace",
    "filename",
    "uuid",
    "resources",
];

/// File/dependency calls.
const DEP_KEYS: &[&str] = &[
    "files",
    "removefiles",
    "includedirs",
    "removeincludedirs",
    "forceincludes",
    "defines",
    "removedefines",
    "undefines",
    "links",
    "libdirs",
    "dependson",
    "inlining",
    "uses",
    "prebuildcommands",
    "prebuildmessage",
    "postbuildcommands",
    "postbuildmessage",
    "prelinkcommands",
    "pchheader",
    "pchsource",
    "filter",
    "filters",
    "vectorextensions",
    "propertydefinition",
    "newoption",
    "newaction",
    "include",
    "externalincludedirs",
    "dpiawareness",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["premake", "workspace", "configurations", "project"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "workspace",
    "project",
    "kind",
    "language",
    "files",
    "includedirs",
    "defines",
    "links",
    "filter",
    "configurations",
    "libdirs",
    "pchheader",
    "targetdir",
    "newoption",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("{k} "))
        || t.contains(&format!("{k}("))
        || t.contains(&format!("{k}{{"))
        || t.contains(&format!("{k} {{"))
        || t.contains(&format!("\"{k}\""))
}

/// Detect a premake script.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 3 && anchor
}

impl Premake {
    /// Count call categories in a premake script. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            scope_keys: 0,
            setting_keys: 0,
            dep_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("--") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in SCOPE_KEYS {
            c.scope_keys += t.matches(k).count();
        }
        for k in SETTING_KEYS {
            c.setting_keys += t.matches(k).count();
        }
        for k in DEP_KEYS {
            c.dep_keys += t.matches(k).count();
        }
        c.keys = c.scope_keys + c.setting_keys + c.dep_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"-- premake5\nworkspace \"myws\"\n   configurations { \"Debug\", \"Release\" }\n   platforms { \"x64\" }\n   project \"app\"\n      kind \"ConsoleApp\"\n      language \"C++\"\n      cppdialect \"C++17\"\n      targetdir \"bin/%{cfg.buildcfg}\"\n      files { \"src/**.h\", \"src/**.cpp\" }\n      includedirs { \"include\" }\n      defines { \"APP\" }\n      links { \"pthread\" }\n      filter \"configurations:Debug\"\n         symbols \"On\"\n      filter \"configurations:Release\"\n         optimize \"On\"\n";
        assert!(detect(b));
        let c = Premake::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.scope_keys >= 4);
        assert!(c.setting_keys >= 5);
        assert!(c.dep_keys >= 5);
        assert!(c.keys >= 14);
    }

    #[test]
    fn rejects_random_lua() {
        assert!(!detect(b"local x = 1\nprint(x)\n"));
        assert!(Premake::parse(b"a = b\n").is_none());
    }
}
