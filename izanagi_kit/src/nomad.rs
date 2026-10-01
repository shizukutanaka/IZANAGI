//! Nomad job specification (HCL) census.
//!
//! A Nomad job file is HCL: `job "name" {` containing `datacenters`,
//! `type`, `group "g" {` with `count`, `task "t" {` with `driver`/`config`,
//! `service {`, `port "p" {`, `resources { cpu = N memory = N }`,
//! `constraint {`, `network {` and `env {` blocks. `parse` counts jobs,
//! groups, tasks, blocks and `key = value` pairs.
//!
//! ```rust
//! let n = concat!(
//!     "job \"api\" {\n",
//!     "  datacenters = [\"dc1\"]\n",
//!     "  type = \"service\"\n",
//!     "  group \"web\" {\n",
//!     "    count = 2\n",
//!     "    task \"http\" {\n",
//!     "      driver = \"docker\"\n",
//!     "      service {\n",
//!     "        port \"http\" {}\n",
//!     "      }\n",
//!     "      resources {\n",
//!     "        cpu = 500\n",
//!     "        memory = 256\n",
//!     "      }\n",
//!     "    }\n",
//!     "  }\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::nomad::Nomad::parse(n.as_bytes()).unwrap();
//! assert_eq!(c.jobs, 1);
//! assert_eq!(c.groups, 1);
//! assert_eq!(c.tasks, 1);
//! ```

/// Nomad job census.
#[derive(Debug, Clone)]
pub struct Nomad {
    /// `job "name" {` declarations.
    pub jobs: usize,
    /// `group "g" {` declarations.
    pub groups: usize,
    /// `task "t" {` declarations.
    pub tasks: usize,
    /// `service {`/`service "x" {` blocks.
    pub services: usize,
    /// `port "p"`/`port {` declarations.
    pub ports: usize,
    /// `constraint {`/`affinity {`/`spread {` blocks.
    pub constraints: usize,
    /// `resources {`/`device {`/`network {`/`volume {`/`env {`/`config {`/`artifact {`/`template {`/`vault {`/`logs {`/`update {`/`migrate {`/`reschedule {` blocks.
    pub blocks: usize,
    /// `key = value` pairs.
    pub kv_pairs: usize,
    /// `cpu`/`memory`/`memory_max`/`cores`/`disk` keys.
    pub resource_keys: usize,
    /// `driver`/`image`/`command`/`args`/`datacenters`/`type`/`region`/`namespace` keys.
    pub drivers: usize,
}

/// Whether the buffer looks like a Nomad job file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("job \"") || t.contains("job {"))
        && (t.contains("datacenters") || t.contains("task \"") || t.contains("group \""))
}

impl Nomad {
    /// Parse a Nomad job file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            jobs: 0,
            groups: 0,
            tasks: 0,
            services: 0,
            ports: 0,
            constraints: 0,
            blocks: 0,
            kv_pairs: 0,
            resource_keys: 0,
            drivers: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with("//") {
                continue;
            }
            if s.contains('=') && !s.contains("==") {
                c.kv_pairs += 1;
            }
            if s.starts_with("job ") {
                c.jobs += 1;
            } else if s.starts_with("group ") {
                c.groups += 1;
            } else if s.starts_with("task ") {
                c.tasks += 1;
            } else if s.starts_with("service ") || s == "service{" {
                c.services += 1;
            } else if s.starts_with("port ") || s.starts_with("port{") {
                c.ports += 1;
            } else if s.starts_with("constraint")
                || s.starts_with("affinity")
                || s.starts_with("spread")
            {
                c.constraints += 1;
            } else if [
                "resources",
                "device",
                "network",
                "volume",
                "env",
                "config",
                "artifact",
                "template",
                "vault",
                "logs",
                "update",
                "migrate",
                "reschedule",
                "periodic",
                "parameterized",
                "dispatch_payload",
                "meta",
                "scaling",
                "kill_timeout",
                "shutdown_delay",
            ]
            .iter()
            .any(|k| s.starts_with(k))
            {
                c.blocks += 1;
            }
            for k in [
                "cpu",
                "memory",
                "memory_max",
                "cores",
                "disk",
                "memory_oversubscription",
            ] {
                if s.starts_with(k) && s[k.len()..].trim_start().starts_with('=') {
                    c.resource_keys += 1;
                    break;
                }
            }
            for k in [
                "driver",
                "image",
                "command",
                "args",
                "datacenters",
                "type",
                "region",
                "namespace",
            ] {
                if s.starts_with(k) && s[k.len()..].trim_start().starts_with('=') {
                    c.drivers += 1;
                    break;
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
    fn parses_job() {
        let b = concat!(
            "job \"api\" {\n",
            "  datacenters = [\"dc1\"]\n",
            "  type = \"service\"\n",
            "  group \"web\" {\n",
            "    count = 2\n",
            "    network {\n",
            "      port \"http\" {}\n",
            "    }\n",
            "    task \"http\" {\n",
            "      driver = \"docker\"\n",
            "      config {\n",
            "        image = \"x\"\n",
            "      }\n",
            "      service {\n",
            "        name = \"api\"\n",
            "      }\n",
            "      resources {\n",
            "        cpu = 500\n",
            "        memory = 256\n",
            "      }\n",
            "      constraint {\n",
            "        attribute = \"${attr}\"\n",
            "      }\n",
            "    }\n",
            "  }\n",
            "}\n",
        );
        let c = Nomad::parse(b.as_bytes()).unwrap();
        assert_eq!(c.jobs, 1);
        assert_eq!(c.groups, 1);
        assert_eq!(c.tasks, 1);
        assert_eq!(c.services, 1);
        assert_eq!(c.ports, 1);
        assert_eq!(c.constraints, 1);
        assert_eq!(c.blocks, 3);
        assert_eq!(c.resource_keys, 2);
        assert_eq!(c.drivers, 4);
    }

    #[test]
    fn rejects_other() {
        assert!(Nomad::parse(b"foo = 1").is_none());
    }
}
