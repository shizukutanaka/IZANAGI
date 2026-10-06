//! PHPStan `phpstan.neon` / `phpstan.neon.dist` census.
//!
//! NEON format (YAML-like): `parameters:`/`includes:`/`services:`/
//! `rules:`/`conditionalTags:` top keys plus phpstan-specific parameters
//! (`level`, `paths`, `excludePaths`, `ignoreErrors`, `checkGenericClassInNonGenericObjectType`,
//! `earlyTerminatingMethodCalls`, `reportMaybes` …).
//!
//! ```rust
//! let n = b"parameters:\n\tlevel: 8\n\tpaths:\n\t\t- src\n\tignoreErrors:\n\t\t- '#Call to undefined#'\n";
//! assert!(izanagi_kit::phpstan::detect(n));
//! let c = izanagi_kit::phpstan::Phpstan::parse(n).unwrap();
//! assert!(c.params >= 3);
//! ```

/// phpstan.neon census.
#[derive(Debug, Clone)]
pub struct Phpstan {
    /// `key:`/`key: value` lines matching a known phpstan parameter.
    pub params: usize,
    /// `includes:`/`parameters:`/`services:`/`rules:` top sections.
    pub sections: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Top-level NEON sections.
const SECTION_KEYS: &[&str] = &[
    "conditionalTags",
    "includes",
    "parameters",
    "parametersSchema",
    "rules",
    "services",
];

/// phpstan parameter names (key before `:`; NEON allows both
/// `key: value` and `- key: value` in lists).
const KEYS: &[&str] = &[
    "allowPrivateConstants",
    "arrayParameters",
    "bootstrap",
    "bootstrapFiles",
    "cache",
    "checkAlwaysUsedProperties",
    "checkBenevolentUnionTypes",
    "checkClassCaseSensitivity",
    "checkFunctionNameCase",
    "checkGenericClassInNonGenericObjectType",
    "checkInternalClassCaseSensitivity",
    "checkMissingCallableSignature",
    "checkMissingIterableValueType",
    "checkMissingVarTagTypehint",
    "checkTooWideReturnTypesInProtectedAndPublicMethods",
    "checkUninitializedProperties",
    "deprecationRules",
    "disallowConstructs",
    "earlyTerminatingFunctionCalls",
    "earlyTerminatingMethodCalls",
    "editorUrl",
    "errorFormatter",
    "excludePaths",
    "featureToggles",
    "fileExtensions",
    "identifier",
    "ignoreErrors",
    "ignoreWarnings",
    "inferPrivatePropertyTypeFromConstructor",
    "internalTags",
    "level",
    "memoryLimitFile",
    "message",
    "messages",
    "parallel",
    "paths",
    "polluteScopeWithAlwaysIterableForeach",
    "polluteScopeWithLoopInitialAssignments",
    "reportMaybes",
    "reportMaybesInMethodSignatures",
    "reportMaybesInPropertyPhpDocTypes",
    "reportStaticMethodSignatures",
    "reportWrongPhpDocTypeInVarTag",
    "resultCachePath",
    "scanDirectories",
    "scanFiles",
    "stubFiles",
    "tips",
    "tmpDir",
    "treatPhpDocTypesAsCertain",
    "usePathConstantsAsConstantString",
];

fn neon_key(l: &str) -> Option<&str> {
    let t = l.trim();
    if t.is_empty()
        || t.starts_with('#')
        || t.starts_with('-') && t.len() > 1 && !t[1..].trim_start().contains(':')
    {
        return None;
    }
    let t = t.strip_prefix('-').map(str::trim_start).unwrap_or(t);
    let (k, _) = t.split_once(':')?;
    let k = k.trim();
    if k.is_empty() || k.contains(' ') {
        return None;
    }
    Some(k)
}

/// Detect a `phpstan.neon`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut keys = 0usize;
    let mut secs = 0usize;
    for l in t.lines() {
        if let Some(k) = neon_key(l) {
            if KEYS.contains(&k) {
                keys += 1;
            } else if SECTION_KEYS.contains(&k) {
                secs += 1;
            }
        }
    }
    keys >= 2 || (secs >= 1 && keys >= 1)
}

impl Phpstan {
    /// Count parameters and sections. Returns `None` when the input does
    /// not look like a `phpstan.neon`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            params: 0,
            sections: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = neon_key(l) {
                if SECTION_KEYS.contains(&k) {
                    c.sections += 1;
                } else if KEYS.contains(&k) {
                    c.params += 1;
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
    fn detects() {
        let b = b"# phpstan\nincludes:\n\t- vendor/phpstan/phpstan-strict-rules/rules.neon\nparameters:\n\tlevel: 8\n\tpaths:\n\t\t- src\n\t\t- tests\n\texcludePaths:\n\t\t- src/Generated\n\tignoreErrors:\n\t\t- '#Call to an undefined method#'\n\tcheckMissingIterableValueType: false\n\ttreatPhpDocTypesAsCertain: false\n\ttmpDir: var/cache/phpstan\n";
        assert!(detect(b));
        let c = Phpstan::parse(b).unwrap();
        assert_eq!(c.sections, 2);
        assert!(c.params >= 6);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"parameters:\n\tfoo: 1\n"));
        assert!(!detect(b"key: value\n"));
        assert!(Phpstan::parse(b"").is_none());
    }
}
