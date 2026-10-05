//! Pyright `pyrightconfig.json` parser.
//!
//! Detects Pyright/type-checker config JSON by `"typeCheckingMode"`,
//! `"include"`/`"exclude"`/`"ignore"`/`"stubPath"`/`"extraPaths"`,
//! `"report*"` diagnostic keys, and `"executionEnvironments"`. All
//! `"key":` occurrences are scanned (single-line JSON supported).
//!
//! ```
//! let b = b"{\n  \"include\": [\"src\"],\n  \"exclude\": [\"tests\"],\n  \"typeCheckingMode\": \"strict\",\n  \"reportMissingImports\": true,\n  \"reportGeneralTypeIssues\": \"error\"\n}\n";
//! assert!(izanagi_kit::pyrightconf::detect(b));
//! let c = izanagi_kit::pyrightconf::Pyright::parse(b).unwrap();
//! assert!(c.report_keys >= 2);
//! ```

/// Parsed pyrightconfig.json summary.
#[derive(Debug, Clone)]
pub struct Pyright {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Core keys (include/exclude/ignore/stubPath/venv*/executionEnvironments/typeCheckingMode).
    pub core_keys: usize,
    /// `report*` diagnostic keys.
    pub report_keys: usize,
    /// `"key": value` pair lines.
    pub assignments: usize,
    /// `//` comment lines.
    pub comments: usize,
}

/// Core (non-diagnostic) keys.
const CORE_KEYS: &[&str] = &[
    "\"include\"",
    "\"exclude\"",
    "\"ignore\"",
    "\"defineConstant\"",
    "\"stubPath\"",
    "\"typeshedPath\"",
    "\"extraPaths\"",
    "\"executionEnvironments\"",
    "\"venv\"",
    "\"venvPath\"",
    "\"typeCheckingMode\"",
    "\"pythonVersion\"",
    "\"pythonPlatform\"",
    "\"strict\"",
    "\"extends\"",
];

/// `report*` diagnostic keys.
const REPORT_KEYS: &[&str] = &[
    "\"reportGeneralTypeIssues\"",
    "\"reportInvalidTypeForm\"",
    "\"reportFunctionMemberAccess\"",
    "\"reportMissingImports\"",
    "\"reportMissingModuleSource\"",
    "\"reportInvalidStub\"",
    "\"reportUnsupportedDunderAll\"",
    "\"reportUnusedExpression\"",
    "\"reportWildcardImportFromLibrary\"",
    "\"reportAssertAlwaysTrue\"",
    "\"reportSelfClsParameterName\"",
    "\"reportMissingSuperCall\"",
    "\"reportInvalidSuperCall\"",
    "\"reportPropertyTypeMismatch\"",
    "\"reportImplicitOverride\"",
    "\"reportIncompatibleMethodOverride\"",
    "\"reportIncompatibleVariableOverride\"",
    "\"reportInconsistentConstructor\"",
    "\"reportOverlappingOverload\"",
    "\"reportMissingParameterType\"",
    "\"reportMissingTypeArgument\"",
    "\"reportInvalidTypeVarUse\"",
    "\"reportCallInDefaultInitializer\"",
    "\"reportUnknownParameterType\"",
    "\"reportUnknownArgumentType\"",
    "\"reportUnknownLambdaType\"",
    "\"reportUnknownVariableType\"",
    "\"reportUnknownMemberType\"",
    "\"reportMissingTypeStubs\"",
    "\"reportUnusedImport\"",
    "\"reportUnusedVariable\"",
    "\"reportUnusedClass\"",
    "\"reportUnusedFunction\"",
    "\"reportDuplicateImport\"",
    "\"reportUntypedFunctionDecorator\"",
    "\"reportUntypedClassDecorator\"",
    "\"reportUntypedBaseClass\"",
    "\"reportUntypedNamedTuple\"",
    "\"reportPrivateUsage\"",
    "\"reportPrivateImportUsage\"",
    "\"reportConstantRedefinition\"",
    "\"reportDeprecated\"",
    "\"reportInconsistentOverload\"",
    "\"reportUndefinedVariable\"",
    "\"reportInvalidStringEscapeSequence\"",
    "\"reportMissingDocstring\"",
    "\"reportInvalidDocstring\"",
    "\"reportAssertTypeFailure\"",
    "\"reportGeneralTypeIssue\"",
    "\"reportImplicitStringConcatenation\"",
    "\"reportMatchNotExhaustive\"",
    "\"reportOptionalSubscript\"",
    "\"reportOptionalMemberAccess\"",
    "\"reportOptionalCall\"",
    "\"reportOptionalIterable\"",
    "\"reportOptionalContextManager\"",
    "\"reportOptionalOperand\"",
    "\"reportTypedDictNotRequiredAccess\"",
    "\"reportShadowedImports\"",
    "\"reportUnnecessaryCast\"",
    "\"reportUnnecessaryComparison\"",
    "\"reportUnnecessaryContains\"",
    "\"reportUnnecessaryTypeIgnoreComment\"",
    "\"reportUnnecessaryIsInstance\"",
    "\"reportUnhashable\"",
    "\"reportUnusedCoroutine\"",
    "\"reportUninitializedInstanceVariable\"",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a pyrightconfig.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = CORE_KEYS
        .iter()
        .chain(REPORT_KEYS.iter())
        .filter(|k| key_present(t, k))
        .count();
    hits >= 2
}

impl Pyright {
    /// Count categories. Returns `None` when the input does not look like
    /// a pyrightconfig.json.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            core_keys: 0,
            report_keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") {
                c.comments += 1;
            } else if tr.contains("\":") {
                c.assignments += 1;
            }
        }
        for k in CORE_KEYS {
            c.core_keys += t.matches(k).count();
        }
        for k in REPORT_KEYS {
            c.report_keys += t.matches(k).count();
        }
        c.keys = c.core_keys + c.report_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"{\n  \"include\": [\"src\"],\n  \"exclude\": [\"**/__pycache__\"],\n  \"typeCheckingMode\": \"strict\",\n  \"stubPath\": \"typings\",\n  \"reportMissingImports\": \"error\",\n  \"reportUndefinedVariable\": \"error\",\n  \"reportOptionalSubscript\": \"warning\",\n  \"executionEnvironments\": [\n    {\"root\": \".\", \"pythonVersion\": \"3.11\"}\n  ]\n}\n";
        assert!(detect(b));
        let c = Pyright::parse(b).unwrap();
        assert_eq!(c.report_keys, 3);
        assert!(c.core_keys >= 4);
        assert_eq!(c.assignments, 9);
    }

    #[test]
    fn rejects_json() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(Pyright::parse(b"").is_none());
    }
}
