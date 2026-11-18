use std::borrow::Cow;
use std::sync::OnceLock;

use regex::Regex;

use crate::utils::redaction::{SENSITIVE_FIELDS, sensitive_field_variants};

type Rule = (Regex, &'static str);

const MAX_EMBEDDED_JSON_DEPTH: usize = 8;

/// Replaces known credentials in one log message.
///
/// Returns a borrowed string when no rule matched.
pub fn scrub(message: &str) -> Cow<'_, str> {
    scrub_at_depth(message, 0)
}

fn scrub_at_depth(message: &str, depth: usize) -> Cow<'_, str> {
    let mut redacted = redact_embedded_json(message, depth);

    if let Some(next) = redact_containers(redacted.as_deref().unwrap_or(message)) {
        redacted = Some(next);
    }

    for (rule, replacement) in rules() {
        let input = redacted.as_deref().unwrap_or(message);

        if let Cow::Owned(next) = rule.replace_all(input, *replacement) {
            redacted = Some(next);
        }
    }

    redacted.map_or(Cow::Borrowed(message), Cow::Owned)
}

fn redact_embedded_json(message: &str, depth: usize) -> Option<String> {
    if depth >= MAX_EMBEDDED_JSON_DEPTH {
        return None;
    }

    let rule = embedded_json_rule();
    let mut redacted = String::new();
    let mut cursor = 0;
    let mut changed = false;

    for literal in rule.find_iter(message) {
        if literal.start() < cursor {
            continue;
        }

        let Ok(decoded) = serde_json::from_str::<String>(literal.as_str()) else {
            continue;
        };

        let trimmed = decoded.trim();

        if !(trimmed.starts_with('{') || trimmed.starts_with('['))
            || serde_json::from_str::<serde_json::Value>(trimmed).is_err()
        {
            continue;
        }

        let scrubbed = scrub_at_depth(&decoded, depth + 1);

        if scrubbed.as_ref() == decoded.as_str() {
            continue;
        }

        let Ok(encoded) = serde_json::to_string(scrubbed.as_ref()) else {
            continue;
        };

        redacted.push_str(&message[cursor..literal.start()]);
        redacted.push_str(&encoded);
        cursor = literal.end();
        changed = true;
    }

    if !changed {
        return None;
    }

    redacted.push_str(&message[cursor..]);
    Some(redacted)
}

fn embedded_json_rule() -> &'static Regex {
    static RULE: OnceLock<Regex> = OnceLock::new();

    RULE.get_or_init(|| Regex::new(r#""(?:[^"\\]|\\.)*""#).expect("embedded JSON rule"))
}

fn redact_containers(message: &str) -> Option<String> {
    let rule = container_rule()?;
    let mut redacted = String::new();
    let mut cursor = 0;
    let mut changed = false;

    for captures in rule.captures_iter(message) {
        let Some(opening) = captures.get(2) else {
            continue;
        };

        if opening.start() < cursor {
            continue;
        }

        let in_query = captures
            .get(1)
            .is_some_and(|prefix| is_query_prefix(prefix.as_str()));

        let (end, replacement) = match container_end(message, opening.start()) {
            Some(end) if ends_at_value_boundary(message, end) => (end, r#""[REDACTED]""#),
            Some(end) => (scalar_value_end(message, end, in_query), "[REDACTED]"),
            None if in_query => (query_value_end(message, opening.start()), "[REDACTED]"),
            None => (record_end(message, opening.start()), "[REDACTED]"),
        };

        redacted.push_str(&message[cursor..opening.start()]);
        redacted.push_str(replacement);
        cursor = end;
        changed = true;
    }

    if !changed {
        return None;
    }

    redacted.push_str(&message[cursor..]);
    Some(redacted)
}

fn container_rule() -> Option<&'static Regex> {
    static RULE: OnceLock<Option<Regex>> = OnceLock::new();

    RULE.get_or_init(|| {
        let keys = key_alternation();

        if keys.is_empty() {
            return None;
        }

        let boundary = r"(?:^|[{},;|&?(\[:\s=])";

        Regex::new(&format!(
            r#"(?i)({boundary}\s*(?:["'](?:{keys})["']|(?:{keys}))\s*[:=]\s*)([\[{{])|{boundary}\s*"(?:[^"\\]|\\.)*"|{boundary}\s*'(?:[^'\\]|\\.)*'"#
        ))
        .ok()
    })
    .as_ref()
}

fn container_end(message: &str, start: usize) -> Option<usize> {
    let mut stack = Vec::new();
    let mut quote = None;
    let mut escaped = false;

    for (index, byte) in message.as_bytes().iter().copied().enumerate().skip(start) {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == delimiter {
                quote = None;
            }

            continue;
        }

        match byte {
            b'"' | b'\'' => quote = Some(byte),
            b'[' | b'{' => stack.push(byte),
            b']' => {
                if stack.pop() != Some(b'[') {
                    return None;
                }

                if stack.is_empty() {
                    return Some(index + 1);
                }
            }
            b'}' => {
                if stack.pop() != Some(b'{') {
                    return None;
                }

                if stack.is_empty() {
                    return Some(index + 1);
                }
            }
            _ => {}
        }
    }

    None
}

