#[derive(Debug, Clone)]
pub struct Config {
    pub server_base_url: String,
    pub host_id: String,
    pub host_token: String,
    pub local_unix_socket: String,
    pub redaction_extra_regex: Vec<String>,
    pub spool_db_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        let server_base_url =
            std::env::var("SERVER_BASE_URL").unwrap_or_else(|_| "ws://127.0.0.1:8787".into());
        let host_id =
            std::env::var("HOST_ID").unwrap_or_else(|_| format!("host-{}", uuid::Uuid::new_v4()));
        let host_token = std::env::var("HOST_TOKEN").unwrap_or_else(|_| "dev-token".into());

        let local_unix_socket =
            std::env::var("LOCAL_UNIX_SOCKET").unwrap_or_else(|_| "/tmp/relay-hostd.sock".into());

        let spool_db_path =
            std::env::var("SPOOL_DB_PATH").unwrap_or_else(|_| "data/hostd-spool.db".into());

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

        Self {
            server_base_url,
            host_id,
            host_token,
            local_unix_socket,
            redaction_extra_regex,
            spool_db_path,
        }
    }
}
