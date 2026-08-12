use regex::{Captures, Regex};
use serde_json::Value;

const REDACTED: &str = "<redacted>";
const REDACTED_VALUE: &str = "\"<redacted>\"";
const REDACTED_PRIVATE_KEY_BLOCK: &str = "<redacted-private-key-block>";

#[derive(Clone, Copy)]
enum RedactedBlock {
    Indented {
        base_indent: usize,
        closing: Option<char>,
    },
    Delimited(&'static str),
}

pub fn redact(content: &str) -> String {
    if let Ok(mut json) = serde_json::from_str::<Value>(content) {
        if redact_json_value(&mut json) {
            let serialized = if content.contains('\n') {
                serde_json::to_string_pretty(&json)
            } else {
                serde_json::to_string(&json)
            }
            .expect("redacted JSON remains serializable");

            return redact_text(&serialized);
        }
    }

    redact_text(content)
}

fn redact_json_value(value: &mut Value) -> bool {
    match value {
        Value::Object(values) => {
            let mut changed = false;
            for (key, value) in values {
                if is_sensitive_key(key) {
                    *value = Value::String(REDACTED.to_owned());
                    changed = true;
                } else {
                    changed |= redact_json_value(value);
                }
            }
            changed
        }
        Value::Array(values) => {
            let mut changed = false;
            for value in values {
                if redact_json_value(value) {
                    changed = true;
                }
            }
            changed
        }
        Value::String(value) => {
            let redacted = redact_text(value);
            if redacted == *value {
                false
            } else {
                *value = redacted;
                true
            }
        }
        _ => false,
    }
}

fn redact_text(content: &str) -> String {
    let complete_private_key = Regex::new(
        r"(?ims)-----BEGIN [^-\r\n]*PRIVATE KEY[^-\r\n]*-----.*?-----END [^-\r\n]*PRIVATE KEY[^-\r\n]*-----",
    )
    .expect("valid complete private-key regex");
    let unterminated_private_key =
        Regex::new(r"(?ims)-----BEGIN [^-\r\n]*PRIVATE KEY[^-\r\n]*-----.*\z")
            .expect("valid unterminated private-key regex");
    let assignment = Regex::new(r#"^(\s*[\"']?([A-Za-z0-9_. -]+?)[\"']?\s*[:=]\s*)(.*)$"#)
        .expect("valid assignment regex");
    let auth = Regex::new(r"(?i)\b(basic|bearer)([ \t]+)([A-Za-z0-9._~+/=-]+)")
        .expect("valid authorization regex");
    let cli_secret = Regex::new(
        r#"(?i)(--(?:api[-_]?key|access[-_]?token|auth[-_]?token|bearer[-_]?token|token|password|client[-_]?secret)(?:[ \t]*=[ \t]*|[ \t]+))(<[^>\r\n]+>|\"[^\"\r\n]*\"|'[^'\r\n]*'|[^\s'\"\\]+)"#,
    )
    .expect("valid CLI secret regex");
    let url_userinfo = Regex::new(r"(?i)\b([a-z][a-z0-9+.-]*://)([^/@\s:]+):([^/@\s]+)@")
        .expect("valid URL userinfo regex");
    let url_query_secret = Regex::new(
        r#"(?i)([?&](?:api[-_]?key|access[-_]?token|auth[-_]?token|bearer[-_]?token|refresh[-_]?token|id[-_]?token|token|client[-_]?secret|password|passwd|signature|x-amz-signature|x-goog-signature)=)([^&#\s'\"<>]+)"#,
    )
    .expect("valid URL query-secret regex");
    let bare_jwt = Regex::new(r"\beyJ[A-Za-z0-9_-]{5,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b")
        .expect("valid JWT regex");
    let known_token = Regex::new(concat!(
        r"\b(?:github_pat_[A-Za-z0-9_]{20,}|gh[pousr]_[A-Za-z0-9]{20,}|",
        r"glpat-[A-Za-z0-9_-]{20,}|sk-(?:proj-|svcacct-|ant-api\d+-)?[A-Za-z0-9_-]{20,}|",
        r"sk_live_[A-Za-z0-9]{16,}|rk_live_[A-Za-z0-9]{16,}|",
        r"xox[baprs]-[A-Za-z0-9-]{16,}|(?:AKIA|ASIA)[A-Z0-9]{16}|",
        r"AIza[A-Za-z0-9_-]{35})\b",
    ))
    .expect("valid known-token regex");
    let slack_webhook =
        Regex::new(r"(?i)https://hooks\.slack\.com/services/[A-Z0-9]+/[A-Z0-9]+/[A-Za-z0-9_-]+")
            .expect("valid Slack webhook regex");
    let discord_webhook = Regex::new(
        r"(?i)https://(?:canary\.|ptb\.)?discord(?:app)?\.com/api/webhooks/[0-9]+/[A-Za-z0-9._-]+",
    )
    .expect("valid Discord webhook regex");

    let without_complete_keys = complete_private_key
        .replace_all(content, REDACTED_PRIVATE_KEY_BLOCK)
        .into_owned();
    let without_private_keys = unterminated_private_key
        .replace_all(&without_complete_keys, REDACTED_PRIVATE_KEY_BLOCK)
        .into_owned();

    let mut result = String::with_capacity(without_private_keys.len());
    let mut redacted_block = None;

    for line in without_private_keys.split_inclusive('\n') {
        let (body, line_ending) = split_line_ending(line);

        if let Some(block) = redacted_block {
            match block {
                RedactedBlock::Delimited(delimiter) => {
                    if body.contains(delimiter) {
                        redacted_block = None;
                    }
                    result.push_str(line_ending);
                    continue;
                }
                RedactedBlock::Indented {
                    base_indent,
                    closing,
                } => {
                    let trimmed = body.trim_start();
                    if trimmed.is_empty() || indentation(body) > base_indent {
                        result.push_str(line_ending);
                        continue;
                    }
                    if closing.is_some_and(|closing| trimmed.starts_with(closing)) {
                        redacted_block = None;
                        result.push_str(line_ending);
                        continue;
                    }
                    redacted_block = None;
                }
            }
        }

        if let Some(captures) = assignment.captures(body) {
            let key = captures.get(2).expect("assignment key capture").as_str();
            if is_sensitive_key(key) {
                let prefix = captures.get(1).expect("assignment prefix capture").as_str();
                let value = captures.get(3).expect("assignment value capture").as_str();

                result.push_str(prefix);
                result.push_str(REDACTED_VALUE);
                result.push_str(line_ending);
                redacted_block = redacted_block_for(value, indentation(body));
                continue;
            }
        }

        let redacted_auth = auth.replace_all(body, |captures: &Captures<'_>| {
            let scheme = captures.get(1).expect("auth scheme capture").as_str();
            let whitespace = captures.get(2).expect("auth whitespace capture").as_str();
            let candidate = captures.get(3).expect("auth credential capture").as_str();

            if is_obvious_auth_prose(candidate) {
                captures
                    .get(0)
                    .expect("full auth capture")
                    .as_str()
                    .to_owned()
            } else {
                format!("{scheme}{whitespace}{REDACTED}")
            }
        });
        let redacted_cli = cli_secret.replace_all(&redacted_auth, |captures: &Captures<'_>| {
            let prefix = captures.get(1).expect("CLI flag prefix capture").as_str();
            let candidate = captures.get(2).expect("CLI value capture").as_str();
            if is_obvious_cli_placeholder(candidate, prefix.contains('=')) {
                captures
                    .get(0)
                    .expect("full CLI capture")
                    .as_str()
                    .to_owned()
            } else {
                format!("{prefix}{REDACTED}")
            }
        });
        let redacted_userinfo = url_userinfo.replace_all(&redacted_cli, "$1<redacted>@");
        let redacted_query = url_query_secret.replace_all(&redacted_userinfo, "$1<redacted>");
        let redacted_webhooks = slack_webhook
            .replace_all(&redacted_query, REDACTED)
            .into_owned();
        let redacted_webhooks = discord_webhook
            .replace_all(&redacted_webhooks, REDACTED)
            .into_owned();
        let redacted_tokens = known_token.replace_all(&redacted_webhooks, REDACTED);
        let redacted_tokens = bare_jwt.replace_all(&redacted_tokens, REDACTED);
        result.push_str(&redacted_tokens);
        result.push_str(line_ending);
    }

