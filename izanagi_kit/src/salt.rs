//! Salt state (SLS) YAML census.
//!
//! An SLS file declares state ids whose bodies hold `<module>.<func>:`
//! function keys (`pkg.installed`, `service.running`, `file.managed`) with
//! `- <arg>` lists, plus requisite keys (`require`/`watch`/`onchanges`/
//! `onfail`/`listen`/`prereq`/`onany`/`order`/`unless`/`onlyif`) and Jinja
//! templating (`{{ ... }}`/`{% ... %}`). `parse` counts each class.
//!
//! ```rust
//! let s = concat!(
//!     "nginx:\n",
//!     "  pkg.installed:\n",
//!     "    - name: nginx\n",
//!     "  service.running:\n",
//!     "    - enable: true\n",
//!     "    - watch:\n",
//!     "      - pkg: nginx\n",
//! );
//! let c = izanagi_kit::salt::Salt::parse(s.as_bytes()).unwrap();
//! assert_eq!(c.functions, 2);
//! assert_eq!(c.requisites, 1);
//! ```

const FAMILIES: &[&str] = &[
    "pkg",
    "file",
    "service",
    "cmd",
    "user",
    "group",
    "git",
    "cron",
    "host",
    "mount",
    "network",
    "test",
    "module",
    "pip",
    "npm",
    "gem",
    "archive",
    "mysql_",
    "postgres_",
    "docker_",
    "k8s",
    "salt",
    "ssh",
    "selinux",
    "firewalld",
    "iptables",
    "alternatives",
    "sysctl",
    "augeas",
    "timezone",
    "locale",
    "makeconf",
    "virtualenv",
    "composer",
    "supervisord",
    "tomcat",
    "win_",
    "boto_",
    "cloud",
    "grains",
    "pillar",
    "http",
    "event",
    "state",
    "saltutil",
    "schedule",
    "beacon",
    "keystone",
];
const ARGS: &[&str] = &[
    "name",
    "names",
    "source",
    "user",
    "group",
    "mode",
    "contents",
    "makedirs",
    "context",
    "defaults",
    "dir_mode",
    "file_mode",
    "target",
    "enable",
    "reload",
    "cwd",
    "env",
    "shell",
    "runas",
];
const REQS: &[&str] = &[
    "require",
    "watch",
    "onchanges",
    "onfail",
    "listen",
    "prereq",
    "onany",
    "require_in",
    "watch_in",
    "onchanges_in",
    "onfail_in",
    "listen_in",
    "order",
    "unless",
    "onlyif",
    "check_cmd",
];

/// Salt SLS census.
#[derive(Debug, Clone)]
pub struct Salt {
    /// Top-level state ids (column-0 `key:` lines).
    pub states: usize,
    /// `<module>.<func>:` function keys.
    pub functions: usize,
    /// `- <arg>` list items.
    pub items: usize,
    /// Requisite keys (`require`/`watch`/`onchanges`/…).
    pub requisites: usize,
    /// `{{ ... }}`/`{% ... %}` Jinja markers.
    pub templated: usize,
    /// `name:`/`names:`/`source:`/`user:`/`group:`/`mode:`/`contents:`/`makedirs:`/`context:`/`defaults:` args.
    pub args: usize,
}

fn is_func(key: &str) -> bool {
    let Some((fam, _)) = key.split_once('.') else {
        return false;
    };
    FAMILIES.iter().any(|f| fam.starts_with(f))
}

/// Whether the buffer looks like a Salt SLS file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut funcs = false;
    for l in t.lines() {
        let s = l.trim();
        if let Some(k) = s.strip_suffix(':') {
            if is_func(k.trim()) {
                funcs = true;
            }
        }
    }
    funcs
}

impl Salt {
    /// Parse an SLS file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            states: 0,
            functions: 0,
            items: 0,
            requisites: 0,
            templated: 0,
            args: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.contains("{{") || s.contains("{%") {
                c.templated += 1;
            }
            if !l.starts_with(' ') && !l.starts_with('\t') && s.ends_with(':') {
                c.states += 1;
                continue;
            }
            if let Some(k) = s.strip_suffix(':') {
                let k = k.trim();
                if is_func(k) {
                    c.functions += 1;
                    continue;
                }
                if REQS.contains(&k) {
                    c.requisites += 1;
                    continue;
                }
            }
            if let Some(inner) = s.strip_prefix("- ") {
                c.items += 1;
                if inner.ends_with(':') && REQS.contains(&inner.trim_end_matches(':')) {
                    c.requisites += 1;
                }
                if let Some(k) = inner.split(':').next() {
                    if ARGS.contains(&k.trim()) {
                        c.args += 1;
                    }
                }
                continue;
            }
            if let Some(k) = s.split(':').next() {
                if ARGS.contains(&k.trim()) {
                    c.args += 1;
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
    fn parses_sls() {
        let b = concat!(
            "nginx:\n",
            "  pkg.installed:\n",
            "    - name: nginx\n",
            "  service.running:\n",
            "    - enable: true\n",
            "    - require:\n",
            "      - pkg: nginx\n",
            "    - watch:\n",
            "      - file: conf\n",
            "/etc/nginx/nginx.conf:\n",
            "  file.managed:\n",
            "    - source: salt://nginx/nginx.conf\n",
            "    - template: jinja\n",
            "    - user: root\n",
        );
        let c = Salt::parse(b.as_bytes()).unwrap();
        assert_eq!(c.states, 2);
        assert_eq!(c.functions, 3);
        assert_eq!(c.requisites, 2);
        assert_eq!(c.items, 9);
        assert_eq!(c.args, 4);
    }

    #[test]
    fn rejects_other() {
        assert!(Salt::parse(b"foo: bar").is_none());
    }
}
