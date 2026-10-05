//! Clojure Leiningen `project.clj` の検出と構造カウント。
//!
//! `(defproject name "version" ...)` フォームと `:description`/`:dependencies`/
//! `:plugins`/`:profiles`/`:repositories`/`:main`/`:aot` 等のキーワードを
//! 識別する。コメントは `;`。
//!
//! ```
//! let c = izanagi_kit::leiningen::parse(
//!     b"(defproject demo \"0.1\"\n  :description \"d\"\n  :dependencies [[org.clojure/clojure \"1.11\"]]\n  :main demo.core)\n").unwrap();
//! assert_eq!(c.options, 3);
//! assert!(izanagi_kit::leiningen::detect(
//!     b"(defproject a \"1\" :description \"x\" :main a.core)\n"));
//! ```

/// `defproject` 内既知キーワード。
const KEYWORDS: &[&str] = &[
    ":aot",
    ":auto-clean",
    ":bootclasspath",
    ":checksum-deps",
    ":class-data",
    ":clean-targets",
    ":compile-path",
    ":dependencies",
    ":deploy-repositories",
    ":description",
    ":eval-in",
    ":eval-in-leiningen",
    ":exclusions",
    ":global-vars",
    ":hooks",
    ":implicits",
    ":injections",
    ":java-agents",
    ":javac-options",
    ":javac-source",
    ":javac-target",
    ":jvm-opts",
    ":license",
    ":main",
    ":managed-dependencies",
    ":manifest",
    ":middleware",
    ":min-lein-version",
    ":monkeypatch-clojure-test",
    ":native-path",
    ":offline?",
    ":omit-source",
    ":out",
    ":pom-addition",
    ":pom-location",
    ":prep-tasks",
    ":profiles",
    ":plugins",
    ":repositories",
    ":repl-options",
    ":repl-options",
    ":resource-paths",
    ":ring",
    ":root",
    ":scm",
    ":source-paths",
    ":target-path",
    ":test-paths",
    ":test-selectors",
    ":uberjar-exclusions",
    ":uberjar-merge",
    ":uberjar-name",
    ":url",
    ":war-name",
    ":zip-include",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `(def*`/`(ns` トップフォーム行数。
    pub sections: usize,
    /// 既知キーワード出現行数。
    pub options: usize,
    /// `;` コメント行数。
    pub comments: usize,
    /// その他の行数(ベクタ/継続行を含む)。
    pub misc: usize,
}

fn kw_hits(t: &str) -> usize {
    KEYWORDS
        .iter()
        .filter(|k| {
            let pat = format!("{} ", k);
            t.contains(&pat)
                || t.contains(&format!("{}\n", k))
                || t.ends_with(*k)
                || t.contains(&format!("{}[", k))
                || t.contains(&format!("{})", k))
        })
        .count()
}

/// `project.clj` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut defproj = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("(defproject") {
            defproj = true;
        }
        hits += kw_hits(t);
        if defproj && hits >= 2 {
            return true;
        }
    }
    defproj && hits >= 1
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if t.starts_with("(defproject") || t.starts_with("(def") || t.starts_with("(ns") {
            c.sections += 1;
            continue;
        }
        if kw_hits(t) > 0 {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"; project\n(defproject demo \"0.1.0\"\n  :description \"demo\"\n  :url \"https://example.com\"\n  :license {:name \"MIT\"}\n  :dependencies [[org.clojure/clojure \"1.11.1\"]\n                 [ring \"1.10.0\"]]\n  :plugins [[lein-ancient \"1.0\"]]\n  :profiles {:dev {:dependencies [[midje \"1.10\"]]}}\n  :main demo.core\n  :aot [demo.core]\n  :uberjar-name \"demo.jar\")\n";

    #[test]
    fn leiningen() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.options, 9);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_leiningen() {
        assert!(!detect(b"(ns foo)\n(defn a [] 1)\n"));
        assert!(parse(b"text\n").is_none());
    }
}
