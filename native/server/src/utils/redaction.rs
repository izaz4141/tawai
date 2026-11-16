use std::borrow::Cow;
use std::sync::LazyLock;

use axum::http::{HeaderMap, HeaderValue};
use serde_json::Value;

pub const REDACTED: &str = "REDACTED";

pub const SENSITIVE_FIELDS: &[&str] = &[
    "token",
    "access_token",
    "refresh_token",
    "id_token",
    "jwt",
    "api_key",
    "x_api_key",
    "encrypted_api_key",
    "decrypted_api_key",
    "auth",
    "auth_token",
    "authorization",
    "proxy_authorization",
    "password",
    "passwd",
    "new_password",
    "old_password",
    "current_password",
    "admin_password",
    "operator_password",
    "confirm_password",
    "password_confirmation",
    "secret",
    "client_secret",
    "secret_key",
    "private_key",
    "signature",
    "sig",
    "csrf",
    "csrf_token",
    "x_csrf_token",
    "session",
    "session_id",
    "session_token",
    "bearer_token",
    "cookie",
    "set_cookie",
    "tawai_jwt",
    "tawai_csrf",
    "master_key",
    "plain_key",
    "encrypted_key",
    "decrypted_key",
    "url",
];

fn normalize_field_name(name: &str) -> Cow<'_, str> {
    if name
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Cow::Borrowed(name);
    }

    Cow::Owned(
        name.chars()
            .filter(|c| !matches!(c, '_' | '-' | ' '))
            .map(|c| c.to_ascii_lowercase())
            .collect(),
    )
}

static NORMALIZED_SENSITIVE_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    SENSITIVE_FIELDS
        .iter()
        .map(|field| normalize_field_name(field).into_owned())
        .collect()
});

pub fn is_sensitive_field(name: &str) -> bool {
    let normalized = normalize_field_name(name);
    NORMALIZED_SENSITIVE_FIELDS
        .iter()
        .any(|field| field == normalized.as_ref())
}

pub fn sensitive_field_variants(field: &str) -> Vec<String> {
    let words: Vec<&str> = field
        .split(['_', '-', ' '])
        .filter(|word| !word.is_empty())
        .collect();

    if words.is_empty() {
        return vec![field.to_ascii_lowercase()];
    }

    let mut variants = vec![
        words.join("_").to_ascii_lowercase(),
        words.join("-").to_ascii_lowercase(),
        words.concat().to_ascii_lowercase(),
    ];
    variants.dedup();
    variants
}

pub fn redact_urlencoded(input: &str) -> Option<String> {
    let mut changed = false;
    let mut serializer = form_urlencoded::Serializer::new(String::new());

    for (key, value) in form_urlencoded::parse(input.as_bytes()) {
        if is_sensitive_field(&key) {
            changed = true;
            serializer.append_pair(&key, REDACTED);
        } else {
            serializer.append_pair(&key, &value);
        }
    }

    changed.then(|| serializer.finish())
}

pub fn redact_json(value: &mut Value) -> bool {
    match value {
        Value::Object(map) => {
            let mut changed = false;
            for (key, value) in map.iter_mut() {
                if is_sensitive_field(key) {
                    *value = Value::String(REDACTED.to_owned());
                    changed = true;
                } else {
                    changed |= redact_json(value);
                }
            }
            changed
        }
        Value::Array(items) => {
            let mut changed = false;
            for item in items {
                changed |= redact_json(item);
            }
            changed
        }
        _ => false,
    }
}

pub fn redact_json_text(input: &str) -> Option<String> {
    let mut value: Value = serde_json::from_str(input).ok()?;
    if !redact_json(&mut value) {
        return None;
    }
    serde_json::to_string(&value).ok()
}

pub fn redact_headers(headers: &mut HeaderMap) -> bool {
    let sensitive: Vec<_> = headers
        .keys()
        .filter(|name| is_sensitive_field(name.as_str()))
        .cloned()
        .collect();

    let changed = !sensitive.is_empty();
    for name in sensitive {
        headers.insert(name, HeaderValue::from_static(REDACTED));
    }
    changed
}

