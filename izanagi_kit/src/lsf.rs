//! IBM LSF `lsb.queues`/`lsb.hosts`/`lsb.users`/`lsb.params`/`lsb.conf` census.
//!
//! LSF batch config files group `PARAM = value` settings inside
//! `Begin Queue <name>` / `Begin Host` / `Begin User` /
//! `Begin Parameter` / `Begin Resource` / `Begin Limit` /
//! `Begin Cluster` blocks closed by a matching `End X`. Queue
//! parameters include `QUEUE_NAME`/`PRIORITY`/`NICE`/`PREEMPTION`/
//! `FAIRSHARE`/`RUNLIMIT`/`CPULIMIT`/`MEMLIMIT`/`SWAPLIMIT`/
//! `PROCESSLIMIT`/`MAX_RSCHED_TIME`/`HOSTS`/`USERS`/`RES_REQ`/
//! `REQUEUE_EXIT_VALUES`/`EXCLUSIVE`/`BACKFILL`/`SLOT_POOL`/
//! `JOB_CONTROLS`/`DESCRIPTION`; host/params files use
//! `HOST_NAME`/`MXJ`/`MODEL`/`FACTOR`/`server`/`ENABLE_INTERACTIVE`/
//! `DEFAULT_QUEUE`/`RUN_WINDOW`/`JOB_ACCEPT_INTERVAL`/`MBD_SLEEP_TIME`/
//! `DEFAULT_HOST_SPEC`/`PRODUCT`/`LSF_LOGDIR`/`LSF_ENVDIR`/
//! `LSF_SERVERDIR`/`LSF_CONFDIR`/`LSF_LIC_API_*`/`LSB_*`/`LSF_*`.
//!
//! ```rust
//! let c = izanagi_kit::lsf::Lsf::parse(
//!     b"Begin Queue\nQUEUE_NAME = normal\nPRIORITY = 30\nEnd Queue\n",
//! ).unwrap();
//! assert_eq!(c.blocks, 1);
//! ```

/// LSF config census.
#[derive(Debug, Clone)]
pub struct Lsf {
    /// `Begin …`/`End …` blocks.
    pub blocks: usize,
    /// `PARAM = value` parameter lines.
    pub params: usize,
    /// `QUEUE_NAME`/`HOST_NAME`/`USER_NAME` definitions.
    pub named_entities: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const BLOCK_HEADS: &[&str] = &[
    "Begin Queue",
    "Begin Host",
    "Begin User",
    "Begin Parameter",
    "Begin Resource",
    "Begin Limit",
    "Begin Cluster",
    "Begin UserGroup",
    "Begin HostGroup",
    "Begin QueueGroup",
    "Begin HostPartition",
    "Begin ShareAssignment",
    "Begin ServiceClasses",
];

const PARAMS: &[&str] = &[
    "QUEUE_NAME",
    "PRIORITY",
    "NICE",
    "PREEMPTION",
    "FAIRSHARE",
    "RUNLIMIT",
    "CPULIMIT",
    "MEMLIMIT",
    "SWAPLIMIT",
    "PROCESSLIMIT",
    "MAX_RSCHED_TIME",
    "HOSTS",
    "USERS",
    "RES_REQ",
    "REQUEUE_EXIT_VALUES",
    "EXCLUSIVE",
    "BACKFILL",
    "SLOT_POOL",
    "JOB_CONTROLS",
    "DESCRIPTION",
    "HOST_NAME",
    "MXJ",
    "MODEL",
    "FACTOR",
    "USER_NAME",
    "MAX_JOBS",
    "JL_P",
    "MAX_PEND_JOBS",
    "DEFAULT_QUEUE",
    "RUN_WINDOW",
    "DISPATCH_WINDOW",
    "JOB_ACCEPT_INTERVAL",
    "MBD_SLEEP_TIME",
    "DEFAULT_HOST_SPEC",
    "PRODUCT",
    "LSF_LOGDIR",
    "LSF_ENVDIR",
    "LSF_SERVERDIR",
    "LSF_CONFDIR",
    "LSF_DEBUG",
    "LSB_DEBUG",
    "ENABLE_INTERACTIVE",
];

fn param_of(s: &str) -> &str {
    s.split('=').next().unwrap_or("").trim()
}

/// Whether the buffer looks like an LSF config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    BLOCK_HEADS.iter().any(|h| t.contains(h))
        && t.lines()
            .filter(|l| {
                let s = l.trim();
                s.contains('=') && PARAMS.contains(&param_of(s))
            })
            .count()
            >= 1
}

impl Lsf {
    /// Parse an LSF config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            blocks: 0,
            params: 0,
            named_entities: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if BLOCK_HEADS.iter().any(|h| s.starts_with(h)) {
                c.blocks += 1;
                continue;
            }
            if s.starts_with("End ") {
                continue;
            }
            if s.contains('=') {
                c.params += 1;
                match param_of(s) {
                    "QUEUE_NAME" | "HOST_NAME" | "USER_NAME" => c.named_entities += 1,
                    _ => {}
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
    fn parses_lsb_queues() {
        let b = concat!(
            "# lsb.queues\n",
            "Begin Queue\n",
            "QUEUE_NAME = normal\n",
            "PRIORITY = 30\n",
            "HOSTS = all\n",
            "End Queue\n",
            "Begin Queue\n",
            "QUEUE_NAME = night\n",
            "RUN_WINDOW = 20:00-8:00\n",
            "End Queue\n",
        );
        let c = Lsf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.blocks, 2);
        assert_eq!(c.params, 5);
        assert_eq!(c.named_entities, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Lsf::parse(b"Begin Foo\nx=1\nEnd Foo\n").is_none());
    }
}
