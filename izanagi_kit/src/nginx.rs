//! nginx.conf / NGINX configuration file detection and census.
//!
//! Detects `events {`+`worker_*` or `http {`/`stream {`/`mail {` contexts
//! and counts context blocks (`events`/`http`/`server`/`location`/`upstream`/
//! `stream`/`mail`/`types`/`geo`/`map`/`split_clients`/`charset_map`/
//! `limit_except`/`if`/`match`/`perl`/`keyval`/`zone_sync`/`queue`), core
//! directives (~200 keys: `worker_processes`/`worker_connections`/`error_log`/
//! `pid`/`access_log`/`include`/`listen`/`server_name`/`root`/`index`/
//! `proxy_pass`/`proxy_set_header`/`fastcgi_pass`/`rewrite`/`return`/`try_files`/
//! `add_header`/`gzip`/`expires`/`client_max_body_size`/`keepalive_timeout`/
//! `sendfile`/`tcp_nopush`/`tcp_nodelay`/`server_tokens`/`limit_req_zone`/
//! `limit_conn_zone`/`proxy_cache_path`/`proxy_cache`/`proxy_cache_valid`/
//! `proxy_buffering`/`proxy_buffers`/`proxy_buffer_size`/`fastcgi_param`/
//! `uwsgi_pass`/`scgi_pass`/`grpc_pass`/`set`/`deny`/`allow`/`auth_basic`/
//! `auth_basic_user_file`/`satisfy`/`error_page`/`internal`/`resolver`/
//! `keepalive_requests`/`keepalive_time`/`limit_rate`/`limit_except`/
//! `default_type`/`types`/`charset`/`etag`/`log_not_found`/`merge_slashes`/
//! `underscores_in_headers`/`ignore_invalid_headers`/`recursive_error_pages`/
//! `absolute_redirect`/`server_name_in_redirect`/`port_in_redirect`/`aio`/
//! `directio`/`read_ahead`/`send_lowat`/`sendfile_max_chunk`/`postpone_output`/
//! `output_buffers`/`subrequest_output_buffer_size`/`variables_hash_max_size`/
//! `variables_hash_bucket_size`/`server_names_hash_max_size`/
//! `server_names_hash_bucket_size`/`types_hash_max_size`/`types_hash_bucket_size`/
//! `large_client_header_buffers`/`client_body_buffer_size`/`client_body_temp_path`/
//! `client_body_timeout`/`client_header_buffer_size`/`client_header_timeout`/
//! `connection_pool_size`/`request_pool_size`/`max_ranges`/`open_file_cache`/
//! `open_file_cache_valid`/`open_file_cache_min_uses`/`open_file_cache_errors`/
//! `reset_timedout_connection`/`lingering_close`/`lingering_time`/
//! `lingering_timeout`/`worker_rlimit_nofile`/`worker_cpu_affinity`/
//! `worker_priority`/`worker_shutdown_timeout`/`working_directory`/`daemon`/
//! `master_process`/`user`/`group`/`env`/`debug_points`/`ssl_engine`/
//! `thread_pool`/`lock_file`/`pcre_jit`/`timer_resolution`/`accept_mutex`/
//! `accept_mutex_delay`/`multi_accept`/`use`/`load_module`/`perl_modules`/
//! `perl_require`/`perl_set`/`ssi`/`ssi_silent_errors`/`ssi_types`/`xml_entities`/
//! `xslt_param`/`xslt_string_param`/`xslt_stylesheet`/`xslt_types`/
//! `chunked_transfer_encoding`/`if_modified_since`/`sub_filter`/
//! `sub_filter_last_modified`/`sub_filter_once`/`sub_filter_types`/`
//! `health_check`/`queue`/`slow_start`/`state`/`ntlm`/`sticky`/`ip_hash`/
//! `least_conn`/`least_time`/`hash`/`random`/`server`/`zone`/`keepalive`/
//! `keepalive_requests`/`keepalive_time`/`keepalive_timeout`/`resolver`/
//! `max_conns`/`max_fails`/`fail_timeout`/`backup`/`down`/`drain`/`resolve`/
//! `service`/`weight`), `location`/`@named` location lines, SSL keys
//! (`ssl_certificate`/`ssl_certificate_key`/`ssl_protocols`/`ssl_ciphers`/
//! `ssl_prefer_server_ciphers`/`ssl_session_cache`/`ssl_session_tickets`/
//! `ssl_session_timeout`/`ssl_stapling`/`ssl_stapling_verify`/
//! `ssl_trusted_certificate`/`ssl_dhparam`/`ssl_ecdh_curve`/`ssl_buffer_size`/
//! `ssl_verify_client`/`ssl_verify_depth`/`ssl_client_certificate`/`ssl_crl`/
//! `ssl_password_file`/`ssl_conf_command`/`ssl_reject_handshake`/
//! `ssl_early_data`/`ssl_ocsp`/`ssl_ocsp_cache`/`ssl_verify`/
//! `proxy_ssl_certificate`/`proxy_ssl_certificate_key`/`proxy_ssl_ciphers`/
//! `proxy_ssl_protocols`/`proxy_ssl_name`/`proxy_ssl_server_name`/
//! `proxy_ssl_session_reuse`/`proxy_ssl_verify`/`proxy_ssl_verify_depth`/
//! `proxy_ssl_trusted_certificate`/`proxy_ssl_conf_command`/`http2`/`quic`),
//! proxy/fastcgi keys (`proxy_pass`/`proxy_set_header`/`proxy_hide_header`/
//! `proxy_pass_header`/`proxy_redirect`/`proxy_http_version`/`proxy_method`/
//! `proxy_pass_request_headers`/`proxy_pass_request_body`/`proxy_read_timeout`/
//! `proxy_connect_timeout`/`proxy_send_timeout`/`proxy_buffering`/`proxy_buffers`/
//! `proxy_buffer_size`/`proxy_busy_buffers_size`/`proxy_max_temp_file_size`/
//! `proxy_temp_file_write_size`/`proxy_temp_path`/`proxy_cache_path`/
//! `proxy_cache`/`proxy_cache_key`/`proxy_cache_valid`/`proxy_cache_use_stale`/
//! `proxy_cache_lock`/`proxy_cache_lock_age`/`proxy_cache_lock_timeout`/
//! `proxy_no_cache`/`proxy_cache_bypass`/`proxy_cache_revalidate`/
//! `proxy_cache_min_uses`/`proxy_cache_methods`/`proxy_cache_purge`/
//! `proxy_next_upstream`/`proxy_next_upstream_timeout`/
//! `proxy_next_upstream_tries`/`proxy_store`/`proxy_store_access`/
//! `proxy_ignore_headers`/`proxy_intercept_errors`/`proxy_force_ranges`/
//! `proxy_limit_rate`/`proxy_headers_hash_max_size`/`proxy_headers_hash_bucket_size`/
//! `proxy_headers_hash`/`proxy_cookie_domain`/`proxy_cookie_path`/
//! `proxy_cookie_flags`/`proxy_bind`/`proxy_socket_keepalive`/`proxy_protocol`/
//! `proxy_ssl_verify`/`fastcgi_pass`/`fastcgi_index`/`fastcgi_param`/
//! `fastcgi_split_path_info`/`fastcgi_buffer_size`/`fastcgi_buffers`/
//! `fastcgi_buffering`/`fastcgi_busy_buffers_size`/`fastcgi_connect_timeout`/
//! `fastcgi_read_timeout`/`fastcgi_send_timeout`/`fastcgi_store`/
//! `fastcgi_store_access`/`fastcgi_cache`/`fastcgi_cache_key`/
//! `fastcgi_cache_valid`/`fastcgi_cache_lock`/`fastcgi_cache_use_stale`/
//! `fastcgi_bind`/`fastcgi_hide_header`/`fastcgi_pass_header`/
//! `fastcgi_ignore_headers`/`fastcgi_intercept_errors`/`fastcgi_keep_conn`/
//! `uwsgi_pass`/`uwsgi_param`/`uwsgi_buffer_size`/`uwsgi_buffers`/
//! `uwsgi_cache`/`uwsgi_cache_key`/`uwsgi_cache_valid`/`uwsgi_hide_header`/
//! `uwsgi_pass_header`/`uwsgi_bind`/`scgi_pass`/`scgi_param`/`scgi_buffer_size`/
//! `scgi_buffers`/`scgi_cache`/`memcached_pass`/`memcached_buffer_size`/
//! `memcached_connect_timeout`/`memcached_read_timeout`/`memcached_send_timeout`/
//! `memcached_next_upstream`/`grpc_pass`/`grpc_bind`/`grpc_buffer_size`/
//! `grpc_connect_timeout`/`grpc_read_timeout`/`grpc_send_timeout`/
//! `grpc_set_header`/`grpc_pass_header`/`grpc_hide_header`/
//! `grpc_intercept_errors`/`grpc_next_upstream`/`grpc_ssl_certificate`/
//! `websocket`/`upgrade`/`map_hash_bucket_size`/`map_hash_max_size`), and `#`
//! comment lines.
//!
//! ```
//! let b = b"events {\n    worker_connections 1024;\n}\nhttp {\n    server {\n        listen 80;\n        server_name example.com;\n        location / {\n            proxy_pass http://backend;\n        }\n    }\n}\n";
//! assert!(izanagi_kit::nginx::detect(b));
//! let c = izanagi_kit::nginx::Nginx::parse(b).unwrap();
//! assert_eq!(c.blocks, 4);
//! assert_eq!(c.locations, 1);
//! ```

