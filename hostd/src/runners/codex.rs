use super::{
    Runner, RunnerSpec, base_prompt_regex, command_from_cmdline, command_from_shell, find_in_path,
    looks_like_shell, resolve_tool_bin, swap_leading_token, validate_bin_exists,
};

pub struct CodexRunner;

fn escape_toml_basic_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\"', "\\\"")
}

fn resolve_relay_mcp_command() -> Option<String> {
    if let Ok(v) = std::env::var("RELAY_MCP_COMMAND") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Some(v);
        }
    }

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => {
            if find_in_path("relay") {
                return Some("relay".to_string());
            }
            return None;
        }
    };
    let Some(dir) = exe.parent() else {
        if find_in_path("relay") {
            return Some("relay".to_string());
        }
        return None;
    };

    #[cfg(windows)]
    let candidate = dir.join("relay.exe");
    #[cfg(not(windows))]
    let candidate = dir.join("relay");

    if candidate.is_file() {
        return Some(candidate.to_string_lossy().to_string());
    }
    if find_in_path("relay") {
        return Some("relay".to_string());
    }
    None
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

fn merged_no_proxy_for_localhost() -> String {
    let existing = std::env::var("NO_PROXY")
        .or_else(|_| std::env::var("no_proxy"))
        .unwrap_or_default();
    let mut parts: Vec<String> = existing
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    for item in ["127.0.0.1", "localhost", "::1"] {
        if !parts.iter().any(|p| p.eq_ignore_ascii_case(item)) {
            parts.push(item.to_string());
        }
    }

    parts.join(",")
}

impl Runner for CodexRunner {
    fn build(&self, cmd: &str, cwd: &str) -> anyhow::Result<RunnerSpec> {
        // Default to launching `codex` directly in a PTY (closest to "type `codex` in terminal").
        // For advanced use (pipes/quotes/etc), we keep the `bash -lc` fallback.
        //
        // Env overrides:
        // - `RELAY_CODEX_BIN`: replace leading `codex` token in the command line (or the default).
        //
        // Shim support:
        // - if the user installs a `codex` wrapper in PATH, hostd must NOT call it (would recurse).
        //   We read `~/.relay/bin-map.json` (written by scripts/install-shims.sh) to find the real
        //   binary path to execute.
        let bin = resolve_tool_bin("codex", "RELAY_CODEX_BIN", "codex");
        validate_bin_exists(
            &bin,
            "codex (set RELAY_CODEX_BIN=/path/to/codex or install shims to record real path)",
        )?;

        let mut final_cmd = cmd.trim().to_string();
        if final_cmd.is_empty() {
            final_cmd = bin.clone();
        } else {
            final_cmd = swap_leading_token(&final_cmd, "codex", &bin);
        }

        let command = if looks_like_shell(&final_cmd) {
            let mut command = command_from_shell(&final_cmd, cwd);
            let no_proxy = merged_no_proxy_for_localhost();
            command.env("NO_PROXY", &no_proxy);
            command.env("no_proxy", &no_proxy);
            command.env("NO_UPDATE_NOTIFIER", "1");
            command
        } else {
            let mut command = command_from_cmdline(&final_cmd, cwd);

            let no_proxy = merged_no_proxy_for_localhost();
            command.env("NO_PROXY", &no_proxy);
            command.env("no_proxy", &no_proxy);
            command.env("NO_UPDATE_NOTIFIER", "1");

            if !env_truthy("RELAY_CODEX_KEEP_USER_MCP") {
                command.arg("--config");
                command.arg("mcp_servers.metamcp.enabled=false");
                command.arg("--config");
                command.arg("mcp_servers.serena.enabled=false");
            }

            // Happy-alignment (A): make Codex aware of `relay mcp` tools so it can use them for
            // file ops / shell execution, with approvals handled by relay PWA via hostd.
            //
            if env_truthy("RELAY_CODEX_ENABLE_RELAY_MCP")
                && !env_truthy("RELAY_CODEX_DISABLE_RELAY_MCP")
            {
                if let Some(relay_cmd) = resolve_relay_mcp_command() {
                    let relay_cmd = escape_toml_basic_string(&relay_cmd);
                    command.arg("--config");
                    command.arg(format!(
                        r#"mcp_servers.relay={{command="{relay_cmd}", args=["mcp"], startup_timeout_sec=20, tool_timeout_sec=600, enabled=true}}"#,
                    ));
                    command.arg("--config");
                    command.arg(
                        r#"mcp_servers.relay.env={RELAY_RUN_ID="${RELAY_RUN_ID}", RELAY_HOSTD_SOCK="${RELAY_HOSTD_SOCK}", RELAY_TOOL="${RELAY_TOOL}"}"#,
                    );
                }
            }

            command
        };

        Ok(RunnerSpec {
            command,
            prompt_regex: base_prompt_regex("codex"),
            approve_text: "y\n".to_string(),
            deny_text: "n\n".to_string(),
        })
    }
}
