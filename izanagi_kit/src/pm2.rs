//! PM2 `ecosystem.config.js`/`ecosystem.yml`/`process.yml` census.
//!
//! PM2 process files list `apps:` (YAML) or `apps: [` /
//! `module.exports = { apps: [...] }` (JS) entries with
//! `name`/`script`/`args`/`cwd`/`instances`/`exec_mode`/
//! `watch`/`ignore_watch`/`max_memory_restart`/`env`/`env_*`/
//! `log_file`/`out_file`/`error_file`/`time`/`merge_logs`/
//! `autorestart`/`restart_delay`/`exp_backoff_restart_delay`/
//! `max_restarts`/`min_uptime`/`kill_timeout`/`wait_ready`/
//! `listen_timeout`/`interpreter`/`interpreter_args`/`source_map_support`/
//! `node_args`/`vizion`/`post_update`/`force`/`cron_restart`/
//! `instance_var`/`pmx`/`automation`/`treekill`/`increment_var`/
//! `deploy` sections.
//!
//! ```rust
//! let c = izanagi_kit::pm2::Pm2::parse(
//!     b"module.exports={apps:[{name:'api',script:'./app.js',instances:2}]}\n",
//! ).unwrap();
//! assert_eq!(c.apps, 1);
//! ```

/// PM2 process file census.
#[derive(Debug, Clone)]
pub struct Pm2 {
    /// App entries (`name:`/`name=` inside apps).
    pub apps: usize,
    /// Option keys recognized.
    pub keys: usize,
    /// `env`/`env_*` blocks.
    pub envs: usize,
    /// `//`/`#`/`/* */` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "name",
    "script",
    "args",
    "cwd",
    "instances",
    "exec_mode",
    "watch",
    "ignore_watch",
    "max_memory_restart",
    "env",
    "log_file",
    "out_file",
    "error_file",
    "log_date_format",
    "time",
    "merge_logs",
    "autorestart",
    "restart_delay",
    "exp_backoff_restart_delay",
    "max_restarts",
    "min_uptime",
    "kill_timeout",
    "wait_ready",
    "listen_timeout",
    "interpreter",
    "interpreter_args",
    "source_map_support",
    "node_args",
    "vizion",
    "post_update",
    "force",
    "cron_restart",
    "instance_var",
    "pmx",
    "automation",
    "treekill",
    "increment_var",
    "deploy",
    "uid",
    "gid",
    "append_env_to_name",
    "namespace",
    "shutdown_with_message",
    "disable_logs",
    "pid_file",
];

fn count_apps(t: &str) -> usize {
    let mut n = 0;
    let mut rest = t;
    while let Some(i) = rest.find("name") {
        let tail = &rest[i + 4..];
        if tail.trim_start().starts_with(':') || tail.trim_start().starts_with('=') {
            n += 1;
        }
        rest = tail;
    }
    n
}

/// Whether the buffer looks like a PM2 process file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("apps:")
        || t.contains("apps =")
        || t.contains("\"apps\"")
        || t.contains("- apps")
        || t.contains("apps:"))
        && (t.contains("script") || t.contains("exec_mode") || t.contains("instances"))
        && KEYS
            .iter()
            .filter(|k| {
                t.contains(&format!("{k}:"))
                    || t.contains(&format!("{k} ="))
                    || t.contains(&format!("{k}="))
            })
            .count()
            >= 3
}

impl Pm2 {
    /// Parse a PM2 process file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            apps: 0,
            keys: 0,
            envs: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with('#') || s.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            for k in KEYS {
                if s.starts_with(k)
                    && (s[k.len()..].starts_with(':') || s[k.len()..].starts_with('='))
                {
                    c.keys += 1;
                    if *k == "env" {
                        c.envs += 1;
                    }
                    break;
                }
            }
        }
        c.apps = count_apps(t);
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ecosystem() {
        let b = concat!(
            "// ecosystem\n",
            "module.exports = {\n",
            "  apps: [{\n",
            "    name: 'api',\n",
            "    script: './app.js',\n",
            "    instances: 2,\n",
            "    exec_mode: 'cluster',\n",
            "    env: { NODE_ENV: 'production' },\n",
            "    max_memory_restart: '500M'\n",
            "  }, {\n",
            "    name: 'worker',\n",
            "    script: './job.js'\n",
            "  }]\n",
            "}\n",
        );
        let c = Pm2::parse(b.as_bytes()).unwrap();
        assert_eq!(c.apps, 2);
        assert_eq!(c.keys, 8);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Pm2::parse(b"apps:\n  - x\n").is_none());
    }
}
