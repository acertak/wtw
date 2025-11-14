use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;

use crate::cli::RemoveCommand;
use crate::config::Config;
use crate::error::AppError;
use crate::git::rev::RepoContext;
use crate::git::runner::{GitError, GitRunner};
use crate::git::worktree::{WorktreeInfo, list_worktrees};
use crate::worktree::common;

pub fn run(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    cmd: &RemoveCommand,
) -> Result<()> {
    let target = cmd
        .target
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::user("worktree name is required"))
        .map_err(anyhow::Error::from)?
        .to_string();

    if cmd.force_branch && !cmd.with_branch {
        return Err(AppError::user("--force-branch requires --with-branch").into());
    }

    let worktrees = list_worktrees(git)?;
    let base_dir = config.resolved_base_dir(repo.main_root());

    let target_info =
        find_target_worktree(&worktrees, &base_dir, &target).map_err(anyhow::Error::from)?;

    let current_path = common::normalize_path(repo.worktree_root());
    let target_path = common::normalize_path(&target_info.path);

    if target_path == current_path {
        return Err(AppError::user(format!(
            "cannot remove the current worktree '{}': {}",
            target,
            target_info.path.display()
        ))
        .into());
    }

    let display_path = common::normalize_path(&target_info.path);
    remove_worktree(git, &target_info.path, cmd.force).map_err(anyhow::Error::from)?;

    let mut stdout = io::stdout().lock();
    writeln!(
        stdout,
        "Removed worktree '{}' at {}",
        target,
        display_path.display()
    )?;

    if cmd.with_branch {
        if let Some(branch) = &target_info.branch {
            remove_branch(git, branch, cmd.force_branch).map_err(anyhow::Error::from)?;
            writeln!(stdout, "Removed branch '{}'", branch)?;
        }
    }

    Ok(())
}

fn find_target_worktree<'a>(
    worktrees: &'a [WorktreeInfo],
    base_dir: &Path,
    target: &str,
) -> std::result::Result<&'a WorktreeInfo, AppError> {
    let mut available = Vec::new();
    let mut found: Option<&WorktreeInfo> = None;

    for info in worktrees {
        if info.is_main {
            continue;
        }
        if !common::is_managed(info, base_dir) {
            continue;
        }

        if info.branch.as_deref() == Some(target) {
            found = Some(info);
            break;
        }

        if info
            .path
            .file_name()
            .map(|name| name.to_string_lossy() == target)
            .unwrap_or(false)
        {
            found = Some(info);
            break;
        }

        let name = common::display_name(info, base_dir);
        if name == target {
            found = Some(info);
            break;
        }

        available.push(name);
    }

    if let Some(info) = found {
        return Ok(info);
    }

    Err(worktree_not_found(target, available))
}

fn remove_worktree(git: &GitRunner, path: &Path, force: bool) -> std::result::Result<(), AppError> {
    let mut args: Vec<OsString> = Vec::new();
    args.push("worktree".into());
    args.push("remove".into());
    if force {
        args.push("--force".into());
    }
    args.push(path.to_string_lossy().into_owned().into());

    match git.run(args) {
        Ok(_) => Ok(()),
        Err(GitError::CommandFailed { stderr, .. }) => {
            let message = stderr.trim();
            if message.is_empty() {
                Err(AppError::git(format!(
                    "git worktree remove failed for {} without error output",
                    path.display()
                )))
            } else {
                Err(AppError::git(message.to_string()))
            }
        }
        Err(err) => Err(AppError::git(err.to_string())),
    }
}

fn remove_branch(git: &GitRunner, branch: &str, force: bool) -> std::result::Result<(), AppError> {
    let flag = if force { "-D" } else { "-d" };
    match git.run(["branch", flag, branch]) {
        Ok(_) => Ok(()),
        Err(GitError::CommandFailed { stderr, .. }) => {
            let message = stderr.trim();
            if message.is_empty() {
                Err(AppError::git(format!(
                    "failed to remove branch '{}'",
                    branch
                )))
            } else {
                Err(AppError::git(message.to_string()))
            }
        }
        Err(err) => Err(AppError::git(err.to_string())),
    }
}

fn worktree_not_found(target: &str, mut available: Vec<String>) -> AppError {
    let suggestion = if available.is_empty() {
        "Run 'wtw list' to see available worktrees.".to_string()
    } else {
        available.sort();
        available.dedup();
        format!(
            "Available worktrees: {}\nRun 'wtw list' to see available worktrees.",
            available.join(", ")
        )
    };

    AppError::user(format!("worktree '{}' not found\n{}", target, suggestion))
}
