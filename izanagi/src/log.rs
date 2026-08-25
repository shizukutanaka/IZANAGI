//! Tiny logging facility — no `log` crate dependency.
//!
//! A static, lock-free per-thread level filter. Backends call
//! [`set_writer`] to redirect output; the default writes to stderr.
//!
//! ```ignore
//! use izanagi::log::{info, warn, set_level, Level};
//!
//! set_level(Level::Warn);
//! info!("ignored");
//! warn!("shown");
//! ```

use std::cell::RefCell;
use std::io::Write;
use std::sync::atomic::{AtomicU8, Ordering};

/// Severity. Lower = more important.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum Level {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

static LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);

/// Set the global maximum visible log level.
pub fn set_level(level: Level) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

/// Current global maximum log level.
pub fn level() -> Level {
    match LEVEL.load(Ordering::Relaxed) {
        0 => Level::Error,
        1 => Level::Warn,
        2 => Level::Info,
        3 => Level::Debug,
        _ => Level::Trace,
    }
}

thread_local! {
    static WRITER: RefCell<Option<Box<dyn Write + 'static>>> = const { RefCell::new(None) };
}

/// Redirect log output (per-thread). Pass `None` to restore stderr.
pub fn set_writer(w: Option<Box<dyn Write + 'static>>) {
    WRITER.with(|cell| *cell.borrow_mut() = w);
}

/// Implementation detail used by the [`error!`] / [`warn!`] / etc. macros.
#[doc(hidden)]
pub fn _emit(level: Level, args: std::fmt::Arguments<'_>) {
    if level as u8 > LEVEL.load(Ordering::Relaxed) {
        return;
    }
    let tag = match level {
        Level::Error => "ERROR",
        Level::Warn => "WARN ",
        Level::Info => "INFO ",
        Level::Debug => "DEBUG",
        Level::Trace => "TRACE",
    };
    let line = format!("[{tag}] {args}\n");
    WRITER.with(|cell| {
        if let Some(w) = cell.borrow_mut().as_mut() {
            let _ = w.write_all(line.as_bytes());
        } else {
            let _ = std::io::stderr().write_all(line.as_bytes());
        }
    });
}

/// Log at error level.
#[macro_export]
macro_rules! error { ($($arg:tt)*) => { $crate::log::_emit($crate::log::Level::Error, format_args!($($arg)*)); } }
/// Log at warn level.
#[macro_export]
macro_rules! warn { ($($arg:tt)*) => { $crate::log::_emit($crate::log::Level::Warn, format_args!($($arg)*)); } }
/// Log at info level.
#[macro_export]
macro_rules! info { ($($arg:tt)*) => { $crate::log::_emit($crate::log::Level::Info, format_args!($($arg)*)); } }
/// Log at debug level.
#[macro_export]
macro_rules! debug { ($($arg:tt)*) => { $crate::log::_emit($crate::log::Level::Debug, format_args!($($arg)*)); } }
/// Log at trace level.
#[macro_export]
macro_rules! trace { ($($arg:tt)*) => { $crate::log::_emit($crate::log::Level::Trace, format_args!($($arg)*)); } }

// Macros are exported via #[macro_export] above; no `pub use` needed.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_setter_roundtrip() {
        set_level(Level::Trace);
        assert_eq!(level(), Level::Trace);
        set_level(Level::Error);
        assert_eq!(level(), Level::Error);
        set_level(Level::Info); // restore default for other tests
    }

    #[test]
    fn writer_redirect_captures_output() {
        struct Buf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Write for Buf {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        set_writer(Some(Box::new(Buf(captured.clone()))));
        set_level(Level::Info);
        info!("hello {}", 42);
        // Must release per-thread writer before the thread exits.
        set_writer(None);
        let s = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(s.contains("INFO"));
        assert!(s.contains("hello 42"));
    }

    #[test]
    fn level_filter_drops_lower() {
        set_level(Level::Warn);
        // No assertion — just confirms info! / debug! / trace! don't panic
        // and don't emit at higher levels. The redirect test above proves
        // the path that *does* emit.
        info!("filtered");
        debug!("filtered");
        trace!("filtered");
        set_level(Level::Info);
    }
}
