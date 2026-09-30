//! Ansible playbook YAML census.
//!
//! A playbook is a list of plays (`- hosts:` with optional `name:`/`vars:`/
//! `become`) containing `tasks:`/`handlers:` lists of `- name:` items plus
//! `roles:`/`include_tasks`/`import_playbook`. Task items invoke modules
//! (`shell:`/`copy:`/`yum:`/`service:`/`template:`/…). `parse` counts plays,
//! tasks, handlers, roles, module calls and variables.
//!
//! ```rust
//! let a = concat!(
//!     "- hosts: web\n",
//!     "  become: true\n",
//!     "  vars:\n",
//!     "    port: 80\n",
//!     "  tasks:\n",
//!     "    - name: install\n",
//!     "      yum: name=nginx state=present\n",
//!     "    - name: copy\n",
//!     "      copy: src=a dest=b\n",
//!     "      notify: restart\n",
//!     "  handlers:\n",
//!     "    - name: restart\n",
//!     "      service: name=nginx state=restarted\n",
//! );
//! let c = izanagi_kit::ansible::Ansible::parse(a.as_bytes()).unwrap();
//! assert_eq!(c.plays, 1);
//! assert_eq!(c.tasks, 2);
//! assert_eq!(c.handlers, 1);
//! ```

const MODULES: &[&str] = &[
    "shell",
    "command",
    "copy",
    "yum",
    "apt",
    "dnf",
    "pip",
    "service",
    "systemd",
    "template",
    "git",
    "file",
    "debug",
    "lineinfile",
    "blockinfile",
    "user",
    "group",
    "cron",
    "mount",
    "fetch",
    "unarchive",
    "get_url",
    "uri",
    "wait_for",
    "pause",
    "set_fact",
    "fail",
    "assert",
    "meta",
    "stat",
    "find",
    "register",
    "delegate_to",
    "become",
    "when",
    "loop",
    "with_items",
    "ignore_errors",
    "changed_when",
    "failed_when",
    "tags",
    "environment",
    "notify",
    "name",
    "include_vars",
];

/// Ansible playbook census.
#[derive(Debug, Clone)]
pub struct Ansible {
    /// Play headers (`- hosts:` or `hosts:` lines).
    pub plays: usize,
    /// `tasks:`/`- ` task items (name-keyed entries under tasks).
    pub tasks: usize,
    /// Items under `handlers:`.
    pub handlers: usize,
    /// Entries under `roles:` plus `role:` uses.
    pub roles: usize,
    /// `vars:` keys and `set_fact`/`vars_files` keys.
    pub vars: usize,
    /// Lines matching a known module/task keyword (`mod:` at item depth).
    pub module_calls: usize,
    /// `become`/`remote_user` keys.
    pub becomes: usize,
    /// `include_tasks`/`import_tasks`/`import_playbook`/`include` keys.
    pub includes: usize,
    /// `- name:` entries total.
    pub names: usize,
}

/// Whether the buffer looks like an Ansible playbook.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("hosts:") || t.contains("- name:"))
        && (t.contains("tasks:") || t.contains("roles:") || t.contains("gather_facts:"))
}

impl Ansible {
    /// Parse an Ansible playbook into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            plays: 0,
            tasks: 0,
            handlers: 0,
            roles: 0,
            vars: 0,
            module_calls: 0,
            becomes: 0,
            includes: 0,
            names: 0,
        };
        let mut scope = "";
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s.starts_with("tasks:") {
                scope = "tasks";
                continue;
            }
            if s.starts_with("handlers:") {
                scope = "handlers";
                continue;
            }
            if s.starts_with("roles:") {
                scope = "roles";
                continue;
            }
            if s.starts_with("vars:") {
                scope = "vars";
                continue;
            }
            if s.starts_with("hosts:") || s == "- hosts:" {
                c.plays += 1;
                scope = "";
                continue;
            }
            if s.starts_with("- hosts:") {
                c.plays += 1;
                scope = "";
                continue;
            }
            if s.starts_with("- name:") {
                c.names += 1;
                match scope {
                    "tasks" => c.tasks += 1,
                    "handlers" => c.handlers += 1,
                    "roles" => c.roles += 1,
                    _ => {}
                }
                continue;
            }
            if scope == "roles" && s.starts_with("- ") {
                c.roles += 1;
                continue;
            }
            if scope == "vars" && !s.starts_with("- ") && s.ends_with(':') {
                c.vars += 1;
                continue;
            }
            if s.starts_with("become") || s.starts_with("remote_user:") {
                c.becomes += 1;
                continue;
            }
            if s.starts_with("include_tasks")
                || s.starts_with("import_tasks")
                || s.starts_with("import_playbook")
                || s.starts_with("include_vars")
                || s.starts_with("include:")
            {
                c.includes += 1;
                continue;
            }
            let head = s
                .trim_start_matches("- ")
                .split(':')
                .next()
                .unwrap_or("")
                .trim();
            if MODULES.contains(&head) {
                c.module_calls += 1;
                continue;
            }
            if scope == "vars" && !s.starts_with("- ") {
                c.vars += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_playbook() {
        let b = concat!(
            "- name: play\n",
            "  hosts: web\n",
            "  become: true\n",
            "  vars:\n",
            "    http_port: 80\n",
            "  tasks:\n",
            "    - name: install\n",
            "      yum: name=nginx state=present\n",
            "    - name: copy\n",
            "      copy: src=a dest=b\n",
            "    - name: run\n",
            "      shell: echo hi\n",
            "  handlers:\n",
            "    - name: restart\n",
            "      service: name=nginx state=restarted\n",
            "- hosts: db\n",
            "  roles:\n",
            "    - common\n",
            "    - db\n",
            "    - { role: extra }\n",
        );
        let c = Ansible::parse(b.as_bytes()).unwrap();
        assert_eq!(c.plays, 2);
        assert_eq!(c.tasks, 3);
        assert_eq!(c.handlers, 1);
        assert_eq!(c.roles, 3);
        assert_eq!(c.vars, 1);
        assert_eq!(c.becomes, 1);
        assert_eq!(c.module_calls, 4);
        assert_eq!(c.names, 5);
    }

    #[test]
    fn rejects_other() {
        assert!(Ansible::parse(b"foo: bar").is_none());
    }
}
