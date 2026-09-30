//! Census of an OpenWrt procd init script (`/etc/init.d/*`).
//!
//! `#!/bin/sh /etc/rc.common` shebang, `START=`/`STOP=`/`USE_PROCD=1`
//! ordering/enable variables, `start_service()`/`stop_service()`/
//! `reload_service()`/`service_triggers()`/`boot()` functions and
//! `procd_open_instance`/`procd_set_param`/`procd_append_param`/
//! `procd_close_instance`/`procd_add_reload_trigger` calls.
//! Counts procd calls by kind plus functions and variables.
//!
//! ```rust
//! let c = izanagi_kit::procd::Procd::parse(
//!     b"#!/bin/sh /etc/rc.common\nSTART=99\nUSE_PROCD=1\nstart_service() {\n\
//!       procd_open_instance\nprocd_set_param command /bin/d\nprocd_close_instance\n}\n",
//! ).unwrap();
//! assert_eq!(c.procd_calls, 3);
//! ```
#![forbid(unsafe_code)]

/// procd init script census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Procd {
    /// `procd_*` API calls.
    pub procd_calls: usize,
    /// `*_service()`/`boot()`/`shutdown()` function defs.
    pub functions: usize,
    /// `START`/`STOP`/`USE_PROCD`/`RELOAD_SERVICE`/`EXTRA_COMMANDS`/`EXTRA_HELP` variables.
    pub variables: usize,
    /// `#` comment lines (shebang excluded).
    pub comments: usize,
}

/// procd API prefixes.
const PROCD_FUNCS: &[&str] = &[
    "procd_open_instance",
    "procd_close_instance",
    "procd_set_param",
    "procd_append_param",
    "procd_add_reload_trigger",
    "procd_add_reload_mount_trigger",
    "procd_add_interface_trigger",
    "procd_open_validate",
    "procd_close_validate",
    "procd_add_validation",
    "procd_open_data",
    "procd_close_data",
    "procd_jshdl_add",
];

/// Service-lifecycle function names.
const SERVICE_FUNCS: &[&str] = &[
    "start_service",
    "stop_service",
    "reload_service",
    "restart",
    "boot",
    "shutdown",
    "status",
    "service_triggers",
    "service_started",
    "service_stopped",
    "service_check",
];

/// Init variables.
const VARS: &[&str] = &[
    "START",
    "STOP",
    "USE_PROCD",
    "RELOAD_SERVICE",
    "EXTRA_COMMANDS",
    "EXTRA_HELP",
];

/// True if `b` looks like a procd init script.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("/etc/rc.common")
        || PROCD_FUNCS.iter().any(|f| t.contains(f))
        || (t.contains("USE_PROCD") && t.contains("start_service"))
}

impl Procd {
    /// Parse a procd init script into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            procd_calls: 0,
            functions: 0,
            variables: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') && !l.starts_with("#!") {
                c.comments += 1;
                continue;
            }
            let head = l.split_whitespace().next().unwrap_or("");
            if PROCD_FUNCS.iter().any(|f| l.contains(f) || head == *f) {
                c.procd_calls += 1;
            } else if l.ends_with('{') && SERVICE_FUNCS.iter().any(|f| l.starts_with(f)) {
                c.functions += 1;
            } else if l.contains('=') {
                let k = l.split('=').next().unwrap_or("").trim();
                if VARS.contains(&k) {
                    c.variables += 1;
                }
            }
        }
        if c.procd_calls == 0 && c.variables == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "#!/bin/sh /etc/rc.common\n",
            "START=99\n",
            "STOP=10\n",
            "USE_PROCD=1\n",
            "start_service() {\n",
            "  procd_open_instance\n",
            "  procd_set_param command /bin/d\n",
            "  procd_close_instance\n",
            "}\n",
            "service_triggers() {\n",
            "  procd_add_reload_trigger \"network\"\n",
            "}\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Procd::parse(b.as_bytes()).unwrap();
        assert_eq!(c.procd_calls, 4);
        assert_eq!(c.functions, 2);
        assert_eq!(c.variables, 3);
        assert_eq!(c.comments, 0);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"#!/bin/sh\necho hi\n"));
        assert!(Procd::parse(b"echo hi\n").is_none());
    }
}