/// Parsed nginx config summary.
#[derive(Debug, Clone)]
pub struct Nginx {
    /// context blocks (`events`/`http`/`server`/`upstream`/`stream`/`map`/…).
    pub blocks: usize,
    /// core directives (`worker_*`/`listen`/`root`/`try_files`/`gzip`/…).
    pub directives: usize,
    /// `location` + `@named` location lines.
    pub locations: usize,
    /// `ssl_*`/`proxy_ssl_*`/`http2`/`quic` SSL keys.
    pub ssl_keys: usize,
    /// `proxy_*`/`fastcgi_*`/`uwsgi_*`/`scgi_*`/`memcached_*`/`grpc_*` keys.
    pub proxy_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const BLOCKS: &[&str] = &[
    "events",
    "http",
    "server",
    "location",
    "upstream",
    "stream",
    "mail",
    "types",
    "geo",
    "map",
    "split_clients",
    "charset_map",
    "limit_except",
    "if",
    "match",
    "perl",
    "keyval",
    "zone_sync",
    "queue",
];

const DIRECTIVES: &[&str] = &[
    "worker_processes",
    "worker_connections",
    "worker_rlimit_nofile",
    "worker_cpu_affinity",
    "worker_priority",
    "worker_shutdown_timeout",
    "working_directory",
    "daemon",
    "master_process",
    "user",
    "group",
    "env",
    "debug_points",
    "error_log",
    "pid",
    "access_log",
    "include",
    "listen",
    "server_name",
    "root",
    "index",
    "rewrite",
    "return",
    "try_files",
    "add_header",
    "gzip",
    "gzip_static",
    "gzip_vary",
    "gzip_proxied",
    "gzip_comp_level",
    "gzip_buffers",
    "gzip_min_length",
    "gzip_types",
    "gzip_disable",
    "gzip_http_version",
    "gunzip",
    "gunzip_buffers",
    "expires",
    "client_max_body_size",
    "keepalive_timeout",
    "sendfile",
    "tcp_nopush",
    "tcp_nodelay",
    "server_tokens",
    "limit_req_zone",
    "limit_req",
    "limit_conn_zone",
    "limit_conn",
    "limit_rate",
    "limit_rate_after",
    "limit_except",
    "set",
    "deny",
    "allow",
    "auth_basic",
    "auth_basic_user_file",
    "auth_jwt",
    "auth_jwt_key_file",
    "auth_request",
    "auth_request_set",
    "satisfy",
    "error_page",
    "internal",
    "resolver",
    "resolver_timeout",
    "keepalive_requests",
    "keepalive_time",
    "keepalive_disable",
    "default_type",
    "charset",
    "charset_map",
    "source_charset",
    "etag",
    "log_not_found",
    "log_subrequest",
    "merge_slashes",
    "underscores_in_headers",
    "ignore_invalid_headers",
    "recursive_error_pages",
    "absolute_redirect",
    "server_name_in_redirect",
    "port_in_redirect",
    "aio",
    "threads",
    "directio",
    "directio_alignment",
    "read_ahead",
    "send_lowat",
    "sendfile_max_chunk",
    "postpone_output",
    "output_buffers",
    "subrequest_output_buffer_size",
    "variables_hash_max_size",
    "variables_hash_bucket_size",
    "server_names_hash_max_size",
    "server_names_hash_bucket_size",
    "types_hash_max_size",
    "types_hash_bucket_size",
    "large_client_header_buffers",
    "client_body_buffer_size",
    "client_body_in_file_only",
    "client_body_in_single_buffer",
    "client_body_temp_path",
    "client_body_timeout",
    "client_header_buffer_size",
    "client_header_timeout",
    "connection_pool_size",
    "request_pool_size",
    "max_ranges",
    "open_file_cache",
    "open_file_cache_valid",
    "open_file_cache_min_uses",
    "open_file_cache_errors",
    "reset_timedout_connection",
    "lingering_close",
    "lingering_time",
    "lingering_timeout",
    "ssl_engine",
    "thread_pool",
    "lock_file",
    "pcre_jit",
    "timer_resolution",
    "accept_mutex",
    "accept_mutex_delay",
    "multi_accept",
    "use",
    "load_module",
    "perl_modules",
    "perl_require",
    "perl_set",
    "ssi",
    "ssi_silent_errors",
    "ssi_types",
    "ssi_value_length",
    "ssi_min_file_chunk",
    "ssi_last_modified",
    "xml_entities",
    "xslt_param",
    "xslt_string_param",
    "xslt_stylesheet",
    "xslt_types",
    "xslt_last_modified",
    "chunked_transfer_encoding",
    "if_modified_since",
    "sub_filter",
    "sub_filter_last_modified",
    "sub_filter_once",
    "sub_filter_types",
    "health_check",
    "slow_start",
    "state",
    "ntlm",
    "sticky",
    "ip_hash",
    "least_conn",
    "least_time",
    "hash",
    "random",
    "zone",
    "keepalive",
    "max_conns",
    "max_fails",
    "fail_timeout",
    "backup",
    "down",
    "drain",
    "resolve",
    "service",
    "weight",
    "msie_padding",
    "msie_refresh",
    "modern_browser",
    "ancient_browser",
    "ancient_browser_value",
    "early_hints",
    "empty_gif",
    "flv",
    "mp4",
    "mp4_buffer_size",
    "mp4_max_buffer_size",
    "secure_link",
    "secure_link_md5",
    "secure_link_secret",
    "slice",
    "split_clients",
    "stub_status",
    "surrogate",
    "uninitialized_variable_warn",
    "user_id_domain",
    "user_id_expires",
    "user_id_flags",
    "user_id_mark",
    "user_id_name",
    "user_id_p3p",
    "user_id_path",
    "user_id_received",
    "user_id_service",
    "vary",
    "virtual",
    "write_timeout",
    "worker_aio_requests",
    "poll",
    "devpoll",
    "epoll",
    "eventport",
    "kqueue",
    "select",
    "post_action",
];

const SSL_KEYS: &[&str] = &[
    "ssl_certificate",
    "ssl_certificate_key",
    "ssl_protocols",
    "ssl_ciphers",
    "ssl_prefer_server_ciphers",
    "ssl_session_cache",
    "ssl_session_tickets",
    "ssl_session_ticket_key",
    "ssl_session_timeout",
    "ssl_stapling",
    "ssl_stapling_verify",
    "ssl_stapling_file",
    "ssl_stapling_responder",
    "ssl_trusted_certificate",
    "ssl_dhparam",
    "ssl_ecdh_curve",
    "ssl_buffer_size",
    "ssl_verify_client",
    "ssl_verify_depth",
    "ssl_client_certificate",
    "ssl_crl",
    "ssl_password_file",
    "ssl_conf_command",
    "ssl_reject_handshake",
    "ssl_early_data",
    "ssl_ocsp",
    "ssl_ocsp_cache",
    "ssl_ocsp_responder",
    "ssl_verify",
    "ssl_name",
    "ssl_server_name",
    "proxy_ssl_certificate",
    "proxy_ssl_certificate_key",
    "proxy_ssl_ciphers",
    "proxy_ssl_protocols",
    "proxy_ssl_name",
    "proxy_ssl_server_name",
    "proxy_ssl_session_reuse",
    "proxy_ssl_verify",
    "proxy_ssl_verify_depth",
    "proxy_ssl_trusted_certificate",
    "proxy_ssl_conf_command",
    "proxy_ssl_certificate_cache",
    "grpc_ssl_certificate",
    "grpc_ssl_certificate_key",
    "grpc_ssl_ciphers",
    "grpc_ssl_protocols",
    "grpc_ssl_name",
    "grpc_ssl_server_name",
    "grpc_ssl_session_reuse",
    "grpc_ssl_verify",
    "grpc_ssl_verify_depth",
    "grpc_ssl_trusted_certificate",
    "uwsgi_ssl_certificate",
    "uwsgi_ssl_certificate_key",
    "uwsgi_ssl_ciphers",
    "uwsgi_ssl_protocols",
    "uwsgi_ssl_name",
    "uwsgi_ssl_server_name",
    "uwsgi_ssl_session_reuse",
    "uwsgi_ssl_verify",
    "uwsgi_ssl_verify_depth",
    "uwsgi_ssl_trusted_certificate",
    "http2",
    "quic",
];

const PROXY_KEYS: &[&str] = &[
    "proxy_pass",
    "proxy_set_header",
    "proxy_hide_header",
    "proxy_pass_header",
    "proxy_redirect",
    "proxy_http_version",
    "proxy_method",
    "proxy_pass_request_headers",
    "proxy_pass_request_body",
    "proxy_read_timeout",
    "proxy_connect_timeout",
    "proxy_send_timeout",
    "proxy_buffering",
    "proxy_buffers",
    "proxy_buffer_size",
    "proxy_busy_buffers_size",
    "proxy_max_temp_file_size",
    "proxy_temp_file_write_size",
    "proxy_temp_path",
    "proxy_cache_path",
    "proxy_cache",
    "proxy_cache_key",
    "proxy_cache_valid",
    "proxy_cache_use_stale",
    "proxy_cache_lock",
    "proxy_cache_lock_age",
    "proxy_cache_lock_timeout",
    "proxy_no_cache",
    "proxy_cache_bypass",
    "proxy_cache_revalidate",
    "proxy_cache_min_uses",
    "proxy_cache_methods",
    "proxy_cache_purge",
    "proxy_cache_background_update",
    "proxy_cache_convert_head",
    "proxy_cache_header",
    "proxy_next_upstream",
    "proxy_next_upstream_timeout",
    "proxy_next_upstream_tries",
    "proxy_store",
    "proxy_store_access",
    "proxy_ignore_headers",
    "proxy_intercept_errors",
    "proxy_force_ranges",
    "proxy_limit_rate",
    "proxy_headers_hash_max_size",
    "proxy_headers_hash_bucket_size",
    "proxy_cookie_domain",
    "proxy_cookie_path",
    "proxy_cookie_flags",
    "proxy_bind",
    "proxy_socket_keepalive",
    "proxy_protocol",
    "proxy_protocol_timeout",
    "proxy_download_rate",
    "proxy_upload_rate",
    "proxy_half_close",
    "proxy_disconnect",
    "proxy_responses",
    "proxy_timeout",
    "fastcgi_pass",
    "fastcgi_index",
    "fastcgi_param",
    "fastcgi_split_path_info",
    "fastcgi_buffer_size",
    "fastcgi_buffers",
    "fastcgi_buffering",
    "fastcgi_busy_buffers_size",
    "fastcgi_max_temp_file_size",
    "fastcgi_temp_file_write_size",
    "fastcgi_temp_path",
    "fastcgi_connect_timeout",
    "fastcgi_read_timeout",
    "fastcgi_send_timeout",
    "fastcgi_store",
    "fastcgi_store_access",
    "fastcgi_cache",
    "fastcgi_cache_key",
    "fastcgi_cache_path",
    "fastcgi_cache_valid",
    "fastcgi_cache_lock",
    "fastcgi_cache_lock_age",
    "fastcgi_cache_lock_timeout",
    "fastcgi_cache_min_uses",
    "fastcgi_cache_methods",
    "fastcgi_cache_purge",
    "fastcgi_cache_use_stale",
    "fastcgi_no_cache",
    "fastcgi_cache_bypass",
    "fastcgi_bind",
    "fastcgi_force_ranges",
    "fastcgi_hide_header",
    "fastcgi_pass_header",
    "fastcgi_pass_request_body",
    "fastcgi_pass_request_headers",
    "fastcgi_ignore_client_abort",
    "fastcgi_ignore_headers",
    "fastcgi_intercept_errors",
    "fastcgi_keep_conn",
    "fastcgi_limit_rate",
    "fastcgi_next_upstream",
    "fastcgi_next_upstream_timeout",
    "fastcgi_next_upstream_tries",
    "fastcgi_request_buffering",
    "fastcgi_socket_keepalive",
    "uwsgi_pass",
    "uwsgi_param",
    "uwsgi_buffer_size",
    "uwsgi_buffers",
    "uwsgi_buffering",
    "uwsgi_busy_buffers_size",
    "uwsgi_cache",
    "uwsgi_cache_key",
    "uwsgi_cache_path",
    "uwsgi_cache_valid",
    "uwsgi_cache_lock",
    "uwsgi_cache_methods",
    "uwsgi_cache_min_uses",
    "uwsgi_cache_purge",
    "uwsgi_cache_use_stale",
    "uwsgi_no_cache",
    "uwsgi_cache_bypass",
    "uwsgi_bind",
    "uwsgi_connect_timeout",
    "uwsgi_read_timeout",
    "uwsgi_send_timeout",
    "uwsgi_hide_header",
    "uwsgi_pass_header",
    "uwsgi_ignore_headers",
    "uwsgi_intercept_errors",
    "uwsgi_limit_rate",
    "uwsgi_next_upstream",
    "uwsgi_next_upstream_timeout",
    "uwsgi_next_upstream_tries",
    "uwsgi_request_buffering",
    "uwsgi_socket_keepalive",
    "uwsgi_store",
    "uwsgi_store_access",
    "uwsgi_string",
    "uwsgi_modifier1",
    "uwsgi_modifier2",
    "uwsgi_max_temp_file_size",
    "uwsgi_temp_file_write_size",
    "uwsgi_temp_path",
    "scgi_pass",
    "scgi_param",
    "scgi_buffer_size",
    "scgi_buffers",
    "scgi_buffering",
    "scgi_cache",
    "scgi_cache_key",
    "scgi_cache_path",
    "scgi_cache_valid",
    "scgi_cache_lock",
    "scgi_cache_methods",
    "scgi_cache_min_uses",
    "scgi_cache_purge",
    "scgi_cache_use_stale",
    "scgi_no_cache",
    "scgi_cache_bypass",
    "scgi_bind",
    "scgi_connect_timeout",
    "scgi_read_timeout",
    "scgi_send_timeout",
    "scgi_hide_header",
    "scgi_pass_header",
    "scgi_ignore_headers",
    "scgi_intercept_errors",
    "scgi_limit_rate",
    "scgi_next_upstream",
    "scgi_next_upstream_timeout",
    "scgi_next_upstream_tries",
    "scgi_request_buffering",
    "scgi_socket_keepalive",
    "scgi_store",
    "scgi_store_access",
    "scgi_temp_path",
    "memcached_pass",
    "memcached_buffer_size",
    "memcached_bind",
    "memcached_connect_timeout",
    "memcached_read_timeout",
    "memcached_send_timeout",
    "memcached_next_upstream",
    "memcached_next_upstream_timeout",
    "memcached_next_upstream_tries",
    "memcached_gzip_flag",
    "memcached_socket_keepalive",
    "grpc_pass",
    "grpc_bind",
    "grpc_buffer_size",
    "grpc_connect_timeout",
    "grpc_read_timeout",
    "grpc_send_timeout",
    "grpc_set_header",
    "grpc_pass_header",
    "grpc_hide_header",
    "grpc_ignore_headers",
    "grpc_intercept_errors",
    "grpc_next_upstream",
    "grpc_next_upstream_timeout",
    "grpc_next_upstream_tries",
    "grpc_socket_keepalive",
    "websocket",
    "upgrade",
];

fn first_word(l: &str) -> &str {
    l.split(|c: char| c.is_whitespace() || c == '{' || c == ';')
        .next()
        .unwrap_or("")
}

/// Detects nginx configuration files.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    let events = t.contains("events")
        && (t.contains("worker_connections") || t.contains("worker_processes"));
    let http = (t.contains("http") || t.contains("stream"))
        && (t.contains("server {") || t.contains("server{") || t.contains("upstream"))
        && (t.contains("listen") || t.contains("server_name") || t.contains("proxy_pass"));
    events || http
}