    result
}

fn split_line_ending(line: &str) -> (&str, &str) {
    if let Some(body) = line.strip_suffix("\r\n") {
        (body, "\r\n")
    } else if let Some(body) = line.strip_suffix('\n') {
        (body, "\n")
    } else {
        (line, "")
    }
}

fn indentation(line: &str) -> usize {
    line.chars()
        .take_while(|character| character.is_whitespace())
        .count()
}

fn redacted_block_for(value: &str, base_indent: usize) -> Option<RedactedBlock> {
    const TRIPLE_DOUBLE: &str = "\"\"\"";
    const TRIPLE_SINGLE: &str = "'''";

    let value = value.trim();
    if value.starts_with(TRIPLE_DOUBLE) && !value[TRIPLE_DOUBLE.len()..].contains(TRIPLE_DOUBLE) {
        return Some(RedactedBlock::Delimited(TRIPLE_DOUBLE));
    }
    if value.starts_with(TRIPLE_SINGLE) && !value[TRIPLE_SINGLE.len()..].contains(TRIPLE_SINGLE) {
        return Some(RedactedBlock::Delimited(TRIPLE_SINGLE));
    }
    if value.starts_with('{') && !value[1..].contains('}') {
        return Some(RedactedBlock::Indented {
            base_indent,
            closing: Some('}'),
        });
    }
    if value.starts_with('[') && !value[1..].contains(']') {
        return Some(RedactedBlock::Indented {
            base_indent,
            closing: Some(']'),
        });
    }
    if value.is_empty()
        || value.starts_with('#')
        || value.starts_with('|')
        || value.starts_with('>')
    {
        return Some(RedactedBlock::Indented {
            base_indent,
            closing: None,
        });
    }

    None
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = normalize_key(key);

    is_secret_key(&normalized)
        || is_password_key(&normalized)
        || is_token_key(&normalized)
        || normalized == "api_key"
        || normalized.contains("_api_key")
        || normalized.starts_with("api_key_")
        || normalized == "private_key"
        || normalized.contains("_private_key")
        || normalized.starts_with("private_key_")
        || normalized == "oauth"
        || normalized == "authorization"
        || normalized.ends_with("_authorization")
        || matches!(
            normalized.as_str(),
            "auth" | "auth_header" | "basic_auth" | "bearer_auth" | "proxy_auth"
        )
        || is_cookie_key(&normalized)
        || is_session_key(&normalized)
        || is_credential_key(&normalized)
        || normalized == "access_key"
        || normalized.contains("_access_key")
        || normalized.starts_with("access_key_")
        || normalized == "accesskey"
        || normalized.contains("_accesskey")
        || normalized.starts_with("accesskey_")
}

