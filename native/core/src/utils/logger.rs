use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

use time::{OffsetDateTime, macros::format_description};

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
    if tracing_enabled() {
        tracing::debug!(target: "tawai_core", message = %message);
        return;
    }

    let now = OffsetDateTime::now_local().expect("local time unavailable");
    let fmt = format_description!("[year repr:last_two]/[month]/[day]|[hour]:[minute]:[second]");
    let ts = now.format(&fmt).unwrap();

    println!("[DEBUG][{}] {}", ts, message);
}

pub fn info(message: &str) {
    if tracing_enabled() {
        tracing::info!(target: "tawai_core", message = %message);
        return;
    }

    let now = OffsetDateTime::now_local().expect("local time unavailable");
    let fmt = format_description!("[year repr:last_two]/[month]/[day]|[hour]:[minute]:[second]");
    let ts = now.format(&fmt).unwrap();

    println!("[INFO][{}] {}", ts, message);
}

pub fn warn(message: &str) {
    if tracing_enabled() {
        tracing::warn!(target: "tawai_core", message = %message);
        return;
    }

    let now = OffsetDateTime::now_local().expect("local time unavailable");
    let fmt = format_description!("[year repr:last_two]/[month]/[day]|[hour]:[minute]:[second]");
    let ts = now.format(&fmt).unwrap();

    println!("[WARN][{}] {}", ts, message);
}

pub fn error(message: &str) {
    if tracing_enabled() {
        tracing::error!(target: "tawai_core", message = %message);
        return;
    }

    let now = OffsetDateTime::now_local().expect("local time unavailable");
    let fmt = format_description!("[year repr:last_two]/[month]/[day]|[hour]:[minute]:[second]");
    let ts = now.format(&fmt).unwrap();

    eprintln!("[ERROR][{}] {}", ts, message);
}
