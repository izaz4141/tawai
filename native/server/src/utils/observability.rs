use axum::{
    body::Body,
    extract::Request,
    http::uri::{PathAndQuery, Uri},
    middleware::Next,
    response::Response,
};
use init_tracing_opentelemetry::{Guard, TracingConfig};
use tawai_core::utils::logger::{self, LogLevel};

use crate::utils::redaction;

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

pub const EXCLUDED_PATHS: &[&str] = &["/api/openapi.json"];
pub const EXCLUDED_PREFIXES: &[&str] = &["/api/docs", "/api/tawai/system/status"];

#[derive(Clone)]
struct OriginalRequestUri(PathAndQuery);

/// Replaces sensitive query values before tracing so credentials such as
/// `?token=` never reach a span, then restores the original request URI before
/// routing and authentication so handlers still see the raw request.
pub async fn redact_request_uri(mut req: Request<Body>, next: Next) -> Response {
    let Some(query) = req.uri().query() else {
        return next.run(req).await;
    };

    let Some(sanitized) = redaction::redact_urlencoded(query) else {
        return next.run(req).await;
    };

    let Some(original) = req.uri().path_and_query().cloned() else {
        return next.run(req).await;
    };

    let rebuilt = format!("{}?{}", req.uri().path(), sanitized)
        .parse::<PathAndQuery>()
        .ok()
        .and_then(|path_and_query| {
            let mut parts = req.uri().clone().into_parts();
            parts.path_and_query = Some(path_and_query);
            Uri::from_parts(parts).ok()
        });

    if let Some(uri) = rebuilt {
        req.extensions_mut().insert(OriginalRequestUri(original));
        *req.uri_mut() = uri;
    }

    next.run(req).await
}

pub async fn restore_request_uri(mut req: Request<Body>, next: Next) -> Response {
    let Some(OriginalRequestUri(original)) = req.extensions().get::<OriginalRequestUri>().cloned()
    else {
        return next.run(req).await;
    };

    let mut parts = req.uri().clone().into_parts();
    parts.path_and_query = Some(original);

    if let Ok(uri) = Uri::from_parts(parts) {
        *req.uri_mut() = uri;
    }

    next.run(req).await
}

/// Logs one event per non-excluded request with exactly `method`, `path`,
/// `status`, and `duration_ms`.
///
/// `duration_ms` is the time to response headers: `next.run` resolves once the
/// response is produced, while a streaming body is polled by hyper afterward.
/// This keeps every segment unbuffered and does not measure full body transfer.
pub async fn access_log(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let started = std::time::Instant::now();

    let response = next.run(req).await;

    if !is_excluded(&path) {
        tracing::info!(
            target: "tawai::access",
            method = %method,
            path = %path,
            status = response.status().as_u16(),
            duration_ms = started.elapsed().as_secs_f64() * 1000.0,
        );
    }

    response
}

pub fn should_trace(path: &str) -> bool {
    !is_excluded(path)
}

pub fn is_excluded(path: &str) -> bool {
    EXCLUDED_PATHS.contains(&path)
        || EXCLUDED_PREFIXES
            .iter()
            .any(|prefix| path.starts_with(prefix))
}

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

const OTEL_ENV_TARGET: &str = "otel::setup::env";

fn log_directives(level: LogLevel) -> String {
    format!("{},{}=off", level.as_directive(), OTEL_ENV_TARGET)
}

fn preset() -> TracingConfig {
    if cfg!(debug_assertions) {
        TracingConfig::development()
    } else {
        TracingConfig::production()
    }
}

fn build(level: LogLevel) -> Result<Guard, String> {
    preset()
        .with_log_directives(log_directives(level))
        .init_subscriber()
        .map_err(|e| e.to_string())
}

fn build_console(level: LogLevel) -> Result<Guard, String> {
    preset()
        .with_otel(false)
        .with_log_directives(log_directives(level))
        .init_subscriber()
        .map_err(|e| e.to_string())
}

pub fn init() -> (Guard, bool) {
    let (level, invalid) = logger::log_level();

    let (guard, tracing_ready) = match build(level) {
        Ok(guard) => (guard, true),
        Err(error) => match build_console(LogLevel::Info) {
            Ok(guard) => {
                tracing::error!(
                    %error,
                    level = level.as_directive(),
                    "tracing subscriber init failed; using console-only info"
                );
                (guard, true)
            }
            Err(fallback_error) => {
                logger::error(&format!(
                    "tracing subscriber init failed: {error}; console fallback failed: {fallback_error}"
                ));
                (Guard::global(None), false)
            }
        },
    };

    if let Some(value) = invalid {
        tracing::error!(
            value = %value,
            "invalid TAWAI_LOG_LEVEL; using {}",
            level.as_directive()
        );
    }

    (guard, tracing_ready)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_directives_silences_otel_env_dump() {
        for level in [LogLevel::Debug, LogLevel::Trace, LogLevel::Info] {
            let directives = log_directives(level);
            assert!(directives.starts_with(level.as_directive()));
            assert!(directives.ends_with("otel::setup::env=off"));
        }
    }

    #[test]
    fn excluded_paths_are_not_traced() {
        for path in [
            "/api/openapi.json",
            "/api/docs",
            "/api/docs/swagger-ui.css",
            "/api/tawai/system/status",
        ] {
            assert!(is_excluded(path));
            assert!(!should_trace(path));
        }

        assert!(!is_excluded("/api/tawai/library/tracks"));
        assert!(should_trace("/api/tawai/library/tracks"));
    }
}
