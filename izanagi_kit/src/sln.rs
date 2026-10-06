//! Visual Studio `.sln` solution file census.
//!
//! `Microsoft Visual Studio Solution File, Format Version N.NN` header
//! + `# Visual Studio Version N` + `Project("{type-guid}") = "name",
//! "path", "{proj-guid}"` … `EndProject` +
//!
//! `Global`/`GlobalSection(…)` blocks.
//!
//! ```rust
//! let s = b"Microsoft Visual Studio Solution File, Format Version 12.00\n# Visual Studio Version 17\nProject(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"App\", \"src\\\\App.csproj\", \"{A1B2C3D4-0000-0000-0000-000000000001}\"\nEndProject\nGlobal\nEndGlobal\n";
//! assert!(izanagi_kit::sln::detect(s));
//! let c = izanagi_kit::sln::Sln::parse(s).unwrap();
//! assert_eq!(c.projects, 1);
//! ```

/// .sln census.
#[derive(Debug, Clone)]
pub struct Sln {
    /// `Project(...) = …` lines.
    pub projects: usize,
    /// `Global`/`GlobalSection`/`EndGlobal(Section)` marker lines.
    pub globals: usize,
    /// Header lines (`Microsoft Visual Studio…`, `# Visual Studio…`,
    /// `MinimumVisualStudioVersion`, `VisualStudioVersion`).
    pub headers: usize,
}

fn project_line(tr: &str) -> bool {
    tr.starts_with("Project(") && tr.contains(") =")
}

fn global_line(tr: &str) -> bool {
    tr.starts_with("GlobalSection(")
        || tr == "Global"
        || tr == "EndGlobal"
        || tr.starts_with("GlobalSection")
        || tr == "EndGlobalSection"
        || tr.starts_with("\tGlobalSection")
}

/// Detect a `.sln` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut projects = 0usize;
    let mut header = false;
    let mut globals = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("Microsoft Visual Studio Solution File") {
            header = true;
            continue;
        }
        if project_line(tr) {
            projects += 1;
            continue;
        }
        if global_line(tr) {
            globals += 1;
        }
    }
    header || (projects >= 1 && globals >= 2) || projects >= 2
}

impl Sln {
    /// Count projects/globals/headers. Returns `None` when the input
    /// does not look like a `.sln`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            projects: 0,
            globals: 0,
            headers: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("Microsoft Visual Studio Solution File")
                || tr.starts_with("# Visual Studio Version")
                || tr.starts_with("VisualStudioVersion")
                || tr.starts_with("MinimumVisualStudioVersion")
            {
                c.headers += 1;
                continue;
            }
            if project_line(tr) {
                c.projects += 1;
                continue;
            }
            if global_line(tr) {
                c.globals += 1;
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
        let b = b"\nMicrosoft Visual Studio Solution File, Format Version 12.00\n# Visual Studio Version 17\nVisualStudioVersion = 17.0.31903.59\nMinimumVisualStudioVersion = 10.0.40219.1\nProject(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"App\", \"src\\App.csproj\", \"{A1B2C3D4-0000-0000-0000-000000000001}\"\nEndProject\nProject(\"{2150E333-8FDC-42A3-9474-1A3956D46DE8}\") = \"Solution Items\", \"Solution Items\", \"{B1B2C3D4-0000-0000-0000-000000000002}\"\nEndProject\nGlobal\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Debug|Any CPU\n\tEndGlobalSection\n\tGlobalSection(NestedProjects) = preSolution\n\tEndGlobalSection\nEndGlobal\n";
        assert!(detect(b));
        let c = Sln::parse(b).unwrap();
        assert_eq!(c.projects, 2);
        assert!(c.globals >= 4);
        assert_eq!(c.headers, 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"<Project></Project>"));
        assert!(!detect(b"key=value\n"));
        assert!(Sln::parse(b"").is_none());
    }
}
