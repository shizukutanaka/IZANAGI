//! Google App Engine `app.yaml` / `appengine-web.xml` census.
//!
//! `app.yaml` keys: `runtime` (`nodejs`/`python`/`java`/`go`/`php`/
//! `ruby`/`custom`/`vm`/`flex`), `service`/`module`, `instance_class`
//! (`F1`/`B1`), `runtime_config`, `env_variables`, `handlers`
//! (`url`/`static_files`/`static_dir`/`script`/`secure`/`redirect_http_response_code`/
//! `expiration`/`http_headers`/`mime_type`/`login`/`auth_fail_action`/`application_readable`),
//! `inbound_services`, `automatic_scaling`/`basic_scaling`/`manual_scaling`
//! (`min_instances`/`max_instances`/`target_cpu_utilization`/`cool_down_period_sec`/
//! `max_idle_instances`/`min_idle_instances`/`target_throughput_utilization`),
//! `network` (`name`/`forwarded_ports`/`instance_tag`/`subnetwork_name`),
//! `vpc_access_connector`, `entrypoint`, `service_account`,
//! `build_env_variables`, `default_expiration`, `error_handlers`,
//! `includes`, `beta_settings`, `liveness_check`/`readiness_check`,
//! `resources` (`cpu`/`memory_gb`/`disk_size_gb`/`volumes`),
//! `api_version`, `threadsafe`, `skip_files`, `nobuild_files`,
//! `libraries`, `derived_file_type`, `env`, `flexible_runtime_settings`,
//! `endpoints_api_service`, `main`.
//!
//! ```rust
//! let k = b"runtime: nodejs20\ninstance_class: F1\nhandlers:\n- url: /.*\n  script: auto\n";
//! assert!(izanagi_kit::appengine::detect(k));
//! ```

/// app.yaml census.
#[derive(Debug, Clone)]
pub struct Appengine {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised keys present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "runtime",
    "instance_class",
    "runtime_config",
    "env_variables",
    "handlers",
    "inbound_services",
    "automatic_scaling",
    "basic_scaling",
    "manual_scaling",
    "vpc_access_connector",
    "entrypoint",
    "build_env_variables",
    "default_expiration",
    "error_handlers",
    "beta_settings",
    "liveness_check",
    "readiness_check",
    "resources",
    "threadsafe",
    "skip_files",
    "nobuild_files",
    "libraries",
    "derived_file_type",
    "flexible_runtime_settings",
    "endpoints_api_service",
    "static_files",
    "static_dir",
    "redirect_http_response_code",
    "application_readable",
    "auth_fail_action",
    "cool_down_period_sec",
    "target_cpu_utilization",
    "target_throughput_utilization",
    "max_idle_instances",
    "min_idle_instances",
    "max_concurrent_requests",
    "max_instances",
    "min_instances",
    "max_pending_latency",
    "min_pending_latency",
    "memory_gb",
    "disk_size_gb",
    "volumes",
    "instance_tag",
    "forwarded_ports",
    "subnetwork_name",
    "service_account",
    "check_interval_sec",
    "timeout_sec",
    "success_threshold",
    "failure_threshold",
    "api_version",
    "includes",
];

const WEAK: &[&str] = &[
    "url",
    "host",
    "script",
    "login",
    "secure",
    "expiration",
    "http_headers",
    "mime_type",
    "service",
    "module",
    "version",
    "env",
    "network",
    "name",
    "main",
    "cpu",
    "health_check",
    "network_check",
    "app_yaml_apis",
    "no_error_on_missing_custom",
    "packages",
];

fn key(line: &str) -> Option<&str> {
    let s = line.trim_start_matches('-').trim_start();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"').trim_matches('\'');
            if k.is_empty() || k.contains(' ') {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect an `app.yaml` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `runtime`+`handlers`/`instance_class`/`*_scaling`/`env_variables`
    // are app.yaml-exclusive combos.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Appengine {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
            }
            if let Some(k) = key(line) {
                c.settings += 1;
                if STRONG.contains(&k) || WEAK.contains(&k) {
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
        let b = b"runtime: nodejs20\ninstance_class: F1\nhandlers:\n- url: /.*\n  script: auto\n";
        assert!(detect(b));
        let c = Appengine::parse(b).unwrap();
        assert!(c.keys >= 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"name: x\nversion: 1\nenv: prod\n"));
        assert!(!detect(
            b"# runtime: nodejs20\n# instance_class: F1\nservice: x\n"
        ));
    }
}
