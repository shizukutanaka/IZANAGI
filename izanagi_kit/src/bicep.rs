//! Parser for Azure Bicep files (`.bicep`).
//!
//! Counts `param`/`var`/`resource`/`module`/`output`/`type`/`func`
//! declarations, `@` decorators, `existing` references, and comments.
//!
//! ```
//! let b = b"targetScope = 'resourceGroup'\nparam env string\nresource st 'Microsoft.Storage/storageAccounts@2023-01-01' = {\n  name: 's'\n}\noutput id string = st.id\n";
//! assert!(izanagi_kit::bicep::detect(b));
//! let x = izanagi_kit::bicep::Bicep::parse(b).unwrap();
//! assert_eq!(x.params, 1);
//! assert_eq!(x.resources, 1);
//! assert_eq!(x.target_scope, "resourceGroup");
//! ```

/// Parsed Bicep file summary.
#[derive(Debug, Clone)]
pub struct Bicep {
    /// `targetScope` value (empty when absent).
    pub target_scope: String,
    /// `param` declarations.
    pub params: usize,
    /// `var` declarations.
    pub vars: usize,
    /// `resource` declarations.
    pub resources: usize,
    /// `module` declarations.
    pub modules: usize,
    /// `output` declarations.
    pub outputs: usize,
    /// User-defined `type` declarations.
    pub types: usize,
    /// User-defined `func` declarations.
    pub functions: usize,
    /// `existing` references.
    pub existing: usize,
    /// `@decorator` lines.
    pub decorators: usize,
    /// `//` and `/*` comment lines.
    pub comments: usize,
}

/// Returns `true` when `b` looks like Bicep source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    if t.contains("targetScope") {
        return true;
    }
    (t.contains("resource ") && t.contains("'@") && t.contains("= "))
        || (t.contains("param ") && t.contains("output ") && t.contains("= "))
}

impl Bicep {
    /// Parses `b` as a Bicep file.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let mut x = Self {
            target_scope: String::new(),
            params: 0,
            vars: 0,
            resources: 0,
            modules: 0,
            outputs: 0,
            types: 0,
            functions: 0,
            existing: 0,
            decorators: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim_start();
            if tr.starts_with("//") || tr.starts_with("/*") {
                x.comments += 1;
                continue;
            }
            if tr.starts_with('@') {
                x.decorators += 1;
            }
            if tr.starts_with("param ") {
                x.params += 1;
            } else if tr.starts_with("var ") {
                x.vars += 1;
            } else if tr.starts_with("resource ") {
                x.resources += 1;
                if tr.contains(" existing ") {
                    x.existing += 1;
                }
            } else if tr.starts_with("module ") {
                x.modules += 1;
            } else if tr.starts_with("output ") {
                x.outputs += 1;
            } else if tr.starts_with("type ") {
                x.types += 1;
            } else if tr.starts_with("func ") {
                x.functions += 1;
            } else if tr.starts_with("targetScope") {
                x.target_scope = tr
                    .split('=')
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .trim_matches(|c| c == '\'' || c == '"')
                    .to_string();
            }
        }
        Some(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"// infra
targetScope = 'resourceGroup'
@description('env name')
param env string
param count int = 3
var name = 'n'
resource st 'Microsoft.Storage/storageAccounts@2023-01-01' = {
  name: 's'
}
resource plan 'Microsoft.Web/serverfarms@2022-03-01' existing = {
  name: 'p'
}
module net 'net.bicep' = {
  name: 'n'
}
type my = {
  id: string
}
func greet() string => 'hi'
output id string = st.id
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"resource x"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let x = Bicep::parse(SRC).unwrap();
        assert_eq!(x.target_scope, "resourceGroup");
        assert_eq!(x.params, 2);
        assert_eq!(x.vars, 1);
        assert_eq!(x.resources, 2);
        assert_eq!(x.existing, 1);
        assert_eq!(x.modules, 1);
        assert_eq!(x.outputs, 1);
        assert_eq!(x.types, 1);
        assert_eq!(x.functions, 1);
        assert_eq!(x.decorators, 1);
        assert_eq!(x.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Bicep::parse(b"\x01\x02").is_none());
    }
}
