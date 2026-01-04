use portable_pty::CommandBuilder;
use regex::Regex;
use std::sync::Arc;

pub struct RunnerSpec {
    pub command: CommandBuilder,
    pub prompt_regex: Arc<Regex>,
}

pub trait Runner: Send + Sync {
    fn build(&self, cmd: &str, cwd: &str) -> anyhow::Result<RunnerSpec>;
}

const RELAY_SHIM_MARKER: &str = "relay shim (installed by scripts/install-shims.sh)";

fn bin_map_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        std::path::PathBuf::from(home)
            .join(".relay")
            .join("bin-map.json"),
    )
}

fn read_bin_map() -> std::collections::HashMap<String, String> {
    let Some(path) = bin_map_path() else {
        return std::collections::HashMap::new();
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&path) {
            let mode = meta.permissions().mode() & 0o777;
            if mode & 0o077 != 0 {
                tracing::warn!(
                    path=%path.to_string_lossy(),
                    mode=%format!("{mode:o}"),
                    "insecure permissions on bin-map.json (recommended 0600)"
                );
            }
        }
    }

    let Ok(raw) = std::fs::read_to_string(&path) else {
        return std::collections::HashMap::new();
    };
    serde_json::from_str::<std::collections::HashMap<String, String>>(&raw).unwrap_or_default()
}

pub fn resolve_tool_bin(tool: &str, env_var: &str, default_bin: &str) -> String {
    if let Ok(v) = std::env::var(env_var) {
        if !v.trim().is_empty() {
            return v;
        }
    }
    let map = read_bin_map();
    map.get(tool)
        .cloned()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default_bin.to_string())
}

fn is_relay_shim_path(path: &std::path::Path) -> bool {
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let mut buf = vec![0u8; 2048];
    let Ok(n) = std::io::Read::read(&mut f, &mut buf) else {
        return false;
    };
    let s = String::from_utf8_lossy(&buf[..n]);
    s.contains(RELAY_SHIM_MARKER)
}

pub fn find_path_in_path(bin: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let full = dir.join(bin);
        if full.is_file() {
            return Some(full);
        }
    }
    None
}

pub fn validate_bin_exists(bin: &str, hint: &str) -> anyhow::Result<()> {
    if bin.contains('/') {
        let p = std::path::Path::new(bin);
        if !p.is_file() {
            return Err(anyhow::anyhow!("{hint}: binary not found at path: {bin}"));
        }
        if is_relay_shim_path(p) {
            return Err(anyhow::anyhow!(
                "{hint}: resolved binary points to a relay shim (would recurse): {bin}"
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(p)?.permissions().mode() & 0o777;
            if mode & 0o111 == 0 {
                return Err(anyhow::anyhow!("{hint}: binary is not executable: {bin}"));
            }
        }
        return Ok(());
    }
    if let Some(found) = find_path_in_path(bin) {
        if is_relay_shim_path(&found) {
            return Err(anyhow::anyhow!(
                "{hint}: {bin} resolves to a relay shim in PATH (refusing to recurse); set RELAY_<TOOL>_BIN or reinstall shims to update ~/.relay/bin-map.json"
            ));
        }
        return Ok(());
    }
    Err(anyhow::anyhow!("{hint}: binary not found in PATH: {bin}"))
}

pub fn for_tool(tool: &str) -> Box<dyn Runner> {
    match tool {
        "codex" => Box::new(crate::runners::codex::CodexRunner {}),
        "claude" => Box::new(crate::runners::claude::ClaudeRunner {}),
        "iflow" => Box::new(crate::runners::iflow::IflowRunner {}),
        _ => Box::new(crate::runners::shell::ShellRunner {}),
    }
}

pub fn find_in_path(bin: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let full = dir.join(bin);
        if full.is_file() {
            return true;
        }
    }
    false
}

pub fn swap_leading_token(cmdline: &str, token: &str, replacement: &str) -> String {
    let trimmed = cmdline.trim_start();
    if trimmed == token {
        return replacement.to_string();
    }
    if let Some(rest) = trimmed.strip_prefix(token) {
        // Only swap when the next character is whitespace (so we don't replace "codexx").
        if rest.starts_with(char::is_whitespace) {
            return format!("{replacement}{rest}");
        }
    }
    cmdline.to_string()
}

fn looks_like_shell(cmd: &str) -> bool {
    // If it contains common shell metacharacters or quotes, keep the legacy `bash -lc` path.
    cmd.contains('\n')
        || cmd.contains(';')
        || cmd.contains('|')
        || cmd.contains('&')
        || cmd.contains('>')
        || cmd.contains('<')
        || cmd.contains('$')
        || cmd.contains('`')
        || cmd.contains('"')
        || cmd.contains('\'')
        || cmd.contains('(')
        || cmd.contains(')')
        || cmd.contains('{')
        || cmd.contains('}')
        || cmd.contains('[')
        || cmd.contains(']')
}

pub fn command_from_cmdline(cmdline: &str, cwd: &str) -> CommandBuilder {
    // Minimal tokenizer: safe for simple CLI invocations like "codex" or "codex --help".
    // Complex strings (quotes/metacharacters) are handled by the caller (fallback to bash -lc).
    let parts = cmdline.split_whitespace().collect::<Vec<_>>();
    let mut command = CommandBuilder::new(parts.first().copied().unwrap_or("bash"));
    for a in parts.iter().skip(1) {
        command.arg(*a);
    }
    command.cwd(cwd);
    command
}

pub fn command_from_shell(cmd: &str, cwd: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("bash");
    command.arg("-lc");
    command.arg(cmd);
    command.cwd(cwd);
    command
}

pub fn base_prompt_regex(tool: &str) -> Arc<Regex> {
    // MVP: heuristic patterns for interactive prompts.
    // Keep patterns broad but avoid matching arbitrary single letters.
    let base = r"(?ix)
        (proceed\\?|continue\\?|are\\s+you\\s+sure\\?|confirm\\b)
        |(\\(\\s*y\\s*/\\s*n\\s*\\))
        |(\\[\\s*y\\s*/\\s*n\\s*\\])
        |(\\(\\s*y\\s*/\\s*N\\s*\\))
        |(\\[\\s*y\\s*/\\s*N\\s*\\])
    ";
    let codex_extra = r"(?ix)
        |(allow\\b.*\\?)
        |(permission\\b.*\\?)
        |(approve\\b.*\\?)
    ";
    let pat = if tool == "codex" {
        format!("{base}{codex_extra}")
    } else {
        base.to_string()
    };
    Arc::new(Regex::new(&pat).expect("valid prompt regex"))
}

pub mod claude;
pub mod codex;
pub mod iflow;
pub mod shell;
