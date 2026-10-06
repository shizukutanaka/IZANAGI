//! FASTBuild `.bff` ビルドスクリプトの検出と構造カウント。
//!
//! `Settings`/`Compiler`/`ObjectList`/`Library`/`Alias`/`Exec`/`Test`/
//! `Unity`/`DLL`/`Exe`/`Copy`/`CopyDir`/`RemoveDir`/`CSAssembly`/
//! `VCXProject`/`VSProject`/`TextFile`/`Print`/`If`/`ForEach`/
//! `Using`/`function` ブロック呼び出しと `.Var =` 代入、`;` コメント、
//! `#include`/`#if`/`#define`/`#import`/`#once` プリプロセッサを識別する。
//!
//! ```
//! let b = b".ToolsPath = \"tools\"\nSettings\n{\n    .Environment = \"x64\"\n}\nCompiler( \"cc\" )\n{\n    .Executable = \"cc\"\n}\n";
//! assert!(izanagi_kit::fastbuild::detect(b));
//! let c = izanagi_kit::fastbuild::FastBuild::parse(b).unwrap();
//! assert_eq!(c.functions, 2);
//! ```

/// Parsed .bff summary.
#[derive(Debug, Clone)]
pub struct FastBuild {
    /// `Name(`/`Name( "x" )` block/call lines.
    pub functions: usize,
    /// `.Var =` assignments.
    pub variables: usize,
    /// `#include`/`#if`/`#define`/`#import`/`#once` lines.
    pub directives: usize,
    /// `;`/`//` comment lines.
    pub comments: usize,
}

/// bff block/function names.
const FUNCS: &[&str] = &[
    "Alias",
    "Assembly",
    "CILibrary",
    "Compiler",
    "Copy",
    "CopyDir",
    "CSAssembly",
    "DLL",
    "Env",
    "Error",
    "Exec",
    "ExecOutput",
    "Executable",
    "ForEach",
    "function",
    "If",
    "Library",
    "ListDependencies",
    "ObjectList",
    "Print",
    "RemoveDir",
    "Settings",
    "SLN",
    "Test",
    "TextFile",
    "Unity",
    "Using",
    "VCXProject",
    "VSSolution",
    "VSProject",
    "WebResource",
    "WinRAR",
    "XCodeProject",
];

const DIRECTIVES: &[&str] = &[
    "#include", "#if", "#define", "#import", "#once", "#undef", "#else", "#endif",
];

fn func_line(tr: &str) -> bool {
    FUNCS
        .iter()
        .any(|f| tr.starts_with(f) && tr[f.len()..].trim_start().starts_with('(') || tr == *f)
}

fn var_line(tr: &str) -> bool {
    tr.starts_with('.') && tr.contains('=')
}

fn directive_line(tr: &str) -> bool {
    DIRECTIVES.iter().any(|d| tr.starts_with(d))
}

/// Detect a FASTBuild .bff script.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut funcs = 0usize;
    let mut vars = 0usize;
    let mut dirs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with(';') || tr.starts_with("//") {
            continue;
        }
        if func_line(tr) {
            funcs += 1;
        } else if var_line(tr) {
            vars += 1;
        } else if directive_line(tr) {
            dirs += 1;
        }
    }
    (funcs >= 1 && vars >= 1) || funcs >= 3 || (funcs >= 1 && dirs >= 1)
}

impl FastBuild {
    /// Count categories. Returns `None` when the input does not look like
    /// a FASTBuild script.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            functions: 0,
            variables: 0,
            directives: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with(';') || tr.starts_with("//") {
                c.comments += 1;
            } else if func_line(tr) {
                c.functions += 1;
            } else if var_line(tr) {
                c.variables += 1;
            } else if directive_line(tr) {
                c.directives += 1;
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`FastBuild::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<FastBuild> {
    FastBuild::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"; fbuild.bff
#include "common.bff"
.ToolsPath = "tools"
.CompilerOptions = "/nologo"
Settings
{
    .Environment = "x64"
    .CachePath = "cache"
}
Compiler( "msvc" )
{
    .Executable = "$ToolChain$/bin/cl.exe"
}
ObjectList( "lib" )
{
    .CompilerInputPath = "src"
}
Library( "lib-a" )
{
    .CompilerOutputPath = "out"
}
Alias( "all" ) { .Targets = { "lib-a" } }
"#;
        assert!(detect(b));
        let c = FastBuild::parse(b).unwrap();
        assert_eq!(c.functions, 5);
        assert!(c.variables >= 4);
        assert_eq!(c.directives, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_settings_plus_var() {
        assert!(detect(b".Env = \"x\"\nSettings\n{\n}\n"));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"func()\nfunc()\nfoo bar\n"));
        assert!(!detect(b"key = value\nother = 2\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(FastBuild::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"Settings");
        assert!(!detect(&b));
    }
}
