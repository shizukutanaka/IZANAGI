//! Angular `angular.json` workspace census.
//!
//! angular.json is JSON(C): `"$schema"`(`@angular/cli` schema),
//! `"projects"`(プロジェクト名キーのネスト), `"architect"`/`"targets"`
//! ブロック(`"builder"`/`"executor"`), `"schematics"`, `"cli"`,
//! `"defaultProject"`, `"newProjectRoot"`.
//!
//! ```rust
//! let c = izanagi_kit::angularconf::Angularconf::parse(b"{ \"projects\": { \"app\": { \"architect\": {} } } }").unwrap();
//! assert_eq!(c.projects, 1);
//! ```

/// `angular.json` census.
#[derive(Debug, Clone)]
pub struct Angularconf {
    /// Project-name keys inside `"projects"`.
    pub projects: usize,
    /// Target-name keys inside `"architect"`/`"targets"` blocks.
    pub targets: usize,
    /// `"builder":`/`"executor":` entries.
    pub builders: usize,
    /// All `"key":` occurrences.
    pub keys: usize,
    /// `//`/`/*` comments.
    pub comments: usize,
}

enum Tok {
    Chr(u8),
    Named(usize, usize), // start,end into source
}

fn toks(t: &str) -> Vec<Tok> {
    let b = t.as_bytes();
    let mut v = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        match b[i] {
            b'/' if i + 1 < b.len() && b[i + 1] == b'/' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < b.len() && b[i + 1] == b'*' => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(b.len());
            }
            b'"' => {
                let start = i + 1;
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                v.push(Tok::Named(start, i));
                i += 1;
            }
            b'{' | b'}' | b'[' | b']' | b':' | b',' => {
                v.push(Tok::Chr(b[i]));
                i += 1;
            }
            _ => i += 1,
        }
    }
    v
}

/// Whether the buffer looks like an angular.json file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"projects\"")
        && (t.contains("\"architect\"")
            || t.contains("\"targets\"")
            || t.contains("\"builder\"")
            || t.contains("\"executor\"")
            || t.contains("angular")
            || t.contains("\"schematics\"")
            || t.contains("\"cli\""))
}

impl Angularconf {
    /// Parse an angular.json file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            projects: 0,
            targets: 0,
            builders: 0,
            keys: 0,
            comments: 0,
        };
        c.comments = t
            .lines()
            .filter(|l| {
                let s = l.trim();
                s.starts_with("//") || s.starts_with("/*")
            })
            .count();
        let tk = toks(t);
        let mut depth = 0usize;
        let mut proj_ctx: Option<usize> = None; // depth where project-name keys live
        let mut tgt_ctx: Option<usize> = None;
        let mut i = 0usize;
        while i < tk.len() {
            match &tk[i] {
                Tok::Named(s, e) => {
                    let key = &t[*s..*e];
                    if matches!(tk.get(i + 1), Some(Tok::Chr(b':'))) {
                        c.keys += 1;
                        if key == "builder" || key == "executor" {
                            c.builders += 1;
                        }
                        let opens_obj = matches!(tk.get(i + 2), Some(Tok::Chr(b'{')));
                        if opens_obj && (key == "projects") {
                            proj_ctx = Some(depth + 1);
                        }
                        if opens_obj && (key == "architect" || key == "targets" || key == "i18n") {
                            tgt_ctx = Some(depth + 1);
                        }
                        if opens_obj {
                            if proj_ctx == Some(depth) && key != "projects" {
                                c.projects += 1;
                            }
                            if tgt_ctx == Some(depth) && key != "architect" && key != "targets" {
                                c.targets += 1;
                            }
                        }
                        i += 2;
                        continue;
                    }
                    i += 1;
                }
                Tok::Chr(b'{') | Tok::Chr(b'[') => {
                    depth += 1;
                    i += 1;
                }
                Tok::Chr(b'}') | Tok::Chr(b']') => {
                    depth = depth.saturating_sub(1);
                    if proj_ctx.is_some_and(|d| depth < d) {
                        proj_ctx = None;
                    }
                    if tgt_ctx.is_some_and(|d| depth < d) {
                        tgt_ctx = None;
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_angular_json() {
        let b = concat!(
            "{\n",
            "  \"$schema\": \"./node_modules/@angular/cli/lib/config/schema.json\",\n",
            "  \"version\": 1,\n",
            "  \"newProjectRoot\": \"projects\",\n",
            "  \"projects\": {\n",
            "    \"myapp\": {\n",
            "      \"projectType\": \"application\",\n",
            "      \"architect\": {\n",
            "        \"build\": { \"builder\": \"@angular/build:application\" },\n",
            "        \"serve\": { \"builder\": \"@angular/build:dev-server\" },\n",
            "        \"test\": { \"executor\": \"@angular/build:karma\" }\n",
            "      }\n",
            "    },\n",
            "    \"lib1\": {\n",
            "      \"projectType\": \"library\",\n",
            "      \"architect\": {\n",
            "        \"build\": { \"builder\": \"@angular/build:ng-packagr\" }\n",
            "      }\n",
            "    }\n",
            "  },\n",
            "  \"cli\": { \"analytics\": false }\n",
            "}\n",
        );
        let c = Angularconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.projects, 2);
        assert_eq!(c.targets, 4);
        assert_eq!(c.builders, 4);
    }

    #[test]
    fn rejects_other() {
        assert!(Angularconf::parse(b"{ \"foo\": {} }").is_none());
    }
}
