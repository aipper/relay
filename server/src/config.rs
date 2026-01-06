use anyhow::{Context, anyhow};
use argon2::PasswordHash;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub jwt_secret: String,
    pub admin_username: String,
    pub admin_password_hash: String,
    pub store_raw_input: bool,
    pub redaction_extra_regex: Vec<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8787".into());
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/server.db".into());

        let jwt_secret =
            std::env::var("JWT_SECRET").context("missing JWT_SECRET (set a random long string)")?;

        let admin_username = std::env::var("ADMIN_USERNAME").context("missing ADMIN_USERNAME")?;
        let admin_password_hash =
            std::env::var("ADMIN_PASSWORD_HASH").context("missing ADMIN_PASSWORD_HASH")?;
        PasswordHash::new(&admin_password_hash).map_err(|e| {
            anyhow!(
                "invalid ADMIN_PASSWORD_HASH (expected an argon2 PHC string like $argon2id$...); run `relay-server --hash-password` or set ADMIN_PASSWORD for docker entrypoint; error={e}"
            )
        })?;

        let store_raw_input = std::env::var("STORE_RAW_INPUT")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if store_raw_input {
            return Err(anyhow!(
                "STORE_RAW_INPUT=true is not supported in MVP; keep it false for safety"
            ));
        }

        let redaction_extra_regex = std::env::var("REDACTION_EXTRA_REGEX")
            .ok()
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(Self {
            bind_addr,
            database_url,
            jwt_secret,
            admin_username,
            admin_password_hash,
            store_raw_input,
            redaction_extra_regex,
        })
    }
}
