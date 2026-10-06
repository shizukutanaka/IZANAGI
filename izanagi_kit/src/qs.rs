//! Q# (Microsoft quantum language) source parser.
//!
//! Counts `namespace`/`open` directives, `operation`/`function`/
//! `newtype` declarations, `@EntryPoint()`/`@Config()` attributes,
//! `use`/`borrow`/`mutable`/`set` bindings and intrinsic calls.
//!
//! ```
//! use izanagi_kit::qs::Qs;
//! let src = b"namespace Bell {\n    open Microsoft.Quantum.Intrinsic;\n    @EntryPoint()\n    operation Main() : Result[] {\n        use q = Qubit();\n        H(q);\n        return [M(q)];\n    }\n}\n";
//! assert!(izanagi_kit::qs::detect(src));
//! let q = Qs::parse(src).unwrap();
//! assert_eq!(q.namespaces, 1);
//! assert_eq!(q.opens, 1);
//! assert_eq!(q.operations, 1);
//! assert_eq!(q.attributes, 1);
//! ```

/// Parsed census of a Q# source file.
#[derive(Debug, Clone)]
pub struct Qs {
    /// `namespace Name {` declarations.
    pub namespaces: usize,
    /// `open X.Y;` directives.
    pub opens: usize,
    /// `operation Name(...) : Ret` declarations.
    pub operations: usize,
    /// `function Name(...) : Ret` declarations.
    pub functions: usize,
    /// `newtype Name = ...` declarations.
    pub newtypes: usize,
    /// `@Attr(...)` attributes (`EntryPoint`, `Config`, `Test`, ...).
    pub attributes: usize,
    /// `use`/`borrow` qubit bindings.
    pub qubit_bindings: usize,
    /// `mutable` bindings.
    pub mutables: usize,
    /// `set` assignments.
    pub sets: usize,
    /// `M(`/`Measure`/`Reset`/`ResetAll` measurement calls.
    pub measurements: usize,
    /// `if`/`elif`/`else`/`for`/`repeat`/`until`/`while`/`within`/`apply` control statements.
    pub control_flow: usize,
    /// `//` comment lines.
    pub comments: usize,
}
fn code_has(t: &str, needle: &str) -> bool {
    // `//`/`/*` コメント行内の言及は証拠にしない。
    t.lines().any(|l| {
        let l = l.trim_start();
        !l.starts_with("//") && !l.starts_with("/*") && l.contains(needle)
    })
}

/// Returns `true` when `b` looks like a Q# source file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    code_has(t, "namespace ")
        && (code_has(t, "open Microsoft.") || code_has(t, "operation ") || code_has(t, "function "))
}

impl Qs {
    /// Parses a Q# source; `None` when `namespace` is absent.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let mut q = Self {
            namespaces: 0,
            opens: 0,
            operations: 0,
            functions: 0,
            newtypes: 0,
            attributes: 0,
            qubit_bindings: 0,
            mutables: 0,
            sets: 0,
            measurements: 0,
            control_flow: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with("//") {
                q.comments += 1;
                continue;
            }
            let kw = l.split_whitespace().next().unwrap_or("");
            match kw {
                "namespace" => q.namespaces += 1,
                "open" => q.opens += 1,
                "operation" => q.operations += 1,
                "function" => q.functions += 1,
                "newtype" => q.newtypes += 1,
                "use" | "borrow" => q.qubit_bindings += 1,
                "mutable" => q.mutables += 1,
                "set" => q.sets += 1,
                "if" | "elif" | "else" | "for" | "repeat" | "until" | "while" | "within"
                | "apply" => q.control_flow += 1,
                _ => {
                    if kw.starts_with('@') {
                        q.attributes += 1;
                    } else if kw.starts_with("M(")
                        || kw.starts_with("MReset")
                        || kw == "Measure"
                        || kw.starts_with("Measure(")
                        || kw.starts_with("Reset")
                        || kw.starts_with("MResetZ")
                    {
                        q.measurements += 1;
                    }
                }
            }
        }
        (q.namespaces > 0).then_some(q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"// namespace Foo\n// operation A() : Unit {}\n"));
    }

    #[test]
    fn detects_namespace() {
        assert!(detect(
            b"namespace X { open Microsoft.Quantum.Intrinsic; }\n"
        ));
        assert!(!detect(b"fn main() {}"));
    }

    #[test]
    fn counts_declarations() {
        let src = b"namespace A {\nopen B;\nopen C;\nfunction F(x : Int) : Int { return x; }\nnewtype T = (Int, Double);\n}\n";
        let q = Qs::parse(src).unwrap();
        assert_eq!(q.opens, 2);
        assert_eq!(q.functions, 1);
        assert_eq!(q.newtypes, 1);
    }

    #[test]
    fn parse_none_without_namespace() {
        assert!(Qs::parse(b"open X;\n").is_none());
    }
}
