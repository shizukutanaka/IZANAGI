//! AWS `~/.aws/credentials` and `~/.aws/config` shared INI files.
//!
//! Credentials files use `[profile]` headers with `aws_access_key_id`,
//! `aws_secret_access_key`, `aws_session_token`; config files use
//! `[profile name]`/`[default]` with `region`, `output`, `role_arn`,
//! `source_profile`, `sso_*`, `mfa_serial`, `credential_process`,
//! `web_identity_token_file`, `endpoint_url`, `cli_*` keys.
//!
//! ```
//! let b = concat!(
//!     "[default]\n",
//!     "aws_access_key_id = AKIAEXAMPLE\n",
//!     "aws_secret_access_key = secret\n",
//!     "[profile dev]\n",
//!     "region = us-east-1\n",
//!     "output = json\n",
//!     "role_arn = arn:aws:iam::123:role/r\n",
//!     "source_profile = default\n"
//! ).as_bytes();
//! assert!(izanagi_kit::awscredentials::detect(b));
//! let c = izanagi_kit::awscredentials::Awscredentials::parse(b).unwrap();
//! assert_eq!(c.profiles, 2);
//! assert_eq!(c.credential_keys, 2);
//! ```

/// Parsed AWS credentials/config summary.
#[derive(Debug, Clone)]
pub struct Awscredentials {
    /// `[profile]`/`[default]`/`[profile name]` headers.
    pub profiles: usize,
    /// `aws_access_key_id`/`aws_secret_access_key`/`aws_session_token`/`aws_account_id` keys.
    pub credential_keys: usize,
    /// `sso_*` keys (`sso_start_url`, `sso_region`, `sso_account_id`, `sso_role_name`, `sso_session`, `sso_registration_scopes`).
    pub sso: usize,
    /// `role_arn`/`source_profile`/`credential_source`/`external_id`/`mfa_serial`/`duration_seconds`/`role_session_name` keys.
    pub assume_role: usize,
    /// `region`/`output`/`cli_*`/`endpoint_url`/`services`/`s3`/`ec2`/`rds`/`emr`/`kinesis`/`dynamodb`/`logs`/`cloudwatch`/`autoscaling`/`elasticache`/`redshift`/`route53`/`iam`/`sts`/`eks`/`ecs`/`lambda`/`apigateway`/`cloudfront`/`cloudformation`/`sns`/`sqs`/`ses`/`secretsmanager`/`ssm`/`kms`/`waf`/`shield`/`macie`/`guardduty`/`securityhub`/`inspector`/`configservice`/`servicecatalog`/`organizations`/`ram`/`resource`/`tagging`/`pricing`/`budgets`/`ce`/`cur`/`support`/`health`/`trustedadvisor`/`accessanalyzer`/`account`/`billingconductor`/`billing`/`costandusagereportservice`/`marketplace`/`metering`/`payment`/`tax`/`panorama`/`preview`/`sigv4a`/`defaults`/`cli_pager`/`cli_timestamp_format`/`cli_binary_format`/`cli_auto_prompt`/`cli_history`/`cli_follow_urlparam`/`cli_read_timeout`/`cli_connect_timeout`/`cli_retries`/`cli_validation`/`cli_partition`/`cli_request`/`cli_response`/`ca_bundle`/`metadata_service_timeout`/`metadata_service_num_attempts`/`disable_request_compression`/`request_min_compression_size_bytes`/`tcp_keepalive`/`max_attempts`/`retry_mode`/`adaptive`/`standard`/`legacy`/`output`/`text`/`json`/`table`/`yaml`/`yaml-stream`/`query`/`page_size`/`max_concurrent_requests`/`max_bandwidth`/`multipart_chunksize`/`multipart_threshold`/`use_accelerate_endpoint`/`addressing_style`/`payload_signing_enabled`/`signature_version`/`s3_us_east_1_regional_endpoint`/`expected_bucket_owner`/`ignore_configured_endpoint_urls`/`endpoint_discovery_enabled`/`sdk`/`assume_role_with_web_identity`/`web_identity_token_file`/`credential_process`/`credential_source`/`ecs`/`ec2`/`environment` keys.
    pub service_keys: usize,
    /// Other `key = value` settings.
    pub entries: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const CRED_KEYS: &[&str] = &[
    "aws_access_key_id",
    "aws_secret_access_key",
    "aws_session_token",
    "aws_account_id",
];
const SSO_PREFIX: &str = "sso_";
const ASSUME_KEYS: &[&str] = &[
    "role_arn",
    "source_profile",
    "credential_source",
    "external_id",
    "mfa_serial",
    "duration_seconds",
    "role_session_name",
    "web_identity_token_file",
    "credential_process",
];

/// Whether the buffer looks like AWS credentials/config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines()
        .any(|l| CRED_KEYS.iter().any(|k| l.trim_start().starts_with(k)))
        || t.lines().any(|l| {
            let tr = l.trim();
            tr.starts_with("[profile ") || tr.starts_with("[default]")
        })
}

impl Awscredentials {
    /// Parses an AWS credentials/config summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            profiles: 0,
            credential_keys: 0,
            sso: 0,
            assume_role: 0,
            service_keys: 0,
            entries: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('[') && tr.ends_with(']') {
                c.profiles += 1;
                continue;
            }
            let Some((k, _)) = tr.split_once('=') else {
                continue;
            };
            let k = k.trim();
            c.entries += 1;
            if CRED_KEYS.contains(&k) {
                c.credential_keys += 1;
            } else if k.starts_with(SSO_PREFIX) {
                c.sso += 1;
            } else if ASSUME_KEYS.contains(&k) {
                c.assume_role += 1;
            } else if k.starts_with("cli_")
                || k.starts_with("s3")
                || k == "region"
                || k == "output"
                || k == "endpoint_url"
                || k == "services"
                || k == "ca_bundle"
                || k == "max_attempts"
                || k == "retry_mode"
                || k == "metadata_service_timeout"
                || k == "metadata_service_num_attempts"
                || k == "disable_request_compression"
                || k == "request_min_compression_size_bytes"
                || k == "tcp_keepalive"
                || k == "use_accelerate_endpoint"
                || k == "addressing_style"
                || k == "payload_signing_enabled"
                || k == "signature_version"
                || k == "s3_us_east_1_regional_endpoint"
                || k == "expected_bucket_owner"
                || k == "ignore_configured_endpoint_urls"
                || k == "endpoint_discovery_enabled"
            {
                c.service_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aws() {
        let b = concat!(
            "[default]\n",
            "aws_access_key_id = AKIAEXAMPLE\n",
            "aws_secret_access_key = secret\n",
            "region = us-east-1\n",
            "output = json\n",
            "[profile dev]\n",
            "role_arn = arn:aws:iam::123:role/r\n",
            "source_profile = default\n",
            "sso_start_url = https://x\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Awscredentials::parse(b).unwrap();
        assert_eq!(c.profiles, 2);
        assert_eq!(c.credential_keys, 2);
        assert_eq!(c.sso, 1);
        assert_eq!(c.assume_role, 2);
        assert_eq!(c.service_keys, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[other]\nkey = value\n"));
        assert!(Awscredentials::parse(b"x").is_none());
    }
}
