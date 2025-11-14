use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use tracing::debug;

use crate::git::rev::RepoContext;

#[derive(Debug, Clone)]
pub struct GitRunner {
    repo: RepoContext,
}

impl GitRunner {
    pub fn new(repo: RepoContext) -> Self {
        Self { repo }
    }

    pub fn repo(&self) -> &RepoContext {
        &self.repo
    }

    pub fn run<I, S>(&self, args: I) -> std::result::Result<GitOutput, GitError>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let output = self.run_with_status(args)?;
        if !output.status.success() {
            return Err(GitError::CommandFailed {
                command: output.command.clone(),
                status: output.status,
                stdout: output.stdout.clone(),
                stderr: output.stderr.clone(),
            });
        }
        Ok(output)
    }

    pub fn run_with_status<I, S>(&self, args: I) -> std::result::Result<GitOutput, GitError>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.run_with_status_in(self.repo.worktree_root(), args)
    }

    pub fn run_in<I, S>(
        &self,
        working_dir: &Path,
        args: I,
    ) -> std::result::Result<GitOutput, GitError>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let output = self.run_with_status_in(working_dir, args)?;
        if !output.status.success() {
            return Err(GitError::CommandFailed {
                command: output.command.clone(),
                status: output.status,
                stdout: output.stdout.clone(),
                stderr: output.stderr.clone(),
            });
        }
        Ok(output)
    }

    pub fn run_with_status_in<I, S>(
        &self,
        working_dir: &Path,
        args: I,
    ) -> std::result::Result<GitOutput, GitError>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        execute_git(working_dir, args)
    }
}

fn execute_git<I, S>(working_dir: &Path, args: I) -> std::result::Result<GitOutput, GitError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args_vec: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let command_string = format_command(&args_vec);

    debug!(
        repo = %working_dir.display(),
        command = %command_string,
        "Executing git command"
    );

    let mut command = Command::new("git");
    command.current_dir(working_dir);
    command.args(&args_vec);

    let output = command.output().map_err(|source| GitError::Spawn {
        command: command_string.clone(),
        source,
        working_directory: working_dir.to_path_buf(),
    })?;

    let stdout = String::from_utf8(output.stdout).map_err(|source| GitError::InvalidUtf8 {
        command: command_string.clone(),
        source,
    })?;

    let stderr = String::from_utf8(output.stderr).map_err(|source| GitError::InvalidUtf8 {
        command: command_string.clone(),
        source,
    })?;

    Ok(GitOutput {
        command: command_string,
        status: output.status,
        stdout,
        stderr,
    })
}

#[derive(Debug)]
pub struct GitOutput {
    pub command: String,
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl GitOutput {
    pub fn into_stdout(self) -> String {
        self.stdout
    }

    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    pub fn stderr(&self) -> &str {
        &self.stderr
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GitError {
    #[error("failed to execute git command '{command}' in {working_directory}: {source}")]
    Spawn {
        command: String,
        working_directory: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("git command '{command}' failed with status {status}")]
    CommandFailed {
        command: String,
        status: ExitStatus,
        stdout: String,
        stderr: String,
    },
    #[error("git command '{command}' returned invalid UTF-8 output: {source}")]
    InvalidUtf8 {
        command: String,
        #[source]
        source: std::string::FromUtf8Error,
    },
}

fn format_command(args: &[OsString]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push("git".to_string());
    for arg in args {
        parts.push(format_argument(arg));
    }
    parts.join(" ")
}

fn format_argument(arg: &OsString) -> String {
    let value = arg.to_string_lossy();
    if value.is_empty() {
        "\"\"".to_string()
    } else if value.contains(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
        let escaped = value.replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        value.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    #[test]
    fn format_command_quotes_arguments_with_spaces() {
        let args = [
            OsString::from("status"),
            OsString::from("--short"),
            OsString::from("feature branch"),
        ];
        let formatted = super::format_command(&args);
        assert_eq!(formatted, r#"git status --short "feature branch""#);
    }

    #[test]
    fn format_argument_escapes_existing_quotes() {
        let arg = OsString::from("say \"hello\"");
        let formatted = super::format_argument(&arg);
        assert_eq!(formatted, r#""say \"hello\"""#);
    }

    #[test]
    fn format_argument_leaves_simple_values_untouched() {
        let arg = OsString::from("--json");
        let formatted = super::format_argument(&arg);
        assert_eq!(formatted, "--json");
    }
}
