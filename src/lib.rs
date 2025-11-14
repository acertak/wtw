use clap::Parser;

pub mod cli;
pub mod config;
pub mod error;
pub mod git;
pub mod hooks;
pub mod logging;
pub mod shell;
pub mod worktree;

use anyhow::{Result, anyhow};
use std::io::{self, Write};
use std::process::ExitCode;

pub fn run() -> Result<ExitCode> {
    let cli = cli::Cli::parse();
    let globals = cli.global.clone();
    logging::init(&globals)?;

    match cli.command {
        cli::Command::Add(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::add::run(&repo, &git, &config, &cmd)?;
        }
        cli::Command::List(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::list::run(
                &repo,
                &git,
                &config,
                worktree::list::ListOptions { json: cmd.json },
            )?;
        }
        cli::Command::Remove(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::remove::run(&repo, &git, &config, &cmd)?;
        }
        cli::Command::Cd(cmd) => {
            let repo = git::rev::RepoContext::discover(globals.repo.clone())?;
            let config = config::load_config(&repo)?;
            let git = git::GitRunner::new(repo.clone());
            worktree::resolve::run(&repo, &git, &config, cmd.target)?;
        }
        cli::Command::Init(cmd) => match cmd.shell {
            cli::ShellKind::Pwsh => {
                let profile = match &cmd.profile {
                    Some(path) => path.clone(),
                    None => shell::init::default_pwsh_profile()?,
                };
                shell::init::init_pwsh(&profile)?;
            }
            cli::ShellKind::Cmd => {
                return Err(anyhow!("shell 'cmd' is not supported yet"));
            }
            cli::ShellKind::Bash => {
                return Err(anyhow!("shell 'bash' is not supported yet"));
            }
        },
        cli::Command::ShellInit(cmd) => match cmd.shell {
            cli::ShellKind::Pwsh => {
                print!("{}", shell::pwsh::script());
                io::stdout().flush()?;
            }
            cli::ShellKind::Cmd => {
                return Err(anyhow!("shell 'cmd' is not supported yet"));
            }
            cli::ShellKind::Bash => {
                return Err(anyhow!("shell 'bash' is not supported yet"));
            }
        },
    }
    Ok(ExitCode::SUCCESS)
}
