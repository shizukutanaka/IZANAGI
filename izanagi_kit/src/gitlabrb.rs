//! GitLab omnibus `gitlab.rb` parser.
//!
//! Detects the omnibus configuration by `external_url` plus
//! `gitlab_rails['key']`/`nginx['key']`/`postgresql['key']` bracketed
//! component settings, and counts structure.
//!
//! ```
//! let b = b"external_url 'https://gitlab.example.com'\ngitlab_rails['gitlab_ssh_host'] = 'gitlab.example.com'\nnginx['listen_port'] = 443\npostgresql['shared_buffers'] = '256MB'\nsidekiq['concurrency'] = 25\n";
//! assert!(izanagi_kit::gitlabrb::detect(b));
//! let c = izanagi_kit::gitlabrb::Gitlabrb::parse(b).unwrap();
//! assert!(c.service_keys >= 4);
//! ```

/// Parsed gitlab.rb summary.
#[derive(Debug, Clone)]
pub struct Gitlabrb {
    /// Recognized directive occurrences.
    pub keys: usize,
    /// Component prefixes (`gitlab_rails[`/`nginx[`/`postgresql[`/`puma[`/`redis[`/`sidekiq[`/`gitaly[`/`gitlab_workhorse[`/`registry[`/`mattermost[`/`pages[`/`monitoring[`/`logging[`/`letsencrypt[`/`prometheus[`/`alertmanager[`/`grafana[`/`consul[`/`patroni[`/`pgbouncer[`/`praefect[`/`sentinel[`/`spamcheck[`/`gitlab_exporter[`/`gitlab_kas[`/`gitlab_sshd[`/`logrotate[`/`node_exporter[`/`postgres_exporter[`/`redis_exporter[`/`gitlab_ci[`/`ci[`).
    pub service_keys: usize,
    /// `key['sub']['deep']` nested settings (count of `['` tokens).
    pub nested_keys: usize,
    /// `external_url`/`registry_external_url`/`*_external_url` URL directives.
    pub url_keys: usize,
    /// `key = value` / `key 'value'` setting lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Component prefixes.
const SERVICE_KEYS: &[&str] = &[
    "gitlab_rails[",
    "gitlab_workhorse[",
    "gitlab_kas[",
    "gitlab_sshd[",
    "gitlab_exporter[",
    "gitlab_ci[",
    "nginx[",
    "puma[",
    "postgresql[",
    "redis[",
    "sidekiq[",
    "gitaly[",
    "registry[",
    "mattermost[",
    "pages[",
    "monitoring[",
    "logging[",
    "letsencrypt[",
    "prometheus[",
    "alertmanager[",
    "grafana[",
    "consul[",
    "patroni[",
    "pgbouncer[",
    "praefect[",
    "sentinel[",
    "spamcheck[",
    "logrotate[",
    "node_exporter[",
    "postgres_exporter[",
    "redis_exporter[",
    "ci[",
];

/// URL directives.
const URL_KEYS: &[&str] = &[
    "external_url",
    "registry_external_url",
    "mattermost_external_url",
    "pages_external_url",
];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "external_url",
    "gitlab_rails[",
    "nginx[",
    "postgresql[",
    "puma[",
    "registry_external_url",
    "mattermost_external_url",
    "pages_external_url",
    "gitaly[",
    "sidekiq[",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a gitlab.rb.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Gitlabrb {
    /// Count categories in a gitlab.rb. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            service_keys: 0,
            nested_keys: 0,
            url_keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.is_empty() {
                continue;
            }
            if tr.contains('=') || tr.contains("external_url") {
                c.assignments += 1;
            }
        }
        for k in SERVICE_KEYS {
            c.service_keys += t.matches(k).count();
        }
        for k in URL_KEYS {
            c.url_keys += t.matches(k).count();
        }
        c.nested_keys = t.matches("['").count();
        c.keys = c.service_keys + c.url_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# omnibus config\nexternal_url 'https://gitlab.example.com'\nregistry_external_url 'https://reg.example.com'\ngitlab_rails['gitlab_ssh_host'] = 'gitlab.example.com'\ngitlab_rails['time_zone'] = 'UTC'\nnginx['listen_port'] = 443\npostgresql['shared_buffers'] = '256MB'\nsidekiq['concurrency'] = 25\n";
        assert!(detect(b));
        let c = Gitlabrb::parse(b).unwrap();
        assert!(c.service_keys >= 4);
        assert_eq!(c.url_keys, 3);
        assert!(c.nested_keys >= 5);
        assert_eq!(c.comments, 1);
        assert!(c.keys >= 7);
    }

    #[test]
    fn rejects_ruby() {
        assert!(!detect(b"def foo\n  1\nend\n"));
        assert!(Gitlabrb::parse(b"x['y'] = 1\n").is_none());
    }
}
