//! Tiltfile (Starlark) census.
//!
//! Function-call statements: `local()`, `docker_build()`,
//! `custom_build()`, `k8s_yaml()`, `k8s_resource()`,
//! `helm()`, `kustomize()`, `docker_compose()`,
//! `local_resource()`, `allow_k8s_contexts()`,
//! `default_registry()`, `watch_file()`, `config.parse()`,
//! `config.define_string()`, `config.define_bool()`,
//! `secret_create_generic()`, `secret_yaml_registry()`,
//! `port_forward()`, `update_settings()`, `k8s_kind()`,
//! `k8s_image_json_path()`, `workload_to_resource_function()`,
//! `trigger_mode()`, `disable_snapshots()`, `blob()`,
//! `listdir()`, `read_file()`, `watch()`, `load()`,
//! `include()`, `os.environ`, `fail()`, `print()`,
//! `decode_json()`, `encode_json()`, `read_yaml()`,
//! `read_json()`, `helm_remote()`, `docker_prune_settings()`,
//! `analytics_settings()`, `version_settings()`,
//! `load_dynamic()`, `structured_merge_patch()`,
//! `docker_build_sub()`, `helm_resource()`,
//! `k8s_context()`, `k8s_namespace()`, `k8s_custom_deploy()`,
//! `k8s_custom_deploy()`, `local_serial()`,
//! `docker_build_registry`, `dc_resource()`,
//! `v1alpha1.extension_repo()`, `v1alpha1.extension()`,
//! `os.getcwd()`, `os.path.join`, `str.replace`.
//!
//! ```rust
//! let t = "k8s_yaml('deploy/app.yaml')\ndocker_build('app', '.', live_update=[sync('./src', '/app/src')])\nk8s_resource('app', port_forwards=8080)\n";
//! let c = izanagi_kit::tiltfile::Tiltfile::parse(t.as_bytes()).unwrap();
//! assert_eq!(c.calls, 3);
//! ```

/// Tiltfile census.
#[derive(Debug, Clone)]
pub struct Tiltfile {
    /// `name(...)` call sites.
    pub calls: usize,
    /// Resource-ish calls (docker_build/k8s_resource/local_resource/local/docker_compose/helm/custom_build/dc_resource).
    pub resources: usize,
    /// `load()`/`include()` dependency lines.
    pub loads: usize,
    /// `key = value` assignments.
    pub settings: usize,
    /// Recognised function names.
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const FUNCS: &[&str] = &[
    "local",
    "docker_build",
    "custom_build",
    "k8s_yaml",
    "k8s_resource",
    "helm",
    "helm_remote",
    "kustomize",
    "docker_compose",
    "dc_resource",
    "local_resource",
    "allow_k8s_contexts",
    "default_registry",
    "watch_file",
    "config.parse",
    "config.define_string",
    "config.define_bool",
    "config.define_int",
    "config.define_string_list",
    "secret_create_generic",
    "secret_yaml_registry",
    "port_forward",
    "update_settings",
    "k8s_kind",
    "k8s_image_json_path",
    "k8s_custom_deploy",
    "workload_to_resource_function",
    "trigger_mode",
    "disable_snapshots",
    "blob",
    "listdir",
    "read_file",
    "watch",
    "load",
    "load_dynamic",
    "include",
    "fail",
    "print",
    "decode_json",
    "encode_json",
    "read_yaml",
    "read_json",
    "docker_prune_settings",
    "analytics_settings",
    "version_settings",
    "structured_merge_patch",
    "docker_build_sub",
    "helm_resource",
    "k8s_context",
    "k8s_namespace",
    "k8s_attach",
    "os.environ.get",
    "os.getcwd",
    "os.path.join",
    "sync",
    "run",
    "live_update",
    "fall_back_on",
    "restart_container",
    "ignore",
    "v1alpha1.extension_repo",
    "v1alpha1.extension",
    "v1alpha1.file_watch",
    "probe",
    "http_get_action",
    "exec_action",
    "sleep",
    "jobs",
    "cert_generated",
];

const RESOURCE_FUNCS: &[&str] = &[
    "docker_build",
    "custom_build",
    "k8s_resource",
    "local_resource",
    "local",
    "docker_compose",
    "dc_resource",
    "helm_resource",
    "k8s_custom_deploy",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect Tiltfile content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        for f in [
            "k8s_yaml",
            "k8s_resource",
            "docker_build",
            "local_resource",
            "docker_compose",
            "allow_k8s_contexts",
            "tiltfile",
        ] {
            if s.starts_with(f) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

impl Tiltfile {
    /// Census a Tiltfile buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            calls: 0,
            resources: 0,
            loads: 0,
            settings: 0,
            named: 0,
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
            if let Some(eq) = s.find('=') {
                let lhs = s[..eq].trim();
                let rhs = s[eq + 1..].trim_start();
                if !lhs.contains('(')
                    && !rhs.starts_with('[')
                    && lhs
                        .bytes()
                        .all(|x| x.is_ascii_alphanumeric() || x == b'_' || x == b'.' || x == b' ')
                {
                    c.settings += 1;
                    continue;
                }
            }
            let head = s
                .split('(')
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches("os.environ[")
                .trim_matches('"');
            if s.contains('(') {
                c.calls += 1;
                if FUNCS.contains(&head) {
                    c.named += 1;
                }
                if RESOURCE_FUNCS.contains(&head) {
                    c.resources += 1;
                }
                if head == "load" || head == "include" || head == "load_dynamic" {
                    c.loads += 1;
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
    fn detects_tiltfile() {
        let b = b"k8s_yaml('a.yaml')\ndocker_build('app', '.')\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_tiltfile() {
        let b = concat!(
            "# Tiltfile\n",
            "load('ext://restart_process', 'docker_build_with_restart')\n",
            "allow_k8s_contexts('kind-dev')\n",
            "default_registry('registry.local:5000')\n",
            "k8s_yaml('deploy/app.yaml')\n",
            "k8s_yaml(kustomize('k8s/overlays/dev'))\n",
            "docker_build('app', '.', dockerfile='Dockerfile',\n",
            "  live_update=[sync('./src', '/app/src'), run('make build')])\n",
            "custom_build('worker', './build.sh', deps=['./src'])\n",
            "local_resource('lint', 'make lint', deps=['./src'])\n",
            "local('make gen')\n",
            "docker_compose('docker-compose.yml')\n",
            "k8s_resource('app', port_forwards=[8080, 9090],\n",
            "  resource_deps=['db'], trigger_mode=TRIGGER_MODE_MANUAL)\n",
            "helm('charts/app', name='app', namespace='prod')\n",
            "watch_file('config/settings.yaml')\n",
            "update_settings(max_parallel_updates=4)\n",
            "IMG = 'app'\n",
            "print('done')\n",
        );
        let c = Tiltfile::parse(b.as_bytes()).unwrap();
        assert_eq!(c.calls, 16);
        assert!(c.resources >= 6);
        assert_eq!(c.loads, 1);
        assert!(c.named >= 14);
        assert_eq!(c.settings, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