pub fn redact_field(name: &str, _value: &str) -> Option<&'static str> {
    is_sensitive_field(name).then_some(REDACTED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn matches_snake_camel_pascal_kebab_and_uppercase() {
        for name in [
            "access_token",
            "accessToken",
            "AccessToken",
            "ACCESS_TOKEN",
            "api_key",
            "apiKey",
            "ApiKey",
            "apikey",
            "api-key",
            "x_api_key",
            "xApiKey",
            "XApiKey",
            "X-API-KEY",
            "new_password",
            "newPassword",
            "NewPassword",
            "session_id",
            "sessionId",
            "SessionId",
            "master_key",
            "masterKey",
            "MasterKey",
            "tawai_jwt",
            "tawaiJwt",
            "TawaiJwt",
        ] {
            assert!(is_sensitive_field(name), "{name} should be sensitive");
        }
    }

    #[test]
    fn ignores_lookalike_field_names() {
        for name in ["tokenizer", "accessTokenCount", "urlPath", "apiKeys"] {
            assert!(!is_sensitive_field(name), "{name} should not be sensitive");
        }
    }

    #[test]
    fn redacts_sensitive_urlencoded_values() {
        assert_eq!(
            redact_urlencoded("token=secret&bitrate=128"),
            Some("token=REDACTED&bitrate=128".to_string())
        );
        assert_eq!(
            redact_urlencoded("bitrate=128&accessToken=secret&quality=high"),
            Some("bitrate=128&accessToken=REDACTED&quality=high".to_string())
        );
        assert_eq!(
            redact_urlencoded("url=https%3A%2F%2Fexample.com%2F%3Ftoken%3Dsecret"),
            Some("url=REDACTED".to_string())
        );
    }

    #[test]
    fn redacts_percent_encoded_and_case_variant_fields() {
        assert_eq!(
            redact_urlencoded("to%6Ben=secret"),
            Some("token=REDACTED".to_string())
        );
        assert_eq!(
            redact_urlencoded("TOKEN=first&token=second"),
            Some("TOKEN=REDACTED&token=REDACTED".to_string())
        );
    }

    #[test]
    fn leaves_safe_urlencoded_untouched() {
        assert_eq!(redact_urlencoded("bitrate=128&quality=high"), None);
        assert_eq!(redact_urlencoded("id=track-1&q=hello+world"), None);
    }

    #[test]
    fn redacts_json_recursively() {
        let mut value = serde_json::json!({
            "accessToken": "secret",
            "user": {
                "apiKey": "secret",
                "name": "listener"
            },
            "items": [
                {"password": "secret"},
                {"bitrate": 128}
            ]
        });

        assert!(redact_json(&mut value));
        assert_eq!(value["accessToken"], REDACTED);
        assert_eq!(value["user"]["apiKey"], REDACTED);
        assert_eq!(value["user"]["name"], "listener");
        assert_eq!(value["items"][0]["password"], REDACTED);
        assert_eq!(value["items"][1]["bitrate"].as_u64(), Some(128));
    }

    #[test]
    fn leaves_safe_json_untouched() {
        let mut value = serde_json::json!({
            "urlPath": "/api/tawai/library/tracks",
            "tokenizer": "wordpiece",
            "apiKeys": 3
        });

        assert!(!redact_json(&mut value));
    }

    #[test]
    fn redacts_json_text_and_reports_no_change() {
        assert_eq!(
            redact_json_text(r#"{"AccessToken":"secret","bitrate":128}"#),
            Some(r#"{"AccessToken":"REDACTED","bitrate":128}"#.to_string())
        );
        assert_eq!(redact_json_text(r#"{"bitrate":128}"#), None);
        assert_eq!(redact_json_text("not json"), None);
    }

    #[test]
    fn redacts_sensitive_headers_only() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer secret"));
        headers.insert("x-api-key", HeaderValue::from_static("secret"));
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        assert!(redact_headers(&mut headers));
        assert_eq!(headers["authorization"], REDACTED);
        assert_eq!(headers["x-api-key"], REDACTED);
        assert_eq!(headers["content-type"], "application/json");
    }

    #[test]
    fn redacts_sensitive_named_fields() {
        assert_eq!(redact_field("accessToken", "secret"), Some(REDACTED));
        assert_eq!(redact_field("bitrate", "128"), None);
    }

    #[test]
    fn variants_cover_separator_styles() {
        assert_eq!(
            sensitive_field_variants("access_token"),
            vec!["access_token", "access-token", "accesstoken"]
        );
        assert_eq!(sensitive_field_variants("token"), vec!["token"]);
    }
}
