use regex::Regex;
use sha2::{Digest, Sha256};

pub struct Redactor {
    kv_regex: Regex,
    bearer_regex: Regex,
    long_token_regex: Regex,
    extra: Vec<Regex>,
}

pub struct RedactionResult {
    pub text_redacted: String,
    pub text_sha256: String,
}

impl Redactor {
    pub fn new(extra_patterns: &[String]) -> anyhow::Result<Self> {
        let kv_regex = Regex::new(
            r#"(?ix)
            \b(api[_-]?key|token|password|secret|authorization)\b
            \s*[:=]\s*
            ([^\s'"]+|"[^"]*"|'[^']*')
        "#,
        )?;

        let bearer_regex = Regex::new(r#"(?i)\bAuthorization\s*:\s*Bearer\s+([^\s]+)"#)?;
        let long_token_regex = Regex::new(r#"[A-Za-z0-9+/=_-]{32,}"#)?;

        let mut extra = Vec::new();
        for pat in extra_patterns {
            extra.push(Regex::new(pat)?);
        }

        Ok(Self {
            kv_regex,
            bearer_regex,
            long_token_regex,
            extra,
        })
    }

    pub fn redact(&self, raw: &str) -> RedactionResult {
        let mut redacted = raw.to_string();

        redacted = self
            .kv_regex
            .replace_all(&redacted, "$1=***REDACTED***")
            .to_string();
        redacted = self
            .bearer_regex
            .replace_all(&redacted, "Authorization: Bearer ***REDACTED***")
            .to_string();

        for extra in &self.extra {
            redacted = extra.replace_all(&redacted, "***REDACTED***").to_string();
        }

        redacted = self
            .long_token_regex
            .replace_all(&redacted, "***REDACTED***")
            .to_string();

        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        let text_sha256 = format!("{:x}", hasher.finalize());

        RedactionResult {
            text_redacted: redacted,
            text_sha256,
        }
    }
}
