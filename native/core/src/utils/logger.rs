use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

use time::{OffsetDateTime, macros::format_description};

use crate::utils::scrubber;

static TRACING_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
    Off,
}

impl LogLevel {
    pub const fn as_directive(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Off => "off",
        }
    }

    const fn tag(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Off => "OFF",
        }
    }
}

impl FromStr for LogLevel {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            "off" => Ok(Self::Off),
            _ => Err(()),
        }
    }
}

pub fn log_level() -> (LogLevel, Option<String>) {
    match std::env::var("TAWAI_LOG_LEVEL") {
        Ok(value) => {
            let value = value.trim();
            if value.is_empty() {
                return (LogLevel::default(), None);
            }

            match value.parse() {
                Ok(level) => (level, None),
                Err(()) => (LogLevel::default(), Some(value.to_string())),
            }
        }
        Err(_) => (LogLevel::default(), None),
    }
}

pub fn enable_tracing() {
    TRACING_ENABLED.store(true, Ordering::Relaxed);
}

fn tracing_enabled() -> bool {
    TRACING_ENABLED.load(Ordering::Relaxed)
}

pub fn debug(message: &str) {
    emit(LogLevel::Debug, message);
}

pub fn info(message: &str) {
    emit(LogLevel::Info, message);
}

pub fn warn(message: &str) {
    emit(LogLevel::Warn, message);
}

pub fn error(message: &str) {
    emit(LogLevel::Error, message);
}

/// Logs one HTTP request with `method`, `path`, `status`, and `duration_ms`.
pub fn access(method: &str, path: &str, status: u16, duration_ms: f64) {
    let method = scrubber::scrub(method);
    let path = scrubber::scrub(path);

    if tracing_enabled() {
        tracing::info!(
            target: "tawai::access",
            method = %method,
            path = %path,
            status = status,
            duration_ms = duration_ms,
        );
        return;
    }

    emit_console(
        LogLevel::Info,
        &format!("{method} {path} {status} {duration_ms:.2}ms"),
    );
}

fn emit(level: LogLevel, message: &str) {
    let message = scrubber::scrub(message);

    if tracing_enabled() {
        match level {
            LogLevel::Trace => tracing::trace!(target: "tawai_core", message = %message),
            LogLevel::Debug => tracing::debug!(target: "tawai_core", message = %message),
            LogLevel::Info => tracing::info!(target: "tawai_core", message = %message),
            LogLevel::Warn => tracing::warn!(target: "tawai_core", message = %message),
            LogLevel::Error => tracing::error!(target: "tawai_core", message = %message),
            LogLevel::Off => return,
        }
        return;
    }

    emit_console(level, message.as_ref());
}

fn emit_console(level: LogLevel, message: &str) {
    if matches!(level, LogLevel::Off) {
        return;
    }

    let now = OffsetDateTime::now_local().expect("local time unavailable");
    let fmt = format_description!("[year repr:last_two]/[month]/[day]|[hour]:[minute]:[second]");
    let ts = now.format(&fmt).unwrap();
    let line = fallback_line(level, &ts, message);

    if matches!(level, LogLevel::Error) {
        eprintln!("{line}");
    } else {
        println!("{line}");
    }
}

fn fallback_line(level: LogLevel, timestamp: &str, message: &str) -> String {
    format!("[{}][{}] {}", level.tag(), timestamp, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_line_redacts_secrets() {
        let message = scrubber::scrub("token=secret");

        assert_eq!(
            fallback_line(LogLevel::Info, "ts", message.as_ref()),
            "[INFO][ts] token=[REDACTED]",
        );
    }
}
