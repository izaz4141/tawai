use init_tracing_opentelemetry::{Guard, TracingConfig};
use tawai_core::utils::logger::{self, LogLevel};

const OTEL_ALIASES: &[(&str, &str)] = &[
    ("TAWAI_OTEL_SDK_DISABLED", "OTEL_SDK_DISABLED"),
    ("TAWAI_OTEL_SERVICE_NAME", "OTEL_SERVICE_NAME"),
    ("TAWAI_OTEL_TRACES_EXPORTER", "OTEL_TRACES_EXPORTER"),
    ("TAWAI_OTEL_TRACES_SAMPLER", "OTEL_TRACES_SAMPLER"),
    ("TAWAI_OTEL_TRACES_SAMPLER_ARG", "OTEL_TRACES_SAMPLER_ARG"),
    ("TAWAI_OTEL_PROPAGATORS", "OTEL_PROPAGATORS"),
    (
        "TAWAI_OTEL_EXPORTER_OTLP_ENDPOINT",
        "OTEL_EXPORTER_OTLP_ENDPOINT",
    ),
    (
        "TAWAI_OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
        "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
    ),
    (
        "TAWAI_OTEL_EXPORTER_OTLP_PROTOCOL",
        "OTEL_EXPORTER_OTLP_PROTOCOL",
    ),
    (
        "TAWAI_OTEL_EXPORTER_OTLP_TRACES_PROTOCOL",
        "OTEL_EXPORTER_OTLP_TRACES_PROTOCOL",
    ),
    (
        "TAWAI_OTEL_EXPORTER_OTLP_HEADERS",
        "OTEL_EXPORTER_OTLP_HEADERS",
    ),
    (
        "TAWAI_OTEL_EXPORTER_OTLP_TRACES_HEADERS",
        "OTEL_EXPORTER_OTLP_TRACES_HEADERS",
    ),
    (
        "TAWAI_OTEL_EXPORTER_OTLP_INSECURE",
        "OTEL_EXPORTER_OTLP_INSECURE",
    ),
    (
        "TAWAI_OTEL_EXPORTER_OTLP_CERTIFICATE",
        "OTEL_EXPORTER_OTLP_CERTIFICATE",
    ),
];

/// Maps `TAWAI_OTEL_*` variables onto the `OTEL_*` names read by the
/// OpenTelemetry SDK.
///
/// Must run from the synchronous entry point before the Tokio runtime starts
/// and before any other thread exists. An explicit `OTEL_*` value set by the
/// operator always takes precedence over its `TAWAI_OTEL_*` alias.
pub fn apply_env_aliases() {
    for &(tawai_name, otel_name) in OTEL_ALIASES {
        if std::env::var_os(otel_name).is_some() {
            continue;
        }

        let Ok(value) = std::env::var(tawai_name) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }

        // SAFETY: called from the synchronous entry point before the Tokio
        // runtime is created, so no other thread can read or write the
        // process environment concurrently.
        unsafe {
            std::env::set_var(otel_name, value);
        }
    }
}

fn build(level: LogLevel) -> Result<Guard, String> {
    let config = if cfg!(debug_assertions) {
        TracingConfig::development()
    } else {
        TracingConfig::production()
    };

    config
        .with_log_directives(level.as_directive())
        .init_subscriber()
        .map_err(|e| e.to_string())
}

pub fn init() -> Guard {
    let (level, invalid) = logger::log_level();

    let guard = match build(level) {
        Ok(guard) => guard,
        Err(error) => {
            let guard = build(LogLevel::Info).expect("tracing subscriber init");
            tracing::error!(
                %error,
                level = level.as_directive(),
                "tracing subscriber init failed; using info"
            );
            guard
        }
    };

    if let Some(value) = invalid {
        tracing::error!(
            value = %value,
            "invalid TAWAI_LOG_LEVEL; using {}",
            level.as_directive()
        );
    }

    guard
}
