use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Config {
    pub server_base_url: String,
    pub host_id: String,
    pub host_token: String,
    pub local_unix_socket: String,
    pub redaction_extra_regex: Vec<String>,
    pub spool_db_path: String,
    pub log_path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct FileConfig {
    server_base_url: Option<String>,
    host_id: Option<String>,
    host_token: Option<String>,
    local_unix_socket: Option<String>,
    redaction_extra_regex: Option<Vec<String>>,
    spool_db_path: Option<String>,
    log_path: Option<String>,
}

fn normalize_server_base_url(raw: String) -> String {
    if raw.starts_with("http://") {
        raw.replacen("http://", "ws://", 1)
    } else if raw.starts_with("https://") {
        raw.replacen("https://", "wss://", 1)
    } else {
        raw
    }
}

fn xdg_config_home() -> Option<std::path::PathBuf> {
    if let Ok(v) = std::env::var("XDG_CONFIG_HOME") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Some(std::path::PathBuf::from(v));
        }
    }
    None
}

fn default_abrelay_hostd_config_path() -> Option<std::path::PathBuf> {
    let base = xdg_config_home().or_else(|| {
        let home = std::env::var("HOME").ok()?;
        let home = home.trim().to_string();
        if home.is_empty() {
            return None;
        }
        Some(std::path::PathBuf::from(home).join(".config"))
    })?;
    Some(base.join("abrelay").join("hostd.json"))
}

fn resolve_config_path() -> Option<std::path::PathBuf> {
    if let Ok(v) = std::env::var("ABRELAY_CONFIG") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Some(std::path::PathBuf::from(v));
        }
    }
    default_abrelay_hostd_config_path()
}

fn env_truthy(name: &str) -> bool {
    let v = match std::env::var(name) {
        Ok(v) => v,
        Err(_) => return false,
    };
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => true,
        _ => false,
    }
}

fn env_falsy(name: &str) -> bool {
    let v = match std::env::var(name) {
        Ok(v) => v,
        Err(_) => return false,
    };
    match v.trim().to_ascii_lowercase().as_str() {
        "0" | "false" | "no" | "n" | "off" => true,
        _ => false,
    }
}

fn default_file_config() -> FileConfig {
    let home = std::env::var("HOME").unwrap_or_default();
    let home = home.trim().to_string();
    let relay_home = if home.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(home).join(".relay"))
    };

    let local_unix_socket_default = relay_home
        .as_ref()
        .map(|p| p.join("relay-hostd.sock").to_string_lossy().to_string())
        .unwrap_or_else(|| "/tmp/relay-hostd.sock".to_string());
    let spool_db_path_default = relay_home
        .as_ref()
        .map(|p| p.join("hostd-spool.db").to_string_lossy().to_string())
        .unwrap_or_else(|| "data/hostd-spool.db".to_string());
    let log_path_default = relay_home
        .as_ref()
        .map(|p| p.join("hostd.log").to_string_lossy().to_string());

    let server_base_url = std::env::var("SERVER_BASE_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(normalize_server_base_url)
        .unwrap_or_else(|| "ws://127.0.0.1:8787".to_string());
    let host_id = std::env::var("HOST_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("host-{}", uuid::Uuid::new_v4()));
    let host_token = std::env::var("HOST_TOKEN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let local_unix_socket = std::env::var("LOCAL_UNIX_SOCKET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(local_unix_socket_default);
    let spool_db_path = std::env::var("SPOOL_DB_PATH")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(spool_db_path_default);
    let log_path = std::env::var("HOSTD_LOG_PATH")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(Some)
        .unwrap_or(log_path_default);

    FileConfig {
        server_base_url: Some(server_base_url),
        host_id: Some(host_id),
        host_token: Some(host_token),
        local_unix_socket: Some(local_unix_socket),
        redaction_extra_regex: Some(Vec::new()),
        spool_db_path: Some(spool_db_path),
        log_path,
    }
}

fn ensure_default_config_file(path: &std::path::Path) -> anyhow::Result<bool> {
    let should_init = if env_falsy("ABRELAY_INIT_CONFIG") {
        false
    } else if env_truthy("ABRELAY_INIT_CONFIG") {
        true
    } else {
        true
    };
    if !should_init {
        return Ok(false);
    }

    if path.exists() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create config dir: {}", parent.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
    }

    let cfg = default_file_config();
    let json = serde_json::to_string_pretty(&cfg).context("encode default config json")?;

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("create config file: {}", path.display()))?;
    use std::io::Write;
    f.write_all(json.as_bytes()).context("write config")?;
    f.write_all(b"\n").context("write newline")?;
    drop(f);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(true)
}

fn read_file_config(path: &std::path::Path) -> anyhow::Result<Option<FileConfig>> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("read config file: {}", path.display())),
    };
    let cfg = serde_json::from_str::<FileConfig>(&raw)
        .with_context(|| format!("parse config json: {}", path.display()))?;
    Ok(Some(cfg))
}

