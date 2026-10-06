//! FreeSWITCH `*.xml` configuration census (`freeswitch.xml`,
//! `sofia.conf.xml`, `modules.conf.xml`, `dialplan/*.xml`,
//! `directory/*.xml` …).
//!
//! `<configuration name="…">` documents with `<settings>` +
//! `<param name="k" value="v"/>`, `X-PRE-PROCESS` macros, dialplan
//! `<extension>`/`<condition>`/`<action|anti-action application=…>`,
//! and `<modules>`/`<load module="…"/>` loader blocks.
//!
//! ```rust
//! let x = br#"<configuration name="sofia.conf">
//! <settings>
//! <param name="sip-ip" value="$$sip_ip"/>
//! <param name="rtp-ip" value="$$sip_ip"/>
//! <param name="sip-port" value="5060"/>
//! </settings>
//! </configuration>"#;
//! assert!(izanagi_kit::freeswitch::detect(x));
//! let c = izanagi_kit::freeswitch::Freeswitch::parse(x).unwrap();
//! assert_eq!(c.params, 3);
//! ```

/// FreeSWITCH XML config census.
#[derive(Debug, Clone)]
pub struct Freeswitch {
    /// `<param …>` elements.
    pub params: usize,
    /// `<configuration>`/`<section>`/`<modules>`/`<load>`/`<extension>`/
    /// `<condition>`/`<action|anti-action>`/`<gateway>`/`<domain>`/
    /// `<X-PRE-PROCESS>` marker lines.
    pub markers: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn marker(t: &str) -> bool {
    t.starts_with("<configuration")
        || t.starts_with("<section")
        || t.starts_with("<modules")
        || t.starts_with("<load ")
        || t.starts_with("<load>")
        || t.starts_with("<extension")
        || t.starts_with("<condition")
        || t.starts_with("<action ")
        || t.starts_with("<anti-action")
        || t.starts_with("<gateway")
        || t.starts_with("<gateways")
        || t.starts_with("<domain")
        || t.starts_with("<domains")
        || t.starts_with("<aliases")
        || t.starts_with("<groups")
        || t.starts_with("<users")
        || t.starts_with("<user ")
        || t.starts_with("<settings")
        || t.starts_with("<global_settings")
        || t.starts_with("<profile ")
        || t.starts_with("<profiles")
        || t.starts_with("<context")
        || t.starts_with("<X-PRE-PROCESS")
        || t.starts_with("<include")
        || t.starts_with("<network-lists")
        || t.starts_with("<param ")
        || t.starts_with("<variable ")
        || t.starts_with("<variables")
        || t.starts_with("<macros")
        || t.starts_with("<macro ")
        || t.starts_with("<input ")
        || t.starts_with("<menu ")
        || t.starts_with("<entry ")
        || t.starts_with("<caller-controls")
        || t.starts_with("<phrases")
        || t.starts_with("<lang ")
        || t.starts_with("<sound ")
        || t.starts_with("<recordings")
        || t.starts_with("<break ")
        || t.starts_with("<digit ")
        || t.starts_with("<nodestroy")
}

/// Detect a FreeSWITCH XML config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut params = 0usize;
    let mut marks = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with("<!--") {
            continue;
        }
        if tr.starts_with("<param ") || tr.starts_with("<param>") {
            params += 1;
            continue;
        }
        if marker(tr) {
            marks += 1;
        }
    }
    params >= 2 || (params >= 1 && marks >= 1) || marks >= 3
}

impl Freeswitch {
    /// Count params and structural markers. Returns `None` when the
    /// input does not look like a FreeSWITCH config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            params: 0,
            markers: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("<param ") || tr.starts_with("<param>") {
                c.params += 1;
                continue;
            }
            if marker(tr) {
                c.markers += 1;
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
        let b = br#"<!-- sofia -->
<configuration name="sofia.conf" description="sofia">
<global_settings>
<param name="log-level" value="0"/>
<param name="debug-presence" value="0"/>
</global_settings>
<profiles>
<profile name="internal">
<settings>
<param name="sip-ip" value="$$sip_ip"/>
<param name="sip-port" value="5060"/>
<param name="rtp-ip" value="$$sip_ip"/>
<param name="dialplan" value="XML"/>
<param name="context" value="public"/>
</settings>
</profile>
</profiles>
</configuration>"#;
        assert!(detect(b));
        let c = Freeswitch::parse(b).unwrap();
        assert_eq!(c.params, 7);
        assert!(c.markers >= 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"<html><body>x</body></html>"));
        assert!(!detect(b"key=value\n"));
        assert!(Freeswitch::parse(b"").is_none());
    }
}
