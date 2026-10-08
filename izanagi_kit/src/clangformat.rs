//! ClangFormat `.clang-format` census.
//!
//! `.clang-format` is YAML: `BasedOnStyle:`/`Language:`/`ColumnLimit:`/
//! `IndentWidth:`/`UseTab:`/`BreakBeforeBraces:`/`AccessModifierOffset:`/
//! `PointerAlignment:`/`Align*`/`AllowShort*`/`Spaces*`/`Penalty*`/
//! `Standard:`/`TabWidth:`/`IncludeCategories:` + `- Regex:`/`Priority:`
//! list entries.
//!
//! ```rust
//! let c = izanagi_kit::clangformat::ClangFormat::parse(b"BasedOnStyle: LLVM\nIndentWidth: 4\nColumnLimit: 100\n").unwrap();
//! assert_eq!(c.settings, 3);
//! ```

use crate::textutil::strip_bom;
/// `.clang-format` census.
#[derive(Debug, Clone)]
pub struct ClangFormat {
    /// `Key: value` settings.
    pub settings: usize,
    /// Settings whose value is `true`/`false`.
    pub booleans: usize,
    /// `- ` list items (`IncludeCategories` entries etc).
    pub listitems: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "BasedOnStyle",
    "Language",
    "ColumnLimit",
    "IndentWidth",
    "TabWidth",
    "UseTab",
    "AccessModifierOffset",
    "AlignAfterOpenBracket",
    "AlignConsecutive",
    "AlignEscapedNewlines",
    "AlignOperands",
    "AlignTrailingComments",
    "AllowAllArgumentsOnNextLine",
    "AllowAllParametersOfDeclarationOnNextLine",
    "AllowShort",
    "AlwaysBreak",
    "BinPack",
    "BitFieldColonSpacing",
    "BraceWrapping",
    "BreakAfterJavaFieldAnnotations",
    "BreakArrays",
    "BreakBefore",
    "BreakConstructorInitializers",
    "BreakInheritanceList",
    "BreakStringLiterals",
    "CompactNamespaces",
    "ContinuationIndentWidth",
    "Cpp11BracedListStyle",
    "DerivePointerAlignment",
    "EmptyLine",
    "FixNamespaceComments",
    "ForEachMacros",
    "IfMacros",
    "IncludeBlocks",
    "IncludeCategories",
    "IndentCase",
    "IndentExternBlock",
    "IndentFunction",
    "IndentGotoLabels",
    "IndentPPDirectives",
    "IndentWrappedFunctionNames",
    "InsertBraces",
    "InsertNewlineAtEOF",
    "IntegerLiteralSeparator",
    "KeepEmptyLines",
    "MacroBlock",
    "MaxEmptyLinesToKeep",
    "NamespaceIndentation",
    "NamespaceMacros",
    "ObjC",
    "PPIndentWidth",
    "PackConstructorInitializers",
    "Penalty",
    "PointerAlignment",
    "QualifierAlignment",
    "QualifierMacros",
    "ReferenceAlignment",
    "ReflowComments",
    "RemoveBracesLLVM",
    "RemoveParentheses",
    "RemoveSemicolon",
    "RequiresClausePosition",
    "RequiresExpressionIndentation",
    "SeparateDefinitionBlocks",
    "ShortNamespaceLines",
    "SortIncludes",
    "SortJavaStaticImport",
    "SortUsingDeclarations",
    "SpaceAfter",
    "SpaceAround",
    "SpaceBefore",
    "SpaceIn",
    "Spaces",
    "Standard",
    "StatementAttributeLikeMacros",
    "StatementMacros",
    "TypenameMacros",
    "WhitespaceSensitiveMacros",
];

fn is_key(k: &str) -> bool {
    KEYS.iter().any(|x| k.starts_with(x))
}

/// Whether the buffer looks like a `.clang-format` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines()
        .filter(|l| {
            let s = l.trim();
            s.split(':')
                .next()
                .map(|k| is_key(k.trim()))
                .unwrap_or(false)
        })
        .count()
        >= 2
}

impl ClangFormat {
    /// Parse a `.clang-format` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            settings: 0,
            booleans: 0,
            listitems: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s == "---" || s == "..." {
                continue;
            }
            if s.strip_prefix("- ").is_some() || s == "-" {
                c.listitems += 1;
                continue;
            }
            if let Some((_, v)) = s.split_once(':') {
                c.settings += 1;
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if v == "true" || v == "false" {
                    c.booleans += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clang_format() {
        let b = concat!(
            "# cf\n",
            "---\n",
            "BasedOnStyle: LLVM\n",
            "Language: Cpp\n",
            "IndentWidth: 4\n",
            "ColumnLimit: 100\n",
            "UseTab: Never\n",
            "BreakBeforeBraces: Attach\n",
            "AllowShortFunctionsOnASingleLine: true\n",
            "SortIncludes: false\n",
            "IncludeCategories:\n",
            "  - Regex: '^<ext/'\n",
            "    Priority: 3\n",
            "  - Regex: '.*'\n",
            "    Priority: 1\n",
            "PointerAlignment: Left\n",
            "...\n",
        );
        let c = ClangFormat::parse(b.as_bytes()).unwrap();
        assert_eq!(c.listitems, 2);
        assert_eq!(c.settings, 12);
        assert_eq!(c.booleans, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(ClangFormat::parse(b"foo: bar").is_none());
        assert!(ClangFormat::parse(b"Checks: '-*'").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
