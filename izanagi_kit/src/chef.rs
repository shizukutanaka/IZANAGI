//! Chef recipe (Ruby DSL) census.
//!
//! Chef recipes declare resources (`package 'nginx'`, `service 'x' do`,
//! `template '/path' do`, `file`, `directory`, `execute`, `cookbook_file`,
//! `user`, `git`, `include_recipe 'a::b'`) with `action`, `notifies`,
//! `only_if`/`not_if` guards and `node['attr']` references. `parse` counts
//! resources, blocks, guards and attribute references.
//!
//! ```rust
//! let c = concat!(
//!     "package 'nginx' do\n",
//!     "  action :install\n",
//!     "end\n",
//!     "template '/etc/nginx.conf' do\n",
//!     "  source 'nginx.conf.erb'\n",
//!     "  notifies :restart, 'service[nginx]', :delayed\n",
//!     "end\n",
//!     "include_recipe 'base::common'\n",
//!     "service 'nginx' do\n",
//!     "  action [:enable, :start]\n",
//!     "  only_if { node['os'] == 'linux' }\n",
//!     "end\n",
//! );
//! let c2 = izanagi_kit::chef::Chef::parse(c.as_bytes()).unwrap();
//! assert_eq!(c2.resources, 3);
//! assert_eq!(c2.recipes, 1);
//! ```

const RESOURCES: &[&str] = &[
    "package",
    "service",
    "template",
    "file",
    "directory",
    "execute",
    "cookbook_file",
    "remote_file",
    "user",
    "group",
    "git",
    "bash",
    "cron",
    "mount",
    "route",
    "link",
    "ifconfig",
    "script",
    "ruby_block",
    "log",
    "apt_repository",
    "yum_repository",
    "zypper_repository",
    "windows_package",
    "registry_key",
    "dsc_resource",
    "http_request",
    "launchd",
    "systemd_unit",
    "osx_profile",
    "dmg_package",
];

/// Chef recipe census.
#[derive(Debug, Clone)]
pub struct Chef {
    /// Resource declarations (`<name> '<title>'` at line start).
    pub resources: usize,
    /// ` do`/` do |` block opens.
    pub blocks: usize,
    /// `action` properties.
    pub actions: usize,
    /// `notifies`/`subscribes` lines.
    pub notifies: usize,
    /// `only_if`/`not_if` guards.
    pub guards: usize,
    /// `node[` attribute references.
    pub attributes: usize,
    /// `include_recipe` lines.
    pub recipes: usize,
    /// `lazy`/`sensitive`/`ignore_failure`/`retries` properties.
    pub extras: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a Chef recipe.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut res = false;
    for l in t.lines() {
        let s = l.trim();
        for r in RESOURCES {
            if s.starts_with(r) && (s[r.len()..].starts_with(' ') || s[r.len()..].starts_with('\''))
            {
                res = true;
            }
        }
    }
    res && (t.contains(" do") || t.contains("include_recipe") || t.contains("action"))
}

impl Chef {
    /// Parse a Chef recipe into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            resources: 0,
            blocks: 0,
            actions: 0,
            notifies: 0,
            guards: 0,
            attributes: 0,
            recipes: 0,
            extras: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s.starts_with("include_recipe") {
                c.recipes += 1;
                continue;
            }
            let mut matched = false;
            for r in RESOURCES {
                if let Some(tail) = s.strip_prefix(r) {
                    if tail.starts_with(' ') || tail.starts_with('\'') || tail.starts_with('"') {
                        c.resources += 1;
                        matched = true;
                        break;
                    }
                }
            }
            if matched {
                if s.ends_with(" do") || s.contains(" do |") {
                    c.blocks += 1;
                }
                continue;
            }
            if s.ends_with(" do") || s.contains(" do |") {
                c.blocks += 1;
            }
            c.attributes += s.matches("node[").count();
            if s.starts_with("action ") || s.starts_with("action[") {
                c.actions += 1;
            } else if s.starts_with("notifies ") || s.starts_with("subscribes ") {
                c.notifies += 1;
            } else if s.starts_with("only_if") || s.starts_with("not_if") {
                c.guards += 1;
            } else if s.starts_with("lazy ")
                || s.starts_with("sensitive ")
                || s.starts_with("ignore_failure")
                || s.starts_with("retries ")
                || s.starts_with("retry_delay")
            {
                c.extras += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recipe() {
        let b = concat!(
            "package 'nginx' do\n",
            "  action :install\n",
            "end\n",
            "service 'nginx' do\n",
            "  action [:enable, :start]\n",
            "end\n",
            "template '/etc/nginx/nginx.conf' do\n",
            "  source 'nginx.conf.erb'\n",
            "  mode '0644'\n",
            "  notifies :restart, 'service[nginx]', :delayed\n",
            "  only_if { node['platform_family'] == 'debian' }\n",
            "end\n",
            "include_recipe 'base::common'\n",
            "execute 'cmd' do\n",
            "  command 'true'\n",
            "  retries 2\n",
            "end\n",
        );
        let c = Chef::parse(b.as_bytes()).unwrap();
        assert_eq!(c.resources, 4);
        assert_eq!(c.blocks, 4);
        assert_eq!(c.actions, 2);
        assert_eq!(c.notifies, 1);
        assert_eq!(c.guards, 1);
        assert_eq!(c.attributes, 1);
        assert_eq!(c.recipes, 1);
        assert_eq!(c.extras, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Chef::parse(b"puts 'hi'").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