impl Nginx {
    /// Parses an nginx config, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            blocks: 0,
            directives: 0,
            locations: 0,
            ssl_keys: 0,
            proxy_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.is_empty() {
                c.comments += usize::from(tr.starts_with('#'));
                continue;
            }
            let w = first_word(tr);
            if tr.starts_with('@') || (w == "location" && (tr.contains('{') || tr.len() > w.len()))
            {
                c.locations += usize::from(w == "location");
                c.blocks += 1;
                continue;
            }
            if w == "location" {
                c.locations += 1;
                c.blocks += 1;
                continue;
            }
            if BLOCKS.contains(&w) && tr.contains('{') {
                c.blocks += 1;
                continue;
            }
            if DIRECTIVES.contains(&w) {
                c.directives += 1;
            }
            if SSL_KEYS.contains(&w) {
                c.ssl_keys += 1;
            }
            if PROXY_KEYS.contains(&w) {
                c.proxy_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"events {\n    worker_connections 1024;\n}\nhttp {\n    upstream backend {\n        server 127:9000;\n        keepalive 32;\n    }\n    server {\n        listen 80;\n        server_name example.com;\n        location / {\n            proxy_pass http://backend;\n            proxy_set_header Host $host;\n        }\n        location ~ \\.php$ {\n            fastcgi_pass unix:/run/php.sock;\n        }\n    }\n}\n";
        let c = Nginx::parse(b).unwrap();
        assert!(c.blocks >= 5);
        assert_eq!(c.locations, 2);
        assert!(c.proxy_keys >= 2);
        assert!(c.directives >= 3);
    }

    #[test]
    fn rejects_plain_text() {
        assert!(Nginx::parse(b"hello world\n").is_none());
        assert!(!detect(b"key: value\n"));
    }
}
