//! MSBuild `*.csproj` / `*.props` / `*.targets` / `*.vcxproj` /
//! `Directory.Build.props` / `Directory.Packages.props` XML census.
//!
//! `<Project>` root (Sdk= attr or msbuild xmlns) + `<PropertyGroup>`/
//! `<ItemGroup>`/`<TargetFramework`/`TargetFrameworks`/
//! `<PackageReference`/`<ProjectReference`/`<Reference`/`<Compile`/
//! `<Import `/`<Target `/`<UsingTask`/`<DefineConstants`/`<OutputType`/
//! `<NuGetAudit`… elements.
//!
//! ```rust
//! let m = br#"<Project Sdk=\"Microsoft.NET.Sdk\">
//! <PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework></PropertyGroup>
//! <ItemGroup><PackageReference Include=\"x\" Version=\"1\"/></ItemGroup>
//! </Project>"#;
//! assert!(izanagi_kit::msbuild::detect(m));
//! ```

/// MSBuild project census.
#[derive(Debug, Clone)]
pub struct Msbuild {
    /// `<Project` root seen.
    pub has_root: bool,
    /// Element lines matching known MSBuild elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<PropertyGroup")
        || tr.starts_with("<ItemGroup")
        || tr.starts_with("<ItemDefinitionGroup")
        || tr.starts_with("<TargetFramework")
        || tr.starts_with("<TargetFrameworks")
        || tr.starts_with("<OutputType")
        || tr.starts_with("<PackageReference")
        || tr.starts_with("<ProjectReference")
        || tr.starts_with("<Reference ")
        || tr.starts_with("<Reference>")
        || tr.starts_with("<Compile ")
        || tr.starts_with("<Compile>")
        || tr.starts_with("<EmbeddedResource")
        || tr.starts_with("<Content ")
        || tr.starts_with("<Content>")
        || tr.starts_with("<None ")
        || tr.starts_with("<Import ")
        || tr.starts_with("<ImportGroup")
        || tr.starts_with("<Target ")
        || tr.starts_with("<Target>")
        || tr.starts_with("<UsingTask")
        || tr.starts_with("<Task ")
        || tr.starts_with("<DefineConstants")
        || tr.starts_with("<LangVersion")
        || tr.starts_with("<Nullable")
        || tr.starts_with("<ImplicitUsings")
        || tr.starts_with("<AssemblyName")
        || tr.starts_with("<RootNamespace")
        || tr.starts_with("<PackageId")
        || tr.starts_with("<Version")
        || tr.starts_with("<VersionPrefix")
        || tr.starts_with("<VersionSuffix")
        || tr.starts_with("<PackageVersion")
        || tr.starts_with("<GlobalPackageReference")
        || tr.starts_with("<PackageVersion")
        || tr.starts_with("<PackageDownload")
        || tr.starts_with("<FrameworkReference")
        || tr.starts_with("<PackageIcon")
        || tr.starts_with("<PackageReadmeFile")
        || tr.starts_with("<PackageLicenseExpression")
        || tr.starts_with("<NuGetAudit")
        || tr.starts_with("<RestorePackagesPath")
        || tr.starts_with("<RuntimeIdentifier")
        || tr.starts_with("<SelfContained")
        || tr.starts_with("<PublishSingleFile")
        || tr.starts_with("<AssemblyVersion")
        || tr.starts_with("<FileVersion")
        || tr.starts_with("<GeneratePackageOnBuild")
        || tr.starts_with("<TreatWarningsAsErrors")
        || tr.starts_with("<WarningsAsErrors")
        || tr.starts_with("<WarningLevel")
        || tr.starts_with("<DebugSymbols")
        || tr.starts_with("<DebugType")
        || tr.starts_with("<Optimize")
        || tr.starts_with("<PlatformTarget")
        || tr.starts_with("<Prefer32Bit")
        || tr.starts_with("<AllowUnsafeBlocks")
        || tr.starts_with("<SignAssembly")
        || tr.starts_with("<AssemblyOriginatorKeyFile")
        || tr.starts_with("<InternalsVisibleTo")
        || tr.starts_with("<ApplicationIcon")
        || tr.starts_with("<StartupObject")
        || tr.starts_with("<AnalysisLevel")
        || tr.starts_with("<EnforceCodeStyleInBuild")
        || tr.starts_with("<IsPackable")
        || tr.starts_with("<Description")
        || tr.starts_with("<Authors")
        || tr.starts_with("<Company")
        || tr.starts_with("<Product")
        || tr.starts_with("<Copyright")
        || tr.starts_with("<PackageProjectUrl")
        || tr.starts_with("<RepositoryUrl")
        || tr.starts_with("<RepositoryType")
        || tr.starts_with("<PackageTags")
        || tr.starts_with("<IncludeSymbols")
        || tr.starts_with("<SymbolPackageFormat")
        || tr.starts_with("<EmbeddedFiles")
        || tr.starts_with("<AdditionalFiles")
        || tr.starts_with("<Analyzer")
        || tr.starts_with("<CodeAnalysisRuleSet")
        || tr.starts_with("<CopyToOutputDirectory")
        || tr.starts_with("<CopyToPublishDirectory")
        || tr.starts_with("<Exclude")
        || tr.starts_with("<Link")
        || tr.starts_with("<PrivateAssets")
        || tr.starts_with("<Service")
        || tr.starts_with("<UseWPF")
        || tr.starts_with("<UseWindowsForms")
        || tr.starts_with("<UserSecretsId")
        || tr.starts_with("<DockerDefaultTargetOS")
        || tr.starts_with("<Configurations")
        || tr.starts_with("<Platforms")
}

/// Detect an MSBuild project file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<Project ") || tr.starts_with("<Project>") {
            root = true;
            continue;
        }
        if element(tr) {
            elems += 1;
        }
    }
    root && elems >= 2 || elems >= 4
}

impl Msbuild {
    /// Count elements. Returns `None` when the input does not look like
    /// an MSBuild project.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            has_root: false,
            elements: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("<Project ") || tr.starts_with("<Project>") {
                c.has_root = true;
                continue;
            }
            if element(tr) {
                c.elements += 1;
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
        let b = br#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
    <LangVersion>latest</LangVersion>
    <AssemblyName>App</AssemblyName>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="Serilog" Version="4.0.0"/>
    <ProjectReference Include="..\Lib\Lib.csproj"/>
    <Compile Include="Program.cs"/>
  </ItemGroup>
</Project>
"#;
        assert!(detect(b));
        let c = Msbuild::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 11);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"<html><body>x</body></html>"));
        assert!(!detect(b"<configuration></configuration>"));
        assert!(Msbuild::parse(b"").is_none());
    }
}
