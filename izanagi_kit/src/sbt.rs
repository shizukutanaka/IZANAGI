//! sbt `build.sbt` census (Scala).
//!
//! `key := value` assignments: `name`, `version`, `scalaVersion`,
//! `organization`, `libraryDependencies` (`+=`/`++=`/`%`/`%%`/`%%%`),
//! `scalacOptions`, `javacOptions`, `resolvers`, `mainClass`,
//! `publishMavenStyle`, `publishTo`, `licenses`, `homepage`,
//! `developers`, `scmInfo`, `lazy val`, `enablePlugins`,
//! `disablePlugins`, `dependsOn`, `aggregate`, `settings`,
//! `ThisBuild`, `Compile`, `Test`, `IntegrationTest`,
//! `crossScalaVersions`, `crossPaths`, `target`, `sourceDirectory`,
//! `resourceDirectory`, `fork`, `javaOptions`, `envVars`,
//! `assembly`, `assemblyMergeStrategy`, `testFrameworks`,
//! `coverageEnabled`, `wartremoverErrors`, `addCompilerPlugin`,
//! `libraryDependencies += "org" %% "x" % "v"`,
//! `dependencyOverrides`, `dependencyCheckSuppressions`,
//! `evictionErrorLevel`, `useCoursier`, `csrConfiguration`,
//! `Global`/`in x`/`root`/`aggregate in`/`settings(...)`.
//!
//! ```rust
//! let k = b"ThisBuild / scalaVersion := \"3.3.0\"\nlazy val root = (project in file(\".\"))\n  .settings(\n    name := \"x\",\n    libraryDependencies += \"org\" %% \"y\" % \"1.0\",\n  )\n";
//! assert!(izanagi_kit::sbt::detect(k));
//! ```

/// build.sbt census.
#[derive(Debug, Clone)]
pub struct Sbt {
    /// `x :=`/`x +=`/`x ++=` setting lines.
    pub settings: usize,
    /// `%%`/`%` Maven-coordinate lines.
    pub dependencies: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "scalaVersion",
    "libraryDependencies",
    "scalacOptions",
    "javacOptions",
    "crossScalaVersions",
    "publishMavenStyle",
    "publishTo",
    "assemblyMergeStrategy",
    "testFrameworks",
    "coverageEnabled",
    "wartremoverErrors",
    "addCompilerPlugin",
    "dependencyOverrides",
    "evictionErrorLevel",
    "useCoursier",
    "csrConfiguration",
    "enablePlugins",
    "disablePlugins",
    "dependsOn",
    "aggregate",
    "lazy val",
    "ThisBuild",
    "Compile",
    "IntegrationTest",
    "updateOptions",
    "concurrentRestrictions",
    "onLoad",
    "compileOrder",
    "resolvers",
    "mainClass",
    "scmInfo",
    "developers",
    "fork",
    "javaOptions",
    "envVars",
    "crossPaths",
    "sourceDirectory",
    "resourceDirectory",
    "moduleName",
    "normalizedName",
    "binaryScalaVersion",
    "autoCompilerPlugins",
    "ivyConfigurations",
    "managedSourceDirectories",
    "apiMappings",
    "docExclusions",
    "excludedJars",
    "dependencyCheck",
    "unmanagedSourceDirectories",
    "sourcesInBase",
    "skip in publish",
    "publishArtifact",
    "credentials",
    "pomIncludeRepository",
    "pomExtra",
];

fn marker(line: &str) -> bool {
    let s = line.trim();
    if s.is_empty() || s.starts_with("//") || s.starts_with('#') || s.starts_with('*') {
        return false;
    }
    if s.contains(" %% ") || s.contains(" %%% ") || s.contains("libraryDependencies") {
        return true;
    }
    KEYS.iter().any(|k| s.contains(k))
}

/// Detect a `build.sbt` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `:=` settings + `%%` coordinates + `scalaVersion`/
    // `lazy val`/`enablePlugins` are sbt-exclusive.
    let mut n = 0usize;
    for line in t.lines() {
        if marker(line) {
            n += 1;
        }
    }
    n >= 2
}

impl Sbt {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            dependencies: 0,
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
            if s.contains(":=") || s.contains("+=") || s.contains("++=") {
                c.settings += 1;
            }
            if s.contains(" %% ") || s.contains(" %%% ") {
                c.dependencies += 1;
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
        let b = b"ThisBuild / scalaVersion := \"3.3.0\"\nlazy val root = (project in file(\".\"))\n  .settings(\n    name := \"x\",\n    libraryDependencies += \"org\" %% \"y\" % \"1.0\",\n  )\n";
        assert!(detect(b));
        let c = Sbt::parse(b).unwrap();
        assert!(c.settings >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"val x = 1\nval y = 2\n"));
        assert!(!detect(
            b"// scalaVersion := \"3\"\n// libraryDependencies += x\nval z = 1\n"
        ));
    }
}