fn record_end(message: &str, start: usize) -> usize {
    message.as_bytes()[start..]
        .iter()
        .position(|byte| matches!(byte, b'\r' | b'\n'))
        .map_or(message.len(), |offset| start + offset)
}

fn query_value_end(message: &str, start: usize) -> usize {
    message.as_bytes()[start..]
        .iter()
        .position(|byte| matches!(byte, b'&' | b'\r' | b'\n'))
        .map_or(message.len(), |offset| start + offset)
}

fn is_query_prefix(prefix: &str) -> bool {
    matches!(prefix.trim_start().as_bytes().first(), Some(b'?' | b'&'))
}

fn ends_at_value_boundary(message: &str, end: usize) -> bool {
    match message.as_bytes().get(end).copied() {
        None => true,
        Some(byte) => is_scalar_terminator(byte) || byte == b']',
    }
}

fn is_scalar_terminator(byte: u8) -> bool {
    byte.is_ascii_whitespace() || matches!(byte, b',' | b'}' | b'&' | b'"' | b'\'')
}

fn is_query_terminator(byte: u8) -> bool {
    byte.is_ascii_whitespace() || matches!(byte, b'&' | b'"' | b'\'')
}

fn scalar_value_end(message: &str, start: usize, in_query: bool) -> usize {
    let bytes = &message.as_bytes()[start..];
    let terminator = if in_query {
        is_query_terminator
    } else {
        is_scalar_terminator
    };
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 2;
            continue;
        }

        if terminator(bytes[index]) {
            return start + index;
        }

        index += 1;
    }

    message.len()
}

fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(build_rules)
}

