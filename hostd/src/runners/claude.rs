use super::{
    Runner, RunnerSpec, base_prompt_regex, command_from_cmdline, command_from_shell,
    looks_like_shell, resolve_tool_bin, swap_leading_token, validate_bin_exists,
};

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
            command_from_cmdline(&final_cmd, cwd)
        };

        Ok(RunnerSpec {
            command,
            prompt_regex: base_prompt_regex("claude"),
        })
    }
}