fn normalize_key(key: &str) -> String {
    let mut normalized = String::with_capacity(key.len());
    let mut previous_was_separator = false;
    let mut previous_was_lowercase_or_digit = false;
    let mut characters = key
        .trim_matches(|character: char| {
            character.is_whitespace() || character == '\'' || character == '"'
        })
        .chars()
        .peekable();

    while let Some(character) = characters.next() {
        if character.is_ascii_alphanumeric() {
            let starts_word = character.is_ascii_uppercase()
                && (previous_was_lowercase_or_digit
                    || characters
                        .peek()
                        .is_some_and(|next| next.is_ascii_lowercase()));
            if starts_word && !previous_was_separator && !normalized.is_empty() {
                normalized.push('_');
            }
            normalized.push(character.to_ascii_lowercase());
            previous_was_separator = false;
            previous_was_lowercase_or_digit =
                character.is_ascii_lowercase() || character.is_ascii_digit();
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
            previous_was_lowercase_or_digit = false;
        }
    }

    normalized.trim_matches('_').to_owned()
}

fn is_secret_key(key: &str) -> bool {
    key == "secret" || key.starts_with("secret_") || key.ends_with("_secret")
}

fn is_password_key(key: &str) -> bool {
    ["password", "passwd"].iter().any(|name| {
        key == *name || key.starts_with(&format!("{name}_")) || key.ends_with(&format!("_{name}"))
    })
}

