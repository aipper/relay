use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const CACHE_VERSION: u32 = 1;
const DEFAULT_AUTO_RUNS: u32 = 5;
const DEFAULT_AUTO_TTL_SECS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRunMode {
    Tui,
    Structured,
}

impl ToolRunMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolRunMode::Tui => "tui",
            ToolRunMode::Structured => "structured",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "tui" | "pty" => Some(ToolRunMode::Tui),
            "structured" | "mcp" => Some(ToolRunMode::Structured),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CacheFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    tools: HashMap<String, ToolEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolEntry {
    #[serde(default)]
    pub mode: String, // "tui" | "structured"
    #[serde(default)]
    pub mcp_args: Option<Vec<String>>,
    #[serde(default)]
    pub probed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub runs_since_probe: u32,
    #[serde(default)]
    pub last_error: Option<String>,
}

impl ToolEntry {
    pub fn parsed_mode(&self) -> Option<ToolRunMode> {
        ToolRunMode::parse(&self.mode)
    }
}

pub struct ToolModeCache {
    path: std::path::PathBuf,
    file: CacheFile,
}

fn relay_home_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let home = std::path::PathBuf::from(home);
    Some(home.join(".relay"))
}

fn cache_path() -> Option<std::path::PathBuf> {
    Some(relay_home_dir()?.join("tool-mode-cache.json"))
}

fn env_u32(name: &str, default: u32) -> u32 {
    match std::env::var(name) {
        Ok(v) => v.trim().parse::<u32>().ok().unwrap_or(default),
        Err(_) => default,
    }
}

fn env_i64(name: &str, default: i64) -> i64 {
    match std::env::var(name) {
        Ok(v) => v.trim().parse::<i64>().ok().unwrap_or(default),
        Err(_) => default,
    }
}

pub fn auto_probe_runs_threshold() -> u32 {
    env_u32("RELAY_TOOL_MODE_AUTO_RUNS", DEFAULT_AUTO_RUNS)
}

pub fn auto_probe_ttl_secs() -> i64 {
    env_i64("RELAY_TOOL_MODE_AUTO_TTL_SECS", DEFAULT_AUTO_TTL_SECS)
}

impl ToolModeCache {
    pub fn load() -> anyhow::Result<Option<Self>> {
        let Some(path) = cache_path() else {
            return Ok(None);
        };

        let file = match std::fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str::<CacheFile>(&raw).unwrap_or_default(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => CacheFile::default(),
            Err(e) => return Err(e).with_context(|| format!("read tool mode cache: {}", path.display())),
        };

        Ok(Some(Self { path, file }))
    }

    pub fn get(&self, tool: &str) -> ToolEntry {
        self.file.tools.get(tool).cloned().unwrap_or_default()
    }

    pub fn set(&mut self, tool: &str, entry: ToolEntry) {
        self.file.tools.insert(tool.to_string(), entry);
    }

    pub fn touch_run(&mut self, tool: &str) {
        let mut e = self.get(tool);
        e.runs_since_probe = e.runs_since_probe.saturating_add(1);
        self.set(tool, e);
    }

    pub fn should_probe(&self, tool: &str, now: DateTime<Utc>) -> bool {
        let runs_threshold = auto_probe_runs_threshold();
        let ttl_secs = auto_probe_ttl_secs();
        let e = self.file.tools.get(tool);
        let Some(e) = e else {
            return true;
        };
        if e.runs_since_probe >= runs_threshold {
            return true;
        }
        let Some(ts) = e.probed_at else {
            return true;
        };
        let age = now.signed_duration_since(ts);
        age.num_seconds() >= ttl_secs
    }

    pub fn record_probe_result(
        &mut self,
        tool: &str,
        mode: ToolRunMode,
        mcp_args: Option<Vec<String>>,
        last_error: Option<String>,
        now: DateTime<Utc>,
    ) {
        let mut e = self.get(tool);
        e.mode = mode.as_str().to_string();
        e.mcp_args = mcp_args;
        e.probed_at = Some(now);
        e.runs_since_probe = 0;
        e.last_error = last_error;
        self.set(tool, e);
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create cache dir: {}", parent.display()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
            }
        }

        let mut out = self.file.clone();
        out.version = CACHE_VERSION;
        let json = serde_json::to_string_pretty(&out).context("encode tool mode cache json")?;

        let tmp = self
            .path
            .with_extension(format!("json.tmp.{}", uuid::Uuid::new_v4()));
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&tmp)
                .with_context(|| format!("write temp cache: {}", tmp.display()))?;
            f.write_all(json.as_bytes())
                .with_context(|| format!("write temp cache body: {}", tmp.display()))?;
            f.write_all(b"\n")
                .with_context(|| format!("write temp cache newline: {}", tmp.display()))?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }

        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("rename cache into place: {}", self.path.display()))?;
        Ok(())
    }
}