impl Config {
    pub fn from_env() -> Self {
        let server_base_url = std::env::var("SERVER_BASE_URL")
            .map(normalize_server_base_url)
            .unwrap_or_else(|_| "ws://127.0.0.1:8787".into());
        let host_id =
            std::env::var("HOST_ID").unwrap_or_else(|_| format!("host-{}", uuid::Uuid::new_v4()));
        let host_token = std::env::var("HOST_TOKEN").unwrap_or_else(|_| "dev-token".into());

        let local_unix_socket = std::env::var("LOCAL_UNIX_SOCKET").unwrap_or_else(|_| {
            // Default to a stable, user-local socket path so CLI shims can discover it without
            // requiring per-shell env vars.
            let home = std::env::var("HOME").unwrap_or_default();
            if home.trim().is_empty() {
                "/tmp/relay-hostd.sock".into()
            } else {
                format!("{home}/.relay/relay-hostd.sock")
            }
        });

        let spool_db_path = std::env::var("SPOOL_DB_PATH").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            if home.trim().is_empty() {
                "data/hostd-spool.db".into()
            } else {
                format!("{home}/.relay/hostd-spool.db")
            }
        });

        let log_path = std::env::var("HOSTD_LOG_PATH")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

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
            log_path,
        }
    }

    pub fn from_env_and_file() -> anyhow::Result<(Self, Option<std::path::PathBuf>)> {
        let path = resolve_config_path();
        if let Some(p) = path.as_ref() {
            let _ = ensure_default_config_file(p);
        }
        let file_cfg = match path.as_ref() {
            Some(p) => read_file_config(p)?,
            None => None,
        };
        let loaded_path = if file_cfg.is_some() {
            path.clone()
        } else {
            None
        };

        let server_base_url = std::env::var("SERVER_BASE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(normalize_server_base_url)
            .or_else(|| {
                file_cfg
                    .as_ref()
                    .and_then(|c| c.server_base_url.clone())
                    .map(normalize_server_base_url)
            })
            .unwrap_or_else(|| "ws://127.0.0.1:8787".into());

        let host_id = std::env::var("HOST_ID")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                file_cfg
                    .as_ref()
                    .and_then(|c| c.host_id.clone())
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or_else(|| format!("host-{}", uuid::Uuid::new_v4()));

        let host_token = std::env::var("HOST_TOKEN")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                file_cfg
                    .as_ref()
                    .and_then(|c| c.host_token.clone())
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or_else(|| "dev-token".into());

        let local_unix_socket = std::env::var("LOCAL_UNIX_SOCKET")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                file_cfg
                    .as_ref()
                    .and_then(|c| c.local_unix_socket.clone())
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_default();
                if home.trim().is_empty() {
                    "/tmp/relay-hostd.sock".into()
                } else {
                    format!("{home}/.relay/relay-hostd.sock")
                }
            });

        let spool_db_path = std::env::var("SPOOL_DB_PATH")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                file_cfg
                    .as_ref()
                    .and_then(|c| c.spool_db_path.clone())
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_default();
                if home.trim().is_empty() {
                    "data/hostd-spool.db".into()
                } else {
                    format!("{home}/.relay/hostd-spool.db")
                }
            });

        let log_path = std::env::var("HOSTD_LOG_PATH")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                file_cfg
                    .as_ref()
                    .and_then(|c| c.log_path.clone())
                    .filter(|s| !s.trim().is_empty())
            });

        let redaction_extra_regex = std::env::var("REDACTION_EXTRA_REGEX")
            .ok()
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .or_else(|| {
                file_cfg
                    .as_ref()
                    .and_then(|c| c.redaction_extra_regex.clone())
            })
            .unwrap_or_default();

        let cfg = Self {
            server_base_url,
            host_id,
            host_token,
            local_unix_socket,
            redaction_extra_regex,
            spool_db_path,
            log_path,
        };

        Ok((cfg, loaded_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_config_overrides_defaults() {
        let dir = std::env::temp_dir().join(format!("abrelay-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hostd.json");
        std::fs::write(
            &path,
            r#"{
  "server_base_url": "https://example.com",
  "host_id": "host-1",
  "host_token": "t-1",
  "local_unix_socket": "/tmp/x.sock",
  "spool_db_path": "/tmp/spool.db",
  "log_path": "/tmp/hostd.log",
  "redaction_extra_regex": ["foo", "bar"]
}"#,
        )
        .unwrap();

        let cfg = read_file_config(&path).unwrap().unwrap();
        assert_eq!(cfg.server_base_url.as_deref(), Some("https://example.com"));
        assert_eq!(cfg.host_id.as_deref(), Some("host-1"));
        assert_eq!(cfg.host_token.as_deref(), Some("t-1"));
    }
}
