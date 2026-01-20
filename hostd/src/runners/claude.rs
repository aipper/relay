use super::{
    Runner, RunnerSpec, base_prompt_regex, command_from_cmdline, command_from_shell,
    looks_like_shell, resolve_tool_bin, swap_leading_token, validate_bin_exists,
};

fn resolve_relay_mcp_command() -> String {
    if let Ok(v) = std::env::var("RELAY_MCP_COMMAND") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return v;
        }
    }

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return "relay".to_string(),
    };
    let Some(dir) = exe.parent() else {
        return "relay".to_string();
    };

    #[cfg(windows)]
    let candidate = dir.join("relay.exe");
    #[cfg(not(windows))]
    let candidate = dir.join("relay");

    if candidate.is_file() {
        return candidate.to_string_lossy().to_string();
    }
    "relay".to_string()
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

#[derive(Clone, Copy, Debug)]
struct ClaudeMcpSupport {
    mcp_config: bool,
}

fn detect_claude_mcp_support(bin: &str) -> ClaudeMcpSupport {
    use std::process::{Command, Stdio};

    let output = Command::new(bin)
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let Ok(output) = output else {
        return ClaudeMcpSupport {
            mcp_config: false,
        };
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let help = format!("{stdout}\n{stderr}");

    ClaudeMcpSupport {
        mcp_config: help.contains("--mcp-config"),
    }
}

fn detect_claude_mcp_support_cached(bin: &str) -> ClaudeMcpSupport {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<HashMap<String, ClaudeMcpSupport>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(map) = cache.lock() {
        if let Some(v) = map.get(bin) {
            return *v;
        }
    }

    let detected = detect_claude_mcp_support(bin);
    if let Ok(mut map) = cache.lock() {
        map.insert(bin.to_string(), detected);
    }
    detected
}

pub struct ClaudeRunner;

impl Runner for ClaudeRunner {
    fn build(&self, cmd: &str, cwd: &str) -> anyhow::Result<RunnerSpec> {
        let bin = resolve_tool_bin("claude", "RELAY_CLAUDE_BIN", "claude");
        validate_bin_exists(
            &bin,
            "claude (set RELAY_CLAUDE_BIN=/path/to/claude or install shims to record real path)",
        )?;

        let mut final_cmd = cmd.trim().to_string();
        if final_cmd.is_empty() {
            final_cmd = bin.clone();
        } else {
            final_cmd = swap_leading_token(&final_cmd, "claude", &bin);
        }

        let command = if looks_like_shell(&final_cmd) {
            command_from_shell(&final_cmd, cwd)
        } else {
            let mut command = command_from_cmdline(&final_cmd, cwd);

            // Happy-alignment: enable `relay mcp` tools for Claude Code (best-effort).
            //
            // Claude supports MCP via `--mcp-config <json>`, where JSON has shape:
            //   { "mcpServers": { "<name>": { "command": "...", "args": ["..."] } } }
            //
            // Behavior:
            // - Default: enabled when `--mcp-config` is supported by the installed Claude CLI.
            // - Opt-out: set `RELAY_CLAUDE_DISABLE_RELAY_MCP=1` (or `RELAY_CLAUDE_ENABLE_RELAY_MCP=0`).
            let enable_relay_mcp = !env_falsy("RELAY_CLAUDE_ENABLE_RELAY_MCP")
                && !env_truthy("RELAY_CLAUDE_DISABLE_RELAY_MCP");
            if enable_relay_mcp && detect_claude_mcp_support_cached(&bin).mcp_config {
                let cfg = serde_json::json!({
                    "mcpServers": {
                        "relay": {
                            "command": resolve_relay_mcp_command(),
                            "args": ["mcp"]
                        }
                    }
                });
                command.arg("--mcp-config");
                command.arg(cfg.to_string());
            }

            command
        };

        Ok(RunnerSpec {
            command,
            prompt_regex: base_prompt_regex("claude"),
        })
    }
}
