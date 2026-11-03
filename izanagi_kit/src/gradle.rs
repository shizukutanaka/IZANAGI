//! Gradle `build.gradle`/`settings.gradle` census (Groovy DSL).
//!
//! Markers: `plugins { }`/`apply plugin:`/`apply from:`,
//! `dependencies { }` configurations `implementation`/`api`/
//! `testImplementation`/`compileOnly`/`runtimeOnly`/`kapt`/
//! `annotationProcessor`/`classpath`, `repositories { }` with
//! `mavenCentral()`/`jcenter()`/`google()`/`maven()`/
//! `gradlePluginPortal()`/`mavenLocal()`, `allprojects`/`subprojects`/
//! `buildscript`, `rootProject.name`, `include`, `includeBuild`,
//! `pluginManagement`, `dependencyResolutionManagement`,
//! `sourceCompatibility`/`targetCompatibility`, `group`, `version`,
//! `task `, `tasks.register`, `publishing`, `signing`,
//! `java {`, `kotlin(`, `kotlin {`, `application {`, `configurations {`,
//! `sourceSets {`, `jar {`, `test {`, `jacoco`, `findbugs`, `checkstyle`,
//! `sonarqube`, `gradle.projectsEvaluated`, `ext.`.
//!
//! ```rust
//! let k = b"plugins {\n    id 'java'\n}\nrepositories {\n    mavenCentral()\n}\ndependencies {\n    implementation 'x:y:1'\n    testImplementation 'j:j:4'\n}\n";
//! assert!(izanagi_kit::gradle::detect(k));
//! ```

/// Gradle build file census.
#[derive(Debug, Clone)]
pub struct Gradle {
    /// `x {`/call/assignment lines.
    pub lines: usize,
    /// `key = value`/`:=` assignments.
    pub settings: usize,
    /// recognised Gradle markers present.
    pub keys: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "mavenCentral()",
    "jcenter()",
    "google()",
    "gradlePluginPortal()",
    "mavenLocal()",
    "maven {",
    "ivy {",
    "flatDir {",
    "implementation",
    "testImplementation",
    "api",
    "runtimeOnly",
    "compileOnly",
    "kapt",
    "annotationProcessor",
    "classpath",
    "compile",
    "testCompile",
    "provided",
    "integrationTestImplementation",
    "functionalTestImplementation",
    "rootProject.name",
    "includeBuild",
    "pluginManagement",
    "dependencyResolutionManagement",
    "allprojects",
    "subprojects",
    "buildscript",
    "sourceCompatibility",
    "targetCompatibility",
    "tasks.register",
    "tasks.withType",
    "sourceSets",
    "publishing",
    "signing",
    "apply plugin",
    "apply from:",
    "rootProject",
    "gradle.projectsEvaluated",
    "gradle.projectsLoaded",
    "evaluationDependsOn",
    "include ",
    "enableFeaturePreview",
    "featurePreview",
    "versionCatalogs",
];

const WEAK: &[&str] = &[
    "plugins",
    "dependencies",
    "repositories",
    "configurations",
    "java",
    "kotlin",
    "application",
    "task ",
    "jar",
    "test",
    "jacoco",
    "checkstyle",
    "findbugs",
    "sonarqube",
    "group",
    "version",
    "name",
    "description",
    "ext",
    "mainClassName",
    "manifest",
    "baseName",
    "archivesBaseName",
    "compileJava",
    "compileTestJava",
    "wrapper",
    "distributionUrl",
    "useJUnitPlatform",
    "dependsOn",
    "buildDir",
];

/// `k` appears in `s` as a configuration token: it is bounded by
/// non-identifier characters on both sides (`api 'g:a:v'` matches,
/// `apiVersion:` does not), and it is not immediately followed by `:` —
/// `api:`/`dependencies:` are YAML/dict keys, never Gradle calls.
fn contains_token(s: &str, k: &str) -> bool {
    let id = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut rest = s;
    while let Some(i) = rest.find(k) {
        let a = i + k.len();
        let left_ok = i == 0 || !id(rest.as_bytes()[i - 1]);
        let right = rest.as_bytes().get(a);
        let right_ok = right.map_or(true, |&c| !id(c) && c != b':');
        if left_ok && right_ok {
            return true;
        }
        rest = &rest[i + 1..];
    }
    false
}

/// The trimmed line starts with `k` used as a configuration token —
/// `name = "x"` and `version '1.0'` qualify, `names:`/`versions:`/
/// `versionCatalogs` do not (see `contains_token` for the boundary rule).
fn starts_with_token(s: &str, k: &str) -> bool {
    if !s.starts_with(k) {
        return false;
    }
    let id = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    match s.as_bytes().get(k.len()) {
        None => true,
        Some(&c) => !id(c) && c != b':',
    }
}

fn marker(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with("//") || s.starts_with('#') || s.starts_with('*') {
        return None;
    }
    if let Some(k) = STRONG.iter().find(|&&k| contains_token(s, k)) {
        return Some(k);
    }
    WEAK.iter().find(|&&k| starts_with_token(s, k)).copied()
}

fn is_strong(m: &str) -> bool {
    STRONG.contains(&m)
}

/// Detect a Gradle build file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // dependency configurations (`implementation`/`api`/
    // `testImplementation`), repo helpers (`mavenCentral()`), and
    // `rootProject`/`include` are Gradle-exclusive.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(m) = marker(line) {
            if is_strong(m) {
                strong += 1;
            } else {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Gradle {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            lines: 0,
            settings: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            c.lines += 1;
            if s.contains('=') && !s.contains("==") {
                c.settings += 1;
            }
            if marker(line).is_some() {
                c.keys += 1;
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
        let b = b"plugins {\n    id 'java'\n}\nrepositories {\n    mavenCentral()\n}\ndependencies {\n    implementation 'x:y:1'\n    testImplementation 'j:j:4'\n}\n";
        assert!(detect(b));
        let c = Gradle::parse(b).unwrap();
        assert!(c.keys >= 5);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"x = 1\ny = 2\n"));
        assert!(!detect(
            b"// mavenCentral()\n// implementation 'x'\nplugins {\n}\n"
        ));
    }

    #[test]
    fn rejects_yaml_token_lookalikes() {
        // `apiVersion` contains `api` but is not a Gradle call
        assert!(!detect(
            b"apiVersion: v1\nkind: Config\nmetadata:\n  name: x\n  names:\n  versions:\n"
        ));
        // `dependencies:`/`plugins:`/`include:` are YAML keys, not calls
        assert!(!detect(
            b"plugins:\n  - a\ndependencies:\n  - b\ninclude:\n  - c\n"
        ));
    }
}