fn build_rules() -> Vec<Rule> {
    let keys = key_alternation();
    let cookie_keys = field_alternation(&["cookie", "set_cookie"]);
    let mut rules = Vec::new();

    push_rule(
        &mut rules,
        &format!(
            r#"(?im)(?:(^[^"'\r\n]*?\b(?:{cookie_keys})\b\s*[:=]\s*)([^\r\n]*)|(\b(?:{cookie_keys})\b\s*[:=]\s*)((?:[^\s"\\']|\\[^\r\n])(?:[^"\\\r\n]|\\[^\r\n])*))"#
        ),
        r#"${1}${3}[REDACTED]"#,
    );

    if !keys.is_empty() {
        push_rule(
            &mut rules,
            &format!(r#"(?i)("?(?:{keys})"?\s*[:=]\s*)"(?:[^"\\]|\\.)*""#),
            r#"${1}"[REDACTED]""#,
        );
        push_rule(
            &mut rules,
            &format!(r#"(?i)(["']?(?:{keys})["']?\s*[:=]\s*)'(?:[^'\\]|\\.)*'"#),
            r#"${1}'[REDACTED]'"#,
        );
        push_rule(
            &mut rules,
            &format!(
                r#"(?i)("(?:{keys})"\s*:\s*)(-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)"#
            ),
            r#"${1}"[REDACTED]""#,
        );
        push_rule(
            &mut rules,
            &format!(r#"(?i)([?&]\s*(?:{keys})\s*[:=]\s*)([^&\s"]+)"#),
            r#"${1}[REDACTED]"#,
        );
        push_rule(
            &mut rules,
            &format!(
                r#"(?i)("?(?:{keys})"?\s*[:=]\s*)((?:(?:NDK|tawai-mk1):|bearer\s+|basic\s+)?)(?:[^"'\s,}}&\\]|\\.)+"#
            ),
            r#"${1}${2}[REDACTED]"#,
        );
    }

    push_rule(
        &mut rules,
        r"eyJ[A-Za-z0-9_-]{4,}\.[A-Za-z0-9_-]{4,}\.[A-Za-z0-9_-]+",
        "[REDACTED]",
    );
    push_rule(
        &mut rules,
        r"(?i)bearer\s+[A-Za-z0-9._~+/=-]+",
        "Bearer [REDACTED]",
    );
    push_rule(&mut rules, r"NDK:[0-9a-fA-F]+", "NDK:[REDACTED]");
    push_rule(
        &mut rules,
        r"tawai-mk1:[0-9a-fA-F]+",
        "tawai-mk1:[REDACTED]",
    );
    push_rule(&mut rules, r"\b[0-9a-fA-F]{64}\b", "[REDACTED]");

    rules
}

fn push_rule(rules: &mut Vec<Rule>, pattern: &str, replacement: &'static str) {
    if let Ok(regex) = Regex::new(pattern) {
        rules.push((regex, replacement));
    }
}

fn key_alternation() -> String {
    field_alternation(SENSITIVE_FIELDS)
}

fn field_alternation(fields: &[&str]) -> String {
    let mut variants: Vec<String> = fields
        .iter()
        .flat_map(|field| sensitive_field_variants(field))
        .map(|variant| regex::escape(&variant))
        .collect();

    variants.sort_unstable();
    variants.dedup();
    variants
        .sort_unstable_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));

    variants.join("|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn plain_json_line_is_borrowed() {
        let line = r#"{"level":"INFO","message":"server started"}"#;
        let scrubbed = scrub(line);

        assert!(matches!(scrubbed, Cow::Borrowed(_)));
        assert_eq!(scrubbed.as_ref(), line);
    }

    #[test]
    fn rule_table_is_complete() {
        assert_eq!(rules().len(), 11);
    }

    #[test]
    fn keyed_alternation_covers_every_sensitive_field_variant() {
        for field in SENSITIVE_FIELDS {
            for variant in sensitive_field_variants(field) {
                let line = format!("{variant}=secret");
                assert_eq!(
                    scrub(&line).as_ref(),
                    format!("{variant}=[REDACTED]"),
                    "field {field} variant {variant}",
                );
            }
        }
    }

    #[test]
    fn keyed_matching_covers_case_and_separator_styles() {
        for name in [
            "access_token",
            "access-token",
            "accesstoken",
            "accessToken",
            "AccessToken",
            "ACCESS_TOKEN",
            "x_api_key",
            "x-api-key",
            "xapikey",
            "xApiKey",
            "XApiKey",
            "X-API-KEY",
            "new_password",
            "newPassword",
            "NewPassword",
            "NEW_PASSWORD",
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
            let line = format!("{name}=secret");
            assert_eq!(
                scrub(&line).as_ref(),
                format!("{name}=[REDACTED]"),
                "name {name}",
            );
        }
    }

    #[test]
    fn redacts_json_string_values_and_keeps_valid_json() {
        let line = r#"{"access_token":"eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abc"}"#;
        let scrubbed = scrub(line);
        let value: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid JSON");

        assert_eq!(value["access_token"].as_str(), Some("[REDACTED]"));
        assert!(!scrubbed.contains("eyJhbGciOiJIUzI1NiJ9"));
    }

    #[test]
    fn redacts_json_numeric_values_and_keeps_valid_json() {
        for (line, expected) in [
            (r#"{"session_id":12345}"#, r#"{"session_id":"[REDACTED]"}"#),
            (r#"{"password":-3.5}"#, r#"{"password":"[REDACTED]"}"#),
            (
                r#"{"token":1e9,"other":"keep"}"#,
                r#"{"token":"[REDACTED]","other":"keep"}"#,
            ),
            (
                r#"{"api_key" : -0.25E-3 }"#,
                r#"{"api_key" : "[REDACTED]" }"#,
            ),
        ] {
            let scrubbed = scrub(line);
            let value: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid JSON");

            assert_eq!(scrubbed.as_ref(), expected, "line {line}");
            assert!(value.is_object(), "line {line}");
        }
    }

    #[test]
    fn keeps_plain_text_numeric_assignments_unquoted() {
        assert_eq!(scrub("session_id=12345").as_ref(), "session_id=[REDACTED]");
        assert_eq!(
            scrub("attempts=3&token=abc").as_ref(),
            "attempts=3&token=[REDACTED]",
        );
        assert_eq!(
            scrub("{\"session_id\":12345,\"attempts\":3}").as_ref(),
            r#"{"session_id":"[REDACTED]","attempts":3}"#,
        );
    }

    #[test]
    fn redacts_camel_and_pascal_json_keys() {
        for key in ["accessToken", "AccessToken", "ACCESS_TOKEN", "access-token"] {
            let line = format!(r#"{{"{key}":"secret"}}"#);
            let scrubbed = scrub(&line);
            let value: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid JSON");

            assert_eq!(value[key].as_str(), Some("[REDACTED]"), "key {key}");
        }
    }

    #[test]
    fn redacts_json_array_values_as_containers() {
        let line = r#"{"api_key":["sk-live-1","sk-live-2"]}"#;
        let scrubbed = scrub(line);
        let value: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid JSON");

        assert_eq!(value["api_key"].as_str(), Some("[REDACTED]"));
        assert!(!scrubbed.contains("sk-live"));
    }

    #[test]
    fn redacts_json_object_values_as_containers() {
        let line = r#"{"password":{"nested":{"token":"hunter2"},"list":["a","b"]}}"#;
        let scrubbed = scrub(line);
        let value: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid JSON");

        assert_eq!(value["password"].as_str(), Some("[REDACTED]"));
        assert!(!scrubbed.contains("hunter2"));
    }

    #[test]
    fn redacts_serialized_json_inside_json_string_values() {
        let line = r#"{"message":"{\"password\":\"hunter2\"}"}"#;
        let scrubbed = scrub(line);

        assert_eq!(
            scrubbed.as_ref(),
            r#"{"message":"{\"password\":\"[REDACTED]\"}"}"#,
        );

        let outer: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid outer JSON");
        let inner: Value = serde_json::from_str(outer["message"].as_str().expect("message string"))
            .expect("valid inner JSON");

        assert_eq!(inner["password"].as_str(), Some("[REDACTED]"));
        assert!(!scrubbed.contains("hunter2"));
    }

    #[test]
    fn redacts_serialized_json_inside_quoted_log_values() {
        let line = r#"payload="{\"token\":\"abc123\"}""#;

        assert_eq!(
            scrub(line).as_ref(),
            r#"payload="{\"token\":\"[REDACTED]\"}""#,
        );
    }

    #[test]
    fn preserves_escaped_siblings_while_redacting_serialized_json() {
        let line = r#"{"message":"{\"password\":\"hunter2\",\"note\":\"a\\\"b\"}"}"#;
        let scrubbed = scrub(line);

        assert_eq!(
            scrubbed.as_ref(),
            r#"{"message":"{\"password\":\"[REDACTED]\",\"note\":\"a\\\"b\"}"}"#,
        );

        let outer: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid outer JSON");
        let inner: Value = serde_json::from_str(outer["message"].as_str().expect("message string"))
            .expect("valid inner JSON");

        assert_eq!(inner["password"].as_str(), Some("[REDACTED]"));
        assert_eq!(inner["note"].as_str(), Some("a\"b"));
    }

    #[test]
    fn redacts_containers_inside_serialized_json_string_values() {
        let line = r#"{"message":"{\"api_key\":[\"sk-live-1\",\"sk-live-2\"]}"}"#;
        let scrubbed = scrub(line);

        assert_eq!(
            scrubbed.as_ref(),
            r#"{"message":"{\"api_key\":\"[REDACTED]\"}"}"#,
        );

        let outer: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid outer JSON");
        let inner: Value = serde_json::from_str(outer["message"].as_str().expect("message string"))
            .expect("valid inner JSON");

        assert_eq!(inner["api_key"].as_str(), Some("[REDACTED]"));
        assert!(!scrubbed.contains("sk-live"));
    }

    #[test]
    fn redacts_nested_serialized_json_string_values() {
        let line = r#"{"message":"{\"payload\":\"{\\\"password\\\":\\\"hunter2\\\"}\"}"}"#;
        let scrubbed = scrub(line);
        let outer: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid outer JSON");
        let middle: Value =
            serde_json::from_str(outer["message"].as_str().expect("message string"))
                .expect("valid middle JSON");
        let inner: Value =
            serde_json::from_str(middle["payload"].as_str().expect("payload string"))
                .expect("valid inner JSON");

        assert_eq!(inner["password"].as_str(), Some("[REDACTED]"));
        assert!(!scrubbed.contains("hunter2"));
    }

    #[test]
    fn leaves_quoted_non_json_values_untouched() {
        for line in [
            r#"{"message":"server started"}"#,
            r#"{"message":"[not json]"}"#,
            r#"{"message":"123"}"#,
            r#"{"message":"{\"note\":\"keep\"}"}"#,
        ] {
            assert!(matches!(scrub(line), Cow::Borrowed(_)), "line {line}");
        }
    }

    #[test]
    fn container_scan_ignores_brackets_inside_strings() {
        let line = r#"{"master_key":["a]b","c{d","e\"}f"]} trailing"#;

        assert_eq!(
            scrub(line).as_ref(),
            r#"{"master_key":"[REDACTED]"} trailing"#,
        );
    }

    #[test]
    fn container_scan_ignores_field_like_text_inside_quoted_values() {
        for line in [
            r#"{"message":"password=[hunter2]"}"#,
            r#"{"message": "password=[hunter2]"}"#,
            r#"{"message":"note api_key={secret}"}"#,
            r#"{"message": "note api_key={secret}"}"#,
            r#"{"message":"he said \"password=[hunter2]\""}"#,
        ] {
            let scrubbed = scrub(line);
            let value: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid JSON");

            assert!(value["message"].is_string(), "line {line}");
            assert!(!scrubbed.contains("hunter2"), "line {line}");
            assert!(!scrubbed.contains("secret"), "line {line}");
        }

        assert_eq!(
            scrub("{'message': 'password=[hunter2]'}").as_ref(),
            "{'message': 'password=[REDACTED]'}",
        );
    }

    #[test]
    fn quoted_container_like_passwords_are_fully_redacted() {
        for line in [
            r#"{"password":"[hunter2]"}"#,
            r#"{"password":"note api_key=[hunter2]"}"#,
            r#"{"password": "note api_key=[hunter2]"}"#,
        ] {
            let scrubbed = scrub(line);
            let value: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid JSON");

            assert_eq!(
                value["password"].as_str(),
                Some("[REDACTED]"),
                "line {line}"
            );
            assert!(!scrubbed.contains("hunter2"), "line {line}");
        }

        assert_eq!(
            scrub(r#"password="[hunter2]""#).as_ref(),
            r#"password="[REDACTED]""#,
        );
        assert_eq!(
            scrub("password='[hunter2]'").as_ref(),
            "password='[REDACTED]'",
        );
    }

    #[test]
    fn redacts_multiple_containers_in_one_line() {
        let line = r#"request {"api_key":["a"]} response {"session_id":{"id":"b"}}"#;

        assert_eq!(
            scrub(line).as_ref(),
            r#"request {"api_key":"[REDACTED]"} response {"session_id":"[REDACTED]"}"#,
        );
    }

    #[test]
    fn containers_without_value_boundaries_fall_back_to_scalar_redaction() {
        for (line, expected) in [
            ("password=[hunter2]tail", "password=[REDACTED]"),
            ("api_key={secret}tail", "api_key=[REDACTED]"),
            (r#"password={"a":"hunter2"}tail"#, "password=[REDACTED]"),
            (r#"password=["a","hunter2"]tail"#, "password=[REDACTED]"),
        ] {
            assert_eq!(scrub(line).as_ref(), expected, "line {line}");
        }
    }

    #[test]
    fn unterminated_containers_are_redacted_to_the_record_boundary() {
        for (line, expected) in [
            ("password=[hunter2", "password=[REDACTED]"),
            (r#"password=["a","hunter2""#, "password=[REDACTED]"),
            (r#"password={"a":"hunter2""#, "password=[REDACTED]"),
            (
                r#"{"password":{"nested":"hunter2""#,
                r#"{"password":[REDACTED]"#,
            ),
            (
                "password=[hunter2\r\nother=keep",
                "password=[REDACTED]\r\nother=keep",
            ),
            (
                "password=[hunter2\nother=keep",
                "password=[REDACTED]\nother=keep",
            ),
            (
                r#"{"password":{"a":"hunter2"},"other":"keep"}"#,
                r#"{"password":"[REDACTED]","other":"keep"}"#,
            ),
        ] {
            let scrubbed = scrub(line);
            assert_eq!(scrubbed.as_ref(), expected, "line {line}");
            assert!(!scrubbed.contains("hunter2"), "line {line}");
        }
    }

    #[test]
    fn scalar_suffix_scan_consumes_escaped_delimiters() {
        for (line, expected) in [
            (r"password=[hunter2]\,tail", "password=[REDACTED]"),
            (r"password=[hunter2]\ tail", "password=[REDACTED]"),
            (r#"password=[hunter2]\"tail"#, "password=[REDACTED]"),
        ] {
            assert_eq!(scrub(line).as_ref(), expected, "line {line}");
        }
    }

    #[test]
    fn scalar_suffix_scan_stops_at_unescaped_commas() {
        assert_eq!(
            scrub("password=[hunter2]tail,other=keep").as_ref(),
            "password=[REDACTED],other=keep",
        );
    }

    #[test]
    fn query_values_treat_commas_as_part_of_the_value() {
        for (line, expected) in [
            (
                "GET /api/tawai?api_key=abc,def&bitrate=128",
                "GET /api/tawai?api_key=[REDACTED]&bitrate=128",
            ),
            (
                "GET /api/tawai?a=1&api_key=abc,def&bitrate=128",
                "GET /api/tawai?a=1&api_key=[REDACTED]&bitrate=128",
            ),
            (
                "GET /api/tawai?api_key=[abc,def]&bitrate=128",
                r#"GET /api/tawai?api_key="[REDACTED]"&bitrate=128"#,
            ),
            (
                "GET /api/tawai?api_key=[abc,def&bitrate=128",
                "GET /api/tawai?api_key=[REDACTED]&bitrate=128",
            ),
        ] {
            let scrubbed = scrub(line);
            assert_eq!(scrubbed.as_ref(), expected, "line {line}");
        }

        assert_eq!(
            scrub("api_key=abc,user=1").as_ref(),
            "api_key=[REDACTED],user=1",
        );
    }

    #[test]
    fn containers_still_redact_at_value_boundaries() {
        let line = r#"{"api_key":["sk-live-1","sk-live-2"],"other":"keep"}"#;
        let scrubbed = scrub(line);
        let value: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid JSON");

        assert_eq!(value["api_key"].as_str(), Some("[REDACTED]"));
        assert_eq!(value["other"].as_str(), Some("keep"));
        assert!(!scrubbed.contains("sk-live"));

        assert_eq!(
            scrub("token=[abc]&user=1").as_ref(),
            r#"token="[REDACTED]"&user=1"#,
        );
        assert_eq!(
            scrub(r#"x=[api_key=["sk-live"]]"#).as_ref(),
            r#"x=[api_key="[REDACTED]"]"#,
        );
    }

    #[test]
    fn redacts_containers_after_query_question_mark() {
        assert_eq!(
            scrub("GET /api/tawai?api_key=[sk-live]&user=1").as_ref(),
            r#"GET /api/tawai?api_key="[REDACTED]"&user=1"#,
        );
        assert_eq!(
            scrub(r#"GET /api/tawai?password={"a":"b&c"}&user=1"#).as_ref(),
            r#"GET /api/tawai?password="[REDACTED]"&user=1"#,
        );
        assert_eq!(
            scrub("GET /api/tawai? token=[abc]&user=1").as_ref(),
            r#"GET /api/tawai? token="[REDACTED]"&user=1"#,
        );
    }

    #[test]
    fn redacts_single_quoted_values() {
        assert_eq!(scrub("api_key='s3cr3t'").as_ref(), "api_key='[REDACTED]'",);
        assert_eq!(
            scrub("password = 'hunter2'").as_ref(),
            "password = '[REDACTED]'",
        );
        assert_eq!(
            scrub("{'session_id': 'abc123'}").as_ref(),
            "{'session_id': '[REDACTED]'}",
        );
    }

    #[test]
    fn redacts_single_quoted_values_with_escapes() {
        let line = r"password='it\'s a secret'";
        let scrubbed = scrub(line);

        assert_eq!(scrubbed.as_ref(), r"password='[REDACTED]'");
        assert!(!scrubbed.contains("a secret"));
    }

    #[test]
    fn preserves_double_quoted_and_bare_redaction_for_mixed_lines() {
        assert_eq!(
            scrub(r#"api_key='one' password="two" token=three"#).as_ref(),
            r#"api_key='[REDACTED]' password="[REDACTED]" token=[REDACTED]"#,
        );
    }

    #[test]
    fn redacts_entire_cookie_header_value() {
        assert_eq!(
            scrub("Cookie: sid=abc123; theme=dark; lang=en").as_ref(),
            "Cookie: [REDACTED]",
        );
        assert_eq!(
            scrub("Set-Cookie: tawai_jwt=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abc; Path=/; HttpOnly; SameSite=Lax").as_ref(),
            "Set-Cookie: [REDACTED]",
        );
        assert_eq!(scrub("cookie=a=1;b=2").as_ref(), "cookie=[REDACTED]");
        assert_eq!(
            scrub(r#"Set-Cookie: session="a b; c"; Path=/; HttpOnly; SameSite=Lax"#).as_ref(),
            "Set-Cookie: [REDACTED]",
        );
        assert_eq!(
            scrub(r#"Cookie: sid="a b"; theme=dark"#).as_ref(),
            "Cookie: [REDACTED]",
        );
    }

    #[test]
    fn cookie_redaction_stops_at_header_boundary() {
        assert_eq!(
            scrub("Cookie: a=1; b=2\r\nUser-Agent: tawai").as_ref(),
            "Cookie: [REDACTED]\r\nUser-Agent: tawai",
        );
        assert_eq!(
            scrub(r#"{"message":"Cookie: a=1; b=2"}"#).as_ref(),
            r#"{"message":"Cookie: [REDACTED]"}"#,
        );
        assert_eq!(
            scrub(r#"{"message":"Cookie: \"a=1; b=2\"; note=\"x\""}"#).as_ref(),
            r#"{"message":"Cookie: [REDACTED]"}"#,
        );
        assert_eq!(
            scrub(r#"{"message":"Set-Cookie: sid=\"a b\"; HttpOnly"}"#).as_ref(),
            r#"{"message":"Set-Cookie: [REDACTED]"}"#,
        );
    }

    #[test]
    fn cookie_redaction_preserves_quoted_values() {
        let line = r#"{"cookie":"a=1; b=2","other":"keep"}"#;
        let scrubbed = scrub(line);
        let value: Value = serde_json::from_str(scrubbed.as_ref()).expect("valid JSON");

        assert_eq!(value["cookie"].as_str(), Some("[REDACTED]"));
        assert_eq!(value["other"].as_str(), Some("keep"));
        assert_eq!(scrub("cookie: 'a=1; b=2'").as_ref(), "cookie: [REDACTED]",);
    }

    #[test]
    fn redacts_query_token_values() {
        assert_eq!(
            scrub("GET /api/tawai/playback/stream?token=abc.def.ghi").as_ref(),
            "GET /api/tawai/playback/stream?token=[REDACTED]",
        );
        assert_eq!(
            scrub("GET /api/tawai/library/tracks?a=1&token=abc").as_ref(),
            "GET /api/tawai/library/tracks?a=1&token=[REDACTED]",
        );
    }

    #[test]
    fn redacts_bearer_and_basic_authorization_values() {
        assert_eq!(
            scrub("authorization: Bearer abc123").as_ref(),
            "authorization: Bearer [REDACTED]",
        );
        assert_eq!(
            scrub("authorization: Basic abc123").as_ref(),
            "authorization: Basic [REDACTED]",
        );
        assert_eq!(scrub("Bearer abc123").as_ref(), "Bearer [REDACTED]");
    }

    #[test]
    fn redacts_keyed_ciphertext_prefixes() {
        assert_eq!(
            scrub("api_key=NDK:00ff11").as_ref(),
            "api_key=NDK:[REDACTED]",
        );
        assert_eq!(scrub("NDK:00ff11").as_ref(), "NDK:[REDACTED]");
        assert_eq!(
            scrub("master_key=tawai-mk1:00ff11").as_ref(),
            "master_key=tawai-mk1:[REDACTED]",
        );
        assert_eq!(scrub("tawai-mk1:00ff11").as_ref(), "tawai-mk1:[REDACTED]",);
    }

    #[test]
    fn redacts_64_hex_but_preserves_32_hex() {
        let hex64 = "0123456789abcdef".repeat(4);
        let hex32 = "0123456789abcdef".repeat(2);

        assert_eq!(scrub(&hex64).as_ref(), "[REDACTED]");

        let scrubbed = scrub(&hex32);
        assert!(matches!(scrubbed, Cow::Borrowed(_)));
        assert_eq!(scrubbed.as_ref(), hex32.as_str());
    }

    #[test]
    fn redacts_a_realistic_jwt_seed() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJsaXN0ZW5lciIsInNlZWQiOiJ0YXdhaSJ9.c2lnbmF0dXJl";

        assert_eq!(scrub(jwt).as_ref(), "[REDACTED]");
        assert!(!scrub(jwt).contains("eyJ"));
    }

    #[test]
    fn redacts_multiple_secrets_in_one_line() {
        let hex64 = "0123456789abcdef".repeat(4);
        let line = format!("token=first password=second key={hex64}");
        let scrubbed = scrub(&line);

        assert!(!scrubbed.contains("first"));
        assert!(!scrubbed.contains("second"));
        assert!(!scrubbed.contains(&hex64));
        assert!(scrubbed.contains("token=[REDACTED]"));
        assert!(scrubbed.contains("password=[REDACTED]"));
    }
}
