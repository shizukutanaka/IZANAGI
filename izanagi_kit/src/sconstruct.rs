//! SCons build file (`SConstruct`/`SConscript`) parser.
//!
//! Detects SCons scripts by their characteristic calls (`Environment()`/
//! `Program`/`Library`/`SharedLibrary`/`StaticLibrary`/`SConscript`/
//! `VariantDir`/`env.Append`/`env.Prepend`/`Export`/`Import`/`Install`/
//! `Command`/`Alias`/`Default`/`Configure`/`Variables`/`GetOption` …)
//! and counts occurrences by category.
//!
//! ```
//! let b = b"env = Environment()\nenv.Append(CCFLAGS='-O2')\nenv.Program('app', ['main.c', 'util.c'])\n";
//! assert!(izanagi_kit::sconstruct::detect(b));
//! let c = izanagi_kit::sconstruct::Scons::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed SCons script summary.
#[derive(Debug, Clone)]
pub struct Scons {
    /// Recognized call occurrences.
    pub keys: usize,
    /// Build-target calls (`Program`/`Library`/`StaticLibrary`/`SharedLibrary`/`Object`/`SharedObject`/`StaticObject`/`Java`/`Jar`/`DVI`/`PDF`/`Package`/`Tar`/`Zip`/`RPMS`/`MSI`/`Install`/`InstallAs`/`InstallVersionedLib`/`Command`/`Clone`).
    pub target_keys: usize,
    /// Env-var calls (`env.Append`/`env.Prepend`/`env.Replace`/`env.AppendUnique`/`env.PrependUnique`/`env.ParseConfig`/`env.MergeFlags`/`env.Tool`/`env.Clone`/`env.Environment`/`env.Override`/`env.Default`/`env.Dir`/`env.File`/`env.Entry`/`env.Glob`/`env.Value`/`env.Action`/`env.Builder`/`env.Scanner`/`env.WhereIs`/`env.Platform`/`env.Tool`/`env.SConsignFile`/`env.Repository`/`env.VariantDir`/`env.GetBuildPath`/`env.subst`/`env.Dump`/`env.FindIxes`/`env.Requires`/`env.Depends`/`env.Ignore`/`env.SideEffect`/`env.Precious`/`env.AlwaysBuild`/`env.Pseudo`/`env.NoClean`/`env.NoCache`/`env.Clean`/`env.FindSourceFiles`/`env.FindInstalledFiles`/`env.Execute`/`env.Configure`/`env.Local`/`env.Flatten`/`env.Split`/`env.Check*` methods).
    pub env_keys: usize,
    /// Top-level calls (`Environment`/`SConscript`/`VariantDir`/`Export`/`Import`/`Return`/`Default`/`Alias`/`Depends`/`Requires`/`AlwaysBuild`/`Clean`/`NoClean`/`NoCache`/`Ignore`/`SideEffect`/`Precious`/`Pseudo`/`Builder`/`Scanner`/`Decider`/`FindFile`/`FindDir`/`FindSourceFiles`/`FindInstalledFiles`/`GetOption`/`SetOption`/`ARGUMENTS`/`Variables`/`BoolVariable`/`EnumVariable`/`ListVariable`/`PathVariable`/`PackageVariable`/`Variables`/`Help`/`Exit`/`Configure`/`conf.Finish`/`CheckLib`/`CheckFunc`/`CheckHeader`/`CheckType`/`CheckCC`/`CheckCXX`/`CheckSHCC`/`CheckProg`/`CheckMember`/`CheckDecl`/`EnsureSConsVersion`/`EnsurePythonVersion`/`GetLaunchDir`/`Platform`/`Tool`/`Glob`/`Dir`/`File`/`Entry`/`Value`/`Action`/`Factory`/`CacheDir`/`SConsignFile`/`Repository`/`SOURCE_DATE_EPOCH`/`BUILD_TARGETS`/`COMMAND_LINE_TARGETS`/`DEFAULT_TARGETS`/`SCONSFLAGS`/`MSVSProject`/`GetBuildFailures`/`Progress`/`Tag`/`LoadableModule`/`PythonFramework`/`RES`).
    pub top_keys: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

/// Build-target calls.
const TARGET_KEYS: &[&str] = &[
    "Program",
    "Library",
    "StaticLibrary",
    "SharedLibrary",
    "Object",
    "SharedObject",
    "StaticObject",
    "Java",
    "Jar",
    "DVI",
    "PDF",
    "Package",
    "Tar",
    "Zip",
    "MSI",
    "Install",
    "InstallAs",
    "InstallVersionedLib",
    "Command",
    "LoadableModule",
];

/// env.* method calls.
const ENV_KEYS: &[&str] = &[
    "env.Append",
    "env.Prepend",
    "env.Replace",
    "env.AppendUnique",
    "env.PrependUnique",
    "env.ParseConfig",
    "env.MergeFlags",
    "env.Tool",
    "env.Clone",
    "env.Override",
    "env.Dir",
    "env.File",
    "env.Entry",
    "env.Glob",
    "env.Value",
    "env.Action",
    "env.Builder",
    "env.Scanner",
    "env.WhereIs",
    "env.Platform",
    "env.SConsignFile",
    "env.Repository",
    "env.VariantDir",
    "env.GetBuildPath",
    "env.subst",
    "env.Dump",
    "env.Requires",
    "env.Depends",
    "env.Ignore",
    "env.SideEffect",
    "env.Precious",
    "env.AlwaysBuild",
    "env.Pseudo",
    "env.NoClean",
    "env.NoCache",
    "env.Clean",
    "env.FindSourceFiles",
    "env.FindInstalledFiles",
    "env.Execute",
    "env.Configure",
    "env.Local",
    "env.Flatten",
    "env.Split",
];

/// Top-level calls.
const TOP_KEYS: &[&str] = &[
    "Environment",
    "SConscript",
    "VariantDir",
    "Export",
    "Import",
    "Return",
    "Default",
    "Alias",
    "Depends",
    "Requires",
    "AlwaysBuild",
    "Clean",
    "NoClean",
    "NoCache",
    "Ignore",
    "SideEffect",
    "Precious",
    "Pseudo",
    "Builder",
    "Scanner",
    "Decider",
    "FindFile",
    "FindDir",
    "FindSourceFiles",
    "FindInstalledFiles",
    "GetOption",
    "SetOption",
    "ARGUMENTS",
    "Variables",
    "BoolVariable",
    "EnumVariable",
    "ListVariable",
    "PathVariable",
    "PackageVariable",
    "Help",
    "Exit",
    "Configure",
    "EnsureSConsVersion",
    "EnsurePythonVersion",
    "GetLaunchDir",
    "Platform",
    "Tool",
    "Glob",
    "Dir",
    "File",
    "Entry",
    "Value",
    "Action",
    "Factory",
    "CacheDir",
    "SConsignFile",
    "Repository",
    "MSVSProject",
    "GetBuildFailures",
    "Progress",
    "Tag",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["Environment", "env.", "SConscript", "Program("];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "Environment",
    "env.Append",
    "env.Prepend",
    "Program",
    "Library",
    "SharedLibrary",
    "StaticLibrary",
    "SConscript",
    "VariantDir",
    "Export",
    "Import",
    "Install",
    "Command",
    "Alias",
    "Default",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("{k}("))
        || t.contains(&format!("\"{k}\""))
        || t.contains(&format!("'{k}'"))
        || t.contains(&format!("{k}="))
        || t.contains(&format!("{k} ="))
}

