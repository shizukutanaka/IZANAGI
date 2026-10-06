//! `php.ini` census.
//!
//! `;` comments, `[PHP]`/`[CLI Server]`/`[Date]`/`[Session]`/`[opcache]`/
//! `[xdebug]`/`[mbstring]`/`[mail function]`/… sections, and
//! `key = value` directives (`memory_limit`, `error_reporting`,
//! `display_errors`, `upload_max_filesize`, `extension`, `date.timezone`,
//! `session.*`, `opcache.*`, `xdebug.*` …).
//!
//! ```rust
//! let p = b"[PHP]\nengine = On\nmemory_limit = 128M\nerror_reporting = E_ALL\ndisplay_errors = Off\ndate.timezone = UTC\n";
//! assert!(izanagi_kit::phpini::detect(p));
//! let c = izanagi_kit::phpini::Phpini::parse(p).unwrap();
//! assert_eq!(c.settings, 5);
//! ```

/// php.ini census.
#[derive(Debug, Clone)]
pub struct Phpini {
    /// `key = value` lines matching a known PHP directive.
    pub settings: usize,
    /// `[Section]` headers matching a known php.ini section.
    pub sections: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

/// php.ini section headers.
const SECTIONS: &[&str] = &[
    "Assertion",
    "bcmath",
    "bz2",
    "Calendar",
    "CLI Server",
    "COM",
    "ctype",
    "curl",
    "Date",
    "dba",
    "exif",
    "ffi",
    "filter",
    "gd",
    "gettext",
    "gmp",
    "iconv",
    "imap",
    "intl",
    "ldap",
    "mail function",
    "mbstring",
    "Mime Magic",
    "MSSQL",
    "MySQLi",
    "mysqlnd",
    "ODBC",
    "opcache",
    "openssl",
    "Pcre",
    "PDO",
    "Phar",
    "PHP",
    "POSIX",
    "Session",
    "shmop",
    "SimpleXML",
    "soap",
    "sockets",
    "sodium",
    "sqlite3",
    "standard",
    "sysvmsg",
    "sysvsem",
    "sysvshm",
    "tidy",
    "tokenizer",
    "xdebug",
    "xml",
    "xmlrpc",
    "xsl",
    "Zend OPcache",
    "zip",
    "zlib",
];

/// PHP directives — exact names and dotted `ext.*` prefixes.
const KEYS: &[&str] = &[
    "allow_url_fopen",
    "allow_url_include",
    "arg_separator",
    "assert",
    "auto_append_file",
    "auto_detect_line_endings",
    "auto_globals_jit",
    "auto_prepend_file",
    "bcmath",
    "browscap",
    "cgi",
    "cli_server",
    "curl",
    "date",
    "dba",
    "default_charset",
    "default_mimetype",
    "default_socket_timeout",
    "disable_classes",
    "disable_functions",
    "display_errors",
    "display_startup_errors",
    "docref_ext",
    "docref_root",
    "doc_root",
    "enable_dl",
    "engine",
    "error_append_string",
    "error_log",
    "error_prepend_string",
    "error_reporting",
    "exif",
    "expose_php",
    "extension",
    "extension_dir",
    "fastcgi",
    "file_uploads",
    "filter",
    "from",
    "hard_timeout",
    "highlight",
    "html_errors",
    "ibcmath",
    "ignore_repeated_errors",
    "ignore_repeated_source",
    "ignore_user_abort",
    "implicit_flush",
    "include_path",
    "input_encoding",
    "internal_encoding",
    "last_modified",
    "ldap",
    "log_errors",
    "log_errors_max_len",
    "mail",
    "max_execution_time",
    "max_file_uploads",
    "max_input_nesting_level",
    "max_input_time",
    "max_input_vars",
    "mbstring",
    "memory_limit",
    "mime_magic",
    "mysqli",
    "mysqlnd",
    "opcache",
    "open_basedir",
    "output_buffering",
    "output_encoding",
    "output_handler",
    "pcre",
    "pdo",
    "phar",
    "post_max_size",
    "precision",
    "realpath_cache_size",
    "realpath_cache_ttl",
    "register_argc_argv",
    "report_memleaks",
    "request_order",
    "sendmail_from",
    "sendmail_path",
    "serialize_precision",
    "session",
    "short_open_tag",
    "smtp",
    "smtp_port",
    "soap",
    "sqlite3",
    "sys_temp_dir",
    "syslog",
    "unserialize_callback_func",
    "unserialize_max_depth",
    "upload_max_filesize",
    "upload_tmp_dir",
    "url_rewriter",
    "user_dir",
    "user_ini",
    "variables_order",
    "windows",
    "xbithack",
    "xdebug",
    "xmlrpc_error_number",
    "xmlrpc_errors",
    "zend",
    "zend_extension",
    "zlib",
];

fn assign_key(l: &str) -> Option<&str> {
    let t = l.trim();
    if t.is_empty() || t.starts_with(';') || t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    if k.is_empty() || k.contains(' ') {
        return None;
    }
    Some(k)
}

fn known_key(k: &str) -> bool {
    KEYS.contains(&k)
        || k.starts_with("session.")
        || k.starts_with("opcache.")
        || k.starts_with("xdebug.")
        || k.starts_with("mysqli.")
        || k.starts_with("pdo_mysql.")
        || k.starts_with("pdo_pgsql.")
        || k.starts_with("mbstring.")
        || k.starts_with("intl.")
        || k.starts_with("soap.")
        || k.starts_with("curl.")
        || k.starts_with("openssl.")
        || k.starts_with("zlib.")
        || k.starts_with("sqlite3.")
        || k.starts_with("mysqlnd.")
        || k.starts_with("imap.")
        || k.starts_with("ldap.")
        || k.starts_with("mail.")
        || k.starts_with("assert.")
        || k.starts_with("date.")
        || k.starts_with("filter.")
        || k.starts_with("iconv.")
        || k.starts_with("phar.")
        || k.starts_with("ffi.")
        || k.starts_with("sodium.")
        || k.starts_with("exif.")
        || k.starts_with("gd.")
        || k.starts_with("xmlrpc_")
        || k.starts_with("syslog.")
        || k.starts_with("highlight.")
        || k.starts_with("url_rewriter.")
        || k.starts_with("arg_separator.")
        || k.starts_with("cgi.")
        || k.starts_with("cli_server.")
        || k.starts_with("browscap")
        || k.starts_with("zend.")
}

fn section(t: &str) -> Option<&str> {
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim())
}

