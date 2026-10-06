//! Tomcat `server.xml` census.
//!
//! Tomcat config is XML rooted at `<Server>`: `<Service>`,
//! `<Connector>` (port/protocol/redirectPort/…), `<Engine>`,
//! `<Host>` (appBase/unpackWARs/autoDeploy), `<Context>`,
//! `<Valve>`, `<Realm>`, `<Listener>`, `<GlobalNamingResources>`,
//! `<Cluster>`, `<Manager>`, `<Resources>`, `<Executor>`,
//! `<UpgradeProtocol>`, `<CredentialHandler>`.
//!
//! ```rust
//! let k = b"<Server port=\"8005\" shutdown=\"SHUTDOWN\">\n<Service name=\"Catalina\">\n<Connector port=\"8080\"/>\n<Engine name=\"Catalina\" defaultHost=\"localhost\">\n<Host name=\"localhost\" appBase=\"webapps\"/>\n</Engine>\n</Service>\n</Server>\n";
//! assert!(izanagi_kit::tomcat::detect(k));
//! ```

/// Tomcat `server.xml` census.
#[derive(Debug, Clone)]
pub struct Tomcat {
    /// element tags (`<X …>`/`<X/>`).
    pub elements: usize,
    /// `key="value"` attributes.
    pub attributes: usize,
    /// recognised tomcat elements present.
    pub keys: usize,
    /// `<!--`/`-->` comment lines.
    pub comments: usize,
}

const ELEMENTS: &[&str] = &[
    "<Server",
    "<Service",
    "<Connector",
    "<Engine",
    "<Host",
    "<Context",
    "<Valve",
    "<Realm",
    "<Listener",
    "<GlobalNamingResources",
    "<Cluster",
    "<Manager",
    "<Resources",
    "<Executor",
    "<UpgradeProtocol",
    "<CredentialHandler",
    "<NamingResources",
    "<WatchedResource",
    "<Environment",
    "<Resource",
    "<JarScanner",
    "<SessionIdGenerator",
    "<CookieProcessor",
    "<Store",
    "<Channel",
    "<Membership",
    "<Interceptor",
    "<Deployer",
];

fn code_line(line: &str) -> &str {
    let s = line.trim();
    if let Some(i) = s.find("<!--") {
        return s[..i].trim();
    }
    s
}

/// Detect a Tomcat `server.xml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `<Server>` is the Tomcat-only root; `<Service>/<Connector>/
    // <Engine>/<Host>` children corroborate.
    let mut server = 0usize;
    let mut elements = 0usize;
    for line in t.lines() {
        let s = code_line(line);
        if s.is_empty() {
            continue;
        }
        if s.contains("<Server") {
            server += 1;
        }
        for e in ELEMENTS {
            if s.contains(e) {
                elements += 1;
                break;
            }
        }
    }
    server >= 1 && elements >= 3
}

impl Tomcat {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            elements: 0,
            attributes: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            let code = code_line(line);
            if code.is_empty() {
                continue;
            }
            for e in ELEMENTS {
                if code.contains(e) {
                    c.elements += 1;
                    c.keys += 1;
                    break;
                }
            }
            c.attributes += code.matches("=\"").count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"<Server port=\"8005\" shutdown=\"SHUTDOWN\">\n<Service name=\"Catalina\">\n<Connector port=\"8080\"/>\n<Engine name=\"Catalina\" defaultHost=\"localhost\">\n<Host name=\"localhost\" appBase=\"webapps\"/>\n</Engine>\n</Service>\n</Server>\n";
        assert!(detect(b));
        let c = Tomcat::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"<web-app><servlet/></web-app>\n"));
        assert!(!detect(
            b"<!-- <Server><Service/><Connector/><Engine/></Server> -->\n"
        ));
    }
}
