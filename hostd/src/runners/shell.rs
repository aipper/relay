use super::{Runner, RunnerSpec, base_prompt_regex};
use portable_pty::CommandBuilder;

pub struct ShellRunner;

impl Runner for ShellRunner {
    fn build(&self, cmd: &str, cwd: &str) -> anyhow::Result<RunnerSpec> {
        let mut command = CommandBuilder::new("bash");
        command.arg("-lc");
        command.arg(cmd);
        command.cwd(cwd);

        Ok(RunnerSpec {
            command,
            prompt_regex: base_prompt_regex("shell"),
        })
    }
}