fn is_token_key(key: &str) -> bool {
    key == "token"
        || key.ends_with("_token")
        || matches!(
            key,
            "access_token"
                | "refresh_token"
                | "auth_token"
                | "api_token"
                | "bearer_token"
                | "id_token"
                | "session_token"
                | "token_value"
                | "token_secret"
        )
}

fn is_cookie_key(key: &str) -> bool {
    key == "cookie"
        || key == "cookies"
        || key == "set_cookie"
        || key.ends_with("_cookie")
        || key.ends_with("_cookies")
        || matches!(key, "cookie_header" | "cookie_value")
}

fn is_session_key(key: &str) -> bool {
    matches!(
        key,
        "session"
            | "session_id"
            | "sessionid"
            | "session_key"
            | "session_token"
            | "session_secret"
            | "session_cookie"
    ) || key.ends_with("_session_id")
        || key.ends_with("_sessionid")
        || key.ends_with("_session_key")
}

fn is_credential_key(key: &str) -> bool {
    key == "credential"
        || key == "credentials"
        || key.ends_with("_credential")
        || key.ends_with("_credentials")
        || matches!(
            key,
            "credential_value" | "credential_blob" | "credential_json"
        )
}

fn is_obvious_auth_prose(candidate: &str) -> bool {
    matches!(
        candidate.to_ascii_lowercase().as_str(),
        "auth"
            | "authentication"
            | "authorization"
            | "credential"
            | "credentials"
            | "flow"
            | "flows"
            | "header"
            | "headers"
            | "scheme"
            | "schemes"
            | "token"
            | "tokens"
    )
}

