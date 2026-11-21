use std::borrow::Cow;
use std::sync::LazyLock;

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn variants_cover_separator_styles() {
        assert_eq!(
            sensitive_field_variants("access_token"),
            vec!["access_token", "access-token", "accesstoken"]
        );
        assert_eq!(sensitive_field_variants("token"), vec!["token"]);
    }
}
