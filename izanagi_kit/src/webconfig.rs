//! IIS/ASP.NET `web.config` XML census.
//!
//! `<configuration>` root + `<system.web>`/`<system.webServer>`/
//! `<system.web.extensions>`/`<system.applicationHost>`/
//! `<appSettings>`/`<connectionStrings>`/`<add key=`/`<compilation`/
//! `<httpRuntime`/`<customErrors`/`<authentication`/`<authorization`/
//! `<machineKey`/`<membership`/`<roleManager`/`<profile`/
//! `<handlers`/`<modules`/`<rewrite`/`<rules`/`<security`/
//! `<staticContent`/`<urlCompression`/`<httpErrors`/`<tracing`/
//! `<location`/`<runtime`/`<assemblyBinding`/`<dependentAssembly`/
//! `<bindingRedirect` elements.
//!
//! ```rust
//! let w = br#"<configuration>
//! <appSettings><add key="k" value="v"/></appSettings>
//! <system.web><compilation debug="true"/><httpRuntime targetFramework="4.8"/></system.web>
//! <system.webServer><modules><remove name="x"/></modules></system.webServer>
//! </configuration>"#;
//! assert!(izanagi_kit::webconfig::detect(w));
//! ```

/// web.config census.
#[derive(Debug, Clone)]
pub struct Webconfig {
    /// Matching element lines.
    pub elements: usize,
    /// `<add key=`/`add name=` lines.
    pub adds: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<system.web")
        || tr.starts_with("<system.webServer")
        || tr.starts_with("<system.web.extensions")
        || tr.starts_with("<system.applicationHost")
        || tr.starts_with("<system.diagnostics")
        || tr.starts_with("<appSettings")
        || tr.starts_with("<connectionStrings")
        || tr.starts_with("<configSections")
        || tr.starts_with("<sectionGroup")
        || tr.starts_with("<section ")
        || tr.starts_with("<compilation")
        || tr.starts_with("<httpRuntime")
        || tr.starts_with("<pages")
        || tr.starts_with("<customErrors")
        || tr.starts_with("<authentication")
        || tr.starts_with("<authorization")
        || tr.starts_with("<membership")
        || tr.starts_with("<roleManager")
        || tr.starts_with("<profile")
        || tr.starts_with("<machineKey")
        || tr.starts_with("<sessionState")
        || tr.starts_with("<globalization")
        || tr.starts_with("<httpHandlers")
        || tr.starts_with("<httpModules")
        || tr.starts_with("<handlers")
        || tr.starts_with("<modules")
        || tr.starts_with("<rewrite")
        || tr.starts_with("<rules")
        || tr.starts_with("<rule ")
        || tr.starts_with("<security")
        || tr.starts_with("<staticContent")
        || tr.starts_with("<urlCompression")
        || tr.starts_with("<httpErrors")
        || tr.starts_with("<tracing")
        || tr.starts_with("<location")
        || tr.starts_with("<runtime")
        || tr.starts_with("<assemblyBinding")
        || tr.starts_with("<dependentAssembly")
        || tr.starts_with("<bindingRedirect")
        || tr.starts_with("<probing")
        || tr.starts_with("<anonymousIdentification")
        || tr.starts_with("<clientTarget")
        || tr.starts_with("<siteMap")
        || tr.starts_with("<webParts")
        || tr.starts_with("<healthMonitoring")
        || tr.starts_with("<processModel")
        || tr.starts_with("<trust")
        || tr.starts_with("<deployment")
        || tr.starts_with("<uri")
        || tr.starts_with("<caching")
        || tr.starts_with("<outputCache")
        || tr.starts_with("<protocols")
        || tr.starts_with("<add key")
        || tr.starts_with("<add name")
        || tr.starts_with("<remove")
        || tr.starts_with("<clear")
}

/// Detect a `web.config` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut cfg = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<configuration") {
            cfg = true;
            continue;
        }
        if element(tr) {
            elems += 1;
        }
    }
    elems >= 3 || (cfg && elems >= 2)
}

impl Webconfig {
    /// Count elements. Returns `None` when the input does not look like a
    /// `web.config`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            elements: 0,
            adds: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("<add key") || tr.starts_with("<add name") {
                c.adds += 1;
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
        let b = br#"<?xml version="1.0"?>
<configuration>
    <appSettings>
        <add key="Setting1" value="x"/>
        <add key="Setting2" value="y"/>
    </appSettings>
    <connectionStrings>
        <add name="Default" connectionString="Server=db"/>
    </connectionStrings>
    <system.web>
        <compilation debug="true" targetFramework="4.8"/>
        <httpRuntime targetFramework="4.8"/>
        <customErrors mode="RemoteOnly"/>
        <authentication mode="Forms"/>
        <membership/>
        <roleManager enabled="true"/>
    </system.web>
    <system.webServer>
        <modules runAllManagedModulesForAllRequests="true"/>
        <handlers><add name="svc" path="*.svc" type="x"/></handlers>
        <rewrite><rules><rule name="a"/></rules></rewrite>
        <security><requestFiltering/></security>
        <staticContent><remove fileExtension=".woff"/></staticContent>
    </system.webServer>
    <runtime>
        <assemblyBinding xmlns="urn:schemas-microsoft-com:asm.v1">
            <dependentAssembly><bindingRedirect oldVersion="0.0.0.0-1" newVersion="1"/></dependentAssembly>
        </assemblyBinding>
    </runtime>
</configuration>
"#;
        assert!(detect(b));
        let c = Webconfig::parse(b).unwrap();
        assert!(c.elements >= 18);
        assert!(c.adds >= 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"<html><body>x</body></html>"));
        assert!(!detect(b"key=value\n"));
        assert!(Webconfig::parse(b"").is_none());
    }
}