/// Detect a `php.ini`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut keys = 0usize;
    let mut secs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if let Some(s) = section(tr) {
            if SECTIONS.contains(&s) {
                secs += 1;
            }
            continue;
        }
        if let Some(k) = assign_key(l) {
            if known_key(k) {
                keys += 1;
            }
        }
    }
    keys >= 3 || (secs >= 1 && keys >= 1)
}

impl Phpini {
    /// Count directives and sections. Returns `None` when the input does
    /// not look like a `php.ini`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            sections: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with(';') || tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(s) = section(tr) {
                if SECTIONS.contains(&s) {
                    c.sections += 1;
                }
                continue;
            }
            if let Some(k) = assign_key(l) {
                if known_key(k) {
                    c.settings += 1;
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
        let b = b"; php.ini\n[PHP]\nengine = On\nshort_open_tag = Off\nprecision = 14\noutput_buffering = 4096\nmemory_limit = 128M\nerror_reporting = E_ALL\ndisplay_errors = Off\nlog_errors = On\npost_max_size = 8M\nfile_uploads = On\nupload_max_filesize = 2M\nmax_file_uploads = 20\nallow_url_fopen = On\ndefault_socket_timeout = 60\n[CLI Server]\ncli_server.color = On\n[Date]\ndate.timezone = UTC\n[Session]\nsession.save_handler = files\nsession.gc_maxlifetime = 1440\n[opcache]\nopcache.enable=1\nopcache.memory_consumption=128\n[xdebug]\nxdebug.mode = debug\n";
        assert!(detect(b));
        let c = Phpini::parse(b).unwrap();
        assert!(c.settings >= 20);
        assert_eq!(c.sections, 6);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[main]\nplugin=1\n"));
        assert!(!detect(b"foo=1\nbar=2\n"));
        assert!(Phpini::parse(b"").is_none());
    }
}