fn is_obvious_cli_placeholder(candidate: &str, assigned_with_equals: bool) -> bool {
    let candidate = candidate.trim_matches(['\'', '"']);
    let lower = candidate.to_ascii_lowercase();
    let explicit_placeholder = (candidate.starts_with('<') && candidate.ends_with('>'))
        || (candidate.starts_with("${") && candidate.ends_with('}'))
        || candidate.starts_with('$')
        || matches!(
            candidate,
            "TOKEN"
                | "API_KEY"
                | "API-KEY"
                | "ACCESS_TOKEN"
                | "ACCESS-TOKEN"
                | "PASSWORD"
                | "SECRET"
                | "YOUR_TOKEN"
                | "YOUR_API_KEY"
        );
    if explicit_placeholder {
        return true;
    }

    !assigned_with_equals
        && matches!(
            lower.as_str(),
            "a" | "an"
                | "for"
                | "from"
                | "in"
                | "is"
                | "must"
                | "requires"
                | "should"
                | "the"
                | "to"
                | "value"
                | "with"
                | "your"
        )
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn redacts_common_secret_shapes() {
        let input =
            "api_key = \"sk-live-value\"\nAuthorization: Bearer abc.def.ghi\nname = \"safe\"";
        let result = redact(input);

        assert!(!result.contains("sk-live-value"));
        assert!(!result.contains("abc.def.ghi"));
        assert!(result.contains("name = \"safe\""));
    }

    #[test]
    fn redacts_basic_bearer_and_sensitive_headers() {
        let input = concat!(
            "Authorization: Basic dXNlcjpwYXNz\n",
            "proxy_authorization = \"Bearer opaque-value\"\n",
            "curl -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload.sig'\n",
            "Cookie: sid=private-session; theme=dark\n",
            "Set-Cookie: session_id=private; Secure\n",
        );
        let result = redact(input);

        for secret in [
            "dXNlcjpwYXNz",
            "opaque-value",
            "eyJhbGciOiJIUzI1NiJ9.payload.sig",
            "private-session",
            "session_id=private",
        ] {
            assert!(!result.contains(secret), "secret leaked: {secret}");
        }
        assert!(result.contains("curl -H 'Authorization: Bearer <redacted>'"));
    }

    #[test]
    fn redacts_session_credentials_and_access_keys() {
        let input = concat!(
            "session_id = abc-123\n",
            "credentials:\n",
            "  username: zane\n",
            "  password: hidden\n",
            "AWS_ACCESS_KEY_ID=AKIAEXAMPLE\n",
            "accessKeyId = CAMELCASEKEY\n",
            "clientSecret = CAMELCASESECRET\n",
            "safe_name = harness-lens\n",
        );
        let result = redact(input);

        for secret in [
            "abc-123",
            "zane",
            "hidden",
            "AKIAEXAMPLE",
            "CAMELCASEKEY",
            "CAMELCASESECRET",
        ] {
            assert!(!result.contains(secret), "secret leaked: {secret}");
        }
        assert!(result.contains("safe_name = harness-lens"));
    }

    #[test]
    fn redacts_complete_and_unterminated_private_key_blocks() {
        let complete = concat!(
            "before\n",
            "-----BEGIN RSA PRIVATE KEY-----\n",
            "complete-secret-material\n",
            "-----END RSA PRIVATE KEY-----\n",
            "after\n",
        );
        let result = redact(complete);
        assert!(!result.contains("complete-secret-material"));
        assert!(result.contains("before"));
        assert!(result.contains("after"));

        let unterminated = concat!(
            "safe-prefix\n",
            "-----BEGIN OPENSSH PRIVATE KEY-----\n",
            "unterminated-secret\n",
            "ambiguous trailing content\n",
        );
        let result = redact(unterminated);
        assert!(result.contains("safe-prefix"));
        assert!(!result.contains("unterminated-secret"));
        assert!(!result.contains("ambiguous trailing content"));
    }

    #[test]
    fn redacts_compact_json_credentials_as_a_whole() {
        let input = r#"{"credentials":{"username":"zane","password":"hidden"},"name":"safe"}"#;
        let result = redact(input);

        assert!(!result.contains("zane"));
        assert!(!result.contains("hidden"));
        assert!(result.contains(r#""name":"safe""#));
    }

    #[test]
    fn redacts_url_credentials_and_sensitive_query_parameters() {
        let input = concat!(
            "endpoint = https://alice:correct-horse@example.com/v1?token=url-secret&mode=safe\n",
            "database = postgres://db-user:db-password@db.example.com/app\n",
            "signed = https://files.example.com/object?x-amz-signature=deadbeef&version=1\n",
        );
        let result = redact(input);

        for secret in [
            "alice",
            "correct-horse",
            "db-user",
            "db-password",
            "url-secret",
            "deadbeef",
        ] {
            assert!(
                !result.contains(secret),
                "URL credential leaked: {secret}; result={result}"
            );
        }
        assert!(result.contains("mode=safe"));
        assert!(result.contains("version=1"));
        assert!(result.contains("https://<redacted>@example.com"));
    }

    #[test]
    fn redacts_cli_values_bare_tokens_and_webhooks() {
        let github_pat = "github_pat_0123456789abcdefghijklmnopqrstuv";
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijklmnop";
        let slack_webhook =
            "https://hooks.slack.com/services/T00000000/B00000000/abcdefghijklmnopqrstuv";
        let discord_webhook =
            "https://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz";
        let input = format!(
            "runner --api-key command-secret --mode safe\nrunner --token=inline-secret\npat {github_pat}\njwt {jwt}\nslack {slack_webhook}\ndiscord {discord_webhook}\n"
        );
        let result = redact(&input);

        for secret in [
            "command-secret",
            "inline-secret",
            github_pat,
            jwt,
            slack_webhook,
            discord_webhook,
        ] {
            assert!(!result.contains(secret), "inline secret leaked: {secret}");
        }
        assert!(result.contains("--mode safe"));
    }

    #[test]
    fn preserves_non_secret_configuration_and_public_material() {
        let input = concat!(
            "session_policy = \"all\"\n",
            "token_budget = 12000\n",
            "credential_provider = \"keychain\"\n",
            "cookie_domain = \"example.com\"\n",
            "public_key = \"public-material\"\n",
            "Bearer authentication is supported.\n",
            "Basic auth can be configured.\n",
            "Use --token for authentication.\n",
            "Example: runner --token TOKEN --api-key=<API_KEY>.\n",
            "policy_url = \"https://example.com/docs?token_budget=12000&mode=safe\"\n",
            "Never print github_pat_ values or Slack webhook URLs.\n",
            "-----BEGIN PUBLIC KEY-----\n",
            "ordinary-public-material\n",
            "-----END PUBLIC KEY-----\n",
        );

        assert_eq!(redact(input), input);
    }
}