/// Detect an SCons script.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Scons {
    /// Count call categories in an SCons script. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            target_keys: 0,
            env_keys: 0,
            top_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
            }
        }
        for k in TARGET_KEYS {
            c.target_keys += t.matches(&format!("{k}(")).count();
        }
        for k in ENV_KEYS {
            c.env_keys += t.matches(&format!("{k}(")).count();
        }
        for k in TOP_KEYS {
            c.top_keys += t.matches(&format!("{k}(")).count();
        }
        c.keys = c.target_keys + c.env_keys + c.top_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# scons\nenv = Environment()\nenv.Append(CCFLAGS='-O2', CPPDEFINES=['X'])\nenv.ParseConfig('pkg-config --cflags --libs zlib')\nenv.Program('app', Glob('src/*.c'))\nenv.SharedLibrary('plugin', ['p.c'])\nSConscript('sub/SConscript', exports='env')\nAlias('all', 'app')\nDefault('all')\n";
        assert!(detect(b));
        let c = Scons::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.target_keys >= 2);
        assert!(c.env_keys >= 2);
        assert!(c.top_keys >= 3);
        assert!(c.keys >= 7);
    }

    #[test]
    fn rejects_random_py() {
        assert!(!detect(b"x = 1\nprint(x)\n"));
        assert!(Scons::parse(b"a = b\n").is_none());
    }
}
