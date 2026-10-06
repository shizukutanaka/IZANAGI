//! ASP.NET Core `appsettings.json` census.
//!
//! `"key":` JSON with ASP.NET conventions: `Logging`/`LogLevel`,
//! `ConnectionStrings`, `Kestrel`, `AllowedHosts`, `Urls`,
//! `Serilog`/`WriteTo`/`MinimumLevel`, `Jwt`/`Issuer`/`Audience`,
//! `ReverseProxy`/`Routes`/`Clusters`, `Cors`, `RateLimiting`,
//! `DetailedErrors`, `Authentication`, `Https`/`Endpoints`/
//! `Certificates` …
//!
//! ```rust
//! let a = br#"{
//!   "Logging": {"LogLevel": {"Default": "Information"}},
//!   "AllowedHosts": "*",
//!   "ConnectionStrings": {"Default": "Server=db"},
//!   "Kestrel": {"Endpoints": {"Http": {"Url": "http://localhost:5000"}}}
//! }"#;
//! assert!(izanagi_kit::appsettings::detect(a));
//! ```

/// appsettings.json census.
#[derive(Debug, Clone)]
pub struct Appsettings {
    /// `"key":` lines matching known ASP.NET config keys.
    pub keys: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

/// ASP.NET Core config keys (top/second level).
const KEYS: &[&str] = &[
    "AllowedHosts",
    "AllowedScopes",
    "ApiKeys",
    "ApplicationInsights",
    "Authentication",
    "Certificates",
    "Clusters",
    "ConnectionStrings",
    "Console",
    "Cors",
    "Debug",
    "Default",
    "DetailedErrors",
    "Endpoints",
    "EnvironmentName",
    "EventSource",
    "Hangfire",
    "Hosts",
    "Https",
    "IdentityServer",
    "Issuer",
    "Jwt",
    "Kestrel",
    "Level",
    "Limits",
    "Listener",
    "LogLevel",
    "Logging",
    "MinimumLevel",
    "Name",
    "PathBase",
    "PathString",
    "Policies",
    "Protocols",
    "RateLimiting",
    "ResponseCaching",
    "ReverseProxy",
    "Routes",
    "Serilog",
    "Smtp",
    "Sni",
    "Urls",
    "UseDeveloperExceptionPage",
    "WriteTo",
];

/// JSON keys that look alike but are not appsettings (npm/dotnet
/// project files).
const REJECT: &[&str] = &[
    "compilerOptions",
    "dependencies",
    "devDependencies",
    "mssql",
    "outDir",
    "references",
    "sdk",
    "targets",
];

fn jkey(l: &str) -> Option<&str> {
    let t = l.trim_start();
    let t = t.strip_prefix('"')?;
    let end = t.find('"')?;
    let key = &t[..end];
    if key.is_empty() {
        return None;
    }
    let rest = t[end + 1..].trim_start();
    if rest.starts_with(':') {
        Some(key)
    } else {
        None
    }
}

/// Detect an `appsettings.json` config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut n = 0usize;
    let mut strong = 0usize;
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if REJECT.contains(&k) {
                return false;
            }
            if KEYS.contains(&k) {
                n += 1;
                if matches!(
                    k,
                    "AllowedHosts"
                        | "Kestrel"
                        | "Serilog"
                        | "ReverseProxy"
                        | "ConnectionStrings"
                        | "Jwt"
                        | "IdentityServer"
                        | "RateLimiting"
                        | "ApplicationInsights"
                        | "Hangfire"
                ) {
                    strong += 1;
                }
            }
        }
    }
    strong >= 1 && n >= 2 || n >= 4
}

impl Appsettings {
    /// Count keys. Returns `None` when the input does not look like an
    /// `appsettings.json`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if let Some(k) = jkey(l) {
                if KEYS.contains(&k) {
                    c.keys += 1;
                }
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
        let b = br#"{
    "Logging": {
        "LogLevel": {
            "Default": "Information",
            "Microsoft": "Warning",
            "Microsoft.Hosting.Lifetime": "Information"
        }
    },
    "AllowedHosts": "*",
    "ConnectionStrings": {
        "DefaultConnection": "Server=db;Database=app"
    },
    "Kestrel": {
        "Endpoints": {
            "Http": { "Url": "http://localhost:5000" }
        }
    },
    "Serilog": {
        "MinimumLevel": "Debug",
        "WriteTo": [ { "Name": "Console" } ]
    }
}
"#;
        assert!(detect(b));
        let c = Appsettings::parse(b).unwrap();
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_package_json() {
        assert!(!detect(
            br#"{"name":"x","dependencies":{"a":"1"},"devDependencies":{}}"#
        ));
        assert!(!detect(b"key=value\n"));
        assert!(Appsettings::parse(b"").is_none());
    }
}
