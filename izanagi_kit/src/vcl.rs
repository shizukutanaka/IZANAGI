//! Varnish VCL (Varnish Configuration Language) census.
//!
//! A VCL file starts `vcl 4.x;` then `backend <name> { .host = "..."; }`,
//! `acl <name> { "..."; }`, `probe`, `director`, `import`, `include`
//! and `sub vcl_<hook> { if (...) { ... } set <obj>.<f> = <v>; return (...); }`
//! routines. `parse` counts declarations and statement classes.
//!
//! ```rust
//! let v = concat!(
//!     "vcl 4.1;\n",
//!     "backend default {\n",
//!     "    .host = \"127.0.0.1\";\n",
//!     "    .port = \"8080\";\n",
//!     "}\n",
//!     "acl purge { \"127.0.0.1\"; }\n",
//!     "sub vcl_recv {\n",
//!     "    if (req.method == \"PURGE\") {\n",
//!     "        return (purge);\n",
//!     "    }\n",
//!     "    return (hash);\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::vcl::Vcl::parse(v.as_bytes()).unwrap();
//! assert_eq!(c.backends, 1);
//! assert_eq!(c.subs, 1);
//! ```

/// Varnish VCL census.
#[derive(Debug, Clone)]
pub struct Vcl {
    /// `vcl 4.x;`/`vcl 4.x #` marker lines.
    pub versions: usize,
    /// `backend <name>` blocks.
    pub backends: usize,
    /// `acl <name>`/`probe <name>` blocks.
    pub acls: usize,
    /// `sub vcl_*`/`sub <name>` routines.
    pub subs: usize,
    /// `import`/`include` lines.
    pub imports: usize,
    /// `director`/`new `/`vsc`/`counter` declarations.
    pub directors: usize,
    /// `.field = value;` member assignments.
    pub fields: usize,
    /// `set`/`unset`/`synthetic`/`call` statements.
    pub sets: usize,
    /// `if (`/`elsif`/`else`/`switch`/`case` control statements.
    pub conditionals: usize,
    /// `return`/`ban`/`hash_data`/`rollback`/`log`/`std.` calls.
    pub returns: usize,
}

/// Whether the buffer looks like a VCL file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("vcl 4.")
        || t.contains("vcl_recv")
        || t.contains("vcl_backend_response")
        || (t.contains("backend ") && t.contains(".host"))
}

impl Vcl {
    /// Parse a VCL file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            versions: 0,
            backends: 0,
            acls: 0,
            subs: 0,
            imports: 0,
            directors: 0,
            fields: 0,
            sets: 0,
            conditionals: 0,
            returns: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with("//") || s == "}" || s == "{" {
                continue;
            }
            if s.starts_with("vcl ") && s.ends_with(';') {
                c.versions += 1;
                continue;
            }
            if s.starts_with("backend ") {
                c.backends += 1;
                continue;
            }
            if s.starts_with("acl ") || s.starts_with("probe ") {
                c.acls += 1;
                continue;
            }
            if s.starts_with("sub ") {
                c.subs += 1;
                continue;
            }
            if s.starts_with("import") || s.starts_with("include") {
                c.imports += 1;
                continue;
            }
            if s.starts_with("director")
                || s.starts_with("new ")
                || s.starts_with("vsc")
                || s.starts_with("counter ")
            {
                c.directors += 1;
                continue;
            }
            if s.starts_with("set ")
                || s.starts_with("unset ")
                || s.starts_with("synthetic")
                || s.starts_with("call ")
                || s.starts_with("regsub")
                || s.starts_with("std.")
            {
                c.sets += 1;
                continue;
            }
            if s.starts_with("if (")
                || s.starts_with("if(")
                || s.starts_with("elsif")
                || s.starts_with("else")
                || s.starts_with("switch")
                || s.starts_with("case ")
                || s == "else {"
            {
                c.conditionals += 1;
                continue;
            }
            if s.starts_with("return")
                || s.starts_with("ban")
                || s.starts_with("hash_data")
                || s.starts_with("rollback")
                || s.starts_with("fail")
                || s.starts_with("retry")
                || s.starts_with("log ")
                || s.starts_with("debug.")
            {
                c.returns += 1;
                continue;
            }
            if s.starts_with('.') && s.contains('=') {
                c.fields += 1;
                continue;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vcl() {
        let b = concat!(
            "vcl 4.1;\n",
            "import std;\n",
            "backend default {\n",
            "    .host = \"127.0.0.1\";\n",
            "    .port = \"8080\";\n",
            "}\n",
            "backend api {\n",
            "    .host = \"10.0.0.1\";\n",
            "}\n",
            "acl purge {\n",
            "    \"localhost\";\n",
            "    \"127.0.0.1\";\n",
            "}\n",
            "sub vcl_recv {\n",
            "    if (req.method == \"PURGE\") {\n",
            "        return (purge);\n",
            "    }\n",
            "    set req.http.X = \"1\";\n",
            "    return (hash);\n",
            "}\n",
            "sub vcl_backend_response {\n",
            "    set beresp.ttl = 10m;\n",
            "    return (deliver);\n",
            "}\n",
        );
        let c = Vcl::parse(b.as_bytes()).unwrap();
        assert_eq!(c.versions, 1);
        assert_eq!(c.backends, 2);
        assert_eq!(c.acls, 1);
        assert_eq!(c.subs, 2);
        assert_eq!(c.imports, 1);
        assert_eq!(c.fields, 3);
        assert_eq!(c.sets, 2);
        assert_eq!(c.conditionals, 1);
        assert_eq!(c.returns, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Vcl::parse(b"foo = 1").is_none());
    }
}
