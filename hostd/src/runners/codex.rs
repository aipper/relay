use super::{
    Runner, RunnerSpec, base_prompt_regex, command_from_cmdline, command_from_shell,
    looks_like_shell, resolve_tool_bin, swap_leading_token, validate_bin_exists,
};

pub struct CodexRunner;

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
            command_from_shell(&final_cmd, cwd)
        } else {
            command_from_cmdline(&final_cmd, cwd)
        };

        Ok(RunnerSpec {
            command,
            prompt_regex: base_prompt_regex("codex"),
        })
    }
}
