use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli::AddCommand;
use crate::config::Config;
use crate::error::AppError;
use crate::git::rev::RepoContext;
use crate::git::runner::{GitError, GitRunner};
use crate::git::worktree::{WorktreeInfo, list_worktrees};
use crate::hooks::executor::HookExecutor;
use crate::worktree::common;

pub fn run(repo: &RepoContext, git: &GitRunner, config: &Config, cmd: &AddCommand) -> Result<()> {
    let existing = list_worktrees(git)?;
    let spec = build_spec(repo, config, cmd, &existing)?;

    ensure_parents_exist(&spec.path)?;
    run_git_add(git, &spec)?;

    let mut stdout = io::stdout().lock();
    let display_path = common::normalize_path(&spec.path);
    writeln!(
        stdout,
        "Created worktree '{}' at {}",
        spec.display_name,
        display_path.display()
    )?;

    let executor = HookExecutor::new(config, repo.main_root());
    executor.execute_post_create_hooks(&mut stdout, &spec.path)?;

    Ok(())
}

struct AddSpec {
    path: PathBuf,
    branch: Option<String>,
    commitish: Option<String>,
    track: bool,
    display_name: String,
}

fn build_spec(
    repo: &RepoContext,
    config: &Config,
    cmd: &AddCommand,
    existing: &[WorktreeInfo],
) -> Result<AddSpec> {
    let base_dir = config.resolved_base_dir(repo.main_root());

    let branch_flag = cmd
        .branch
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let track_flag = cmd
        .track
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let target_arg = cmd
        .target
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    let (branch, commitish, track) = if let Some(track) = track_flag {
        let inferred_branch = branch_flag
            .map(|s| s.to_string())
            .or_else(|| infer_branch_from_track(track));
        if inferred_branch.is_none() {
            return Err(AppError::user(
                "--track requires a branch name (use --branch or specify remote/branch)",
            )
            .into());
        }
        (inferred_branch, Some(track.to_string()), true)
    } else if branch_flag.is_some() {
        (
            branch_flag.map(|s| s.to_string()),
            target_arg.map(|s| s.to_string()),
            false,
        )
    } else {
        let commit = target_arg
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::user("branch or commit is required"))
            .map_err(anyhow::Error::from)?;
        (None, Some(commit), false)
    };

    let identifier = branch
        .clone()
        .or_else(|| commitish.clone())
        .ok_or_else(|| AppError::user("unable to determine worktree name"))
        .map_err(anyhow::Error::from)?;

    let relative = branch_to_relative_path(&identifier);
    if relative.components().next().is_none() {
        return Err(AppError::user(format!(
            "worktree name resolves to an empty path: {}",
            identifier
        ))
        .into());
    }

    let path = base_dir.join(&relative);
    detect_conflicts(&path, branch.as_deref(), existing)?;

    let display_name = branch.clone().unwrap_or_else(|| identifier.clone());

    Ok(AddSpec {
        path,
        branch,
        commitish,
        track,
        display_name,
    })
}

fn infer_branch_from_track(track: &str) -> Option<String> {
    track
        .split_once('/')
        .map(|(_, branch)| branch.to_string())
        .filter(|branch| !branch.is_empty())
}

fn branch_to_relative_path(name: &str) -> PathBuf {
    let mut result = PathBuf::new();
    for segment in name.split(['/', '\\']) {
        let sanitized = sanitize_segment(segment);
        if !sanitized.is_empty() {
            result.push(sanitized);
        }
    }
    if result.as_os_str().is_empty() {
        result.push(sanitize_segment(name));
    }
    result
}

fn sanitize_segment(segment: &str) -> String {
    if segment.is_empty() || segment == "." || segment == ".." {
        return "_".to_string();
    }

    let invalid_chars: HashSet<char> = ['<', '>', ':', '"', '|', '?', '*', '\\']
        .into_iter()
        .collect();

    segment
        .chars()
        .map(|ch| if invalid_chars.contains(&ch) { '_' } else { ch })
        .collect()
}

fn detect_conflicts(path: &Path, branch: Option<&str>, existing: &[WorktreeInfo]) -> Result<()> {
    if let Some(branch_name) = branch {
        if let Some(conflict) = existing
            .iter()
            .find(|wt| wt.branch.as_deref() == Some(branch_name))
        {
            return Err(AppError::user(format!(
                "worktree for branch '{}' already exists: {}",
                branch_name,
                conflict.path.display()
            ))
            .into());
        }
    }

    let target_normalized = common::normalize_path(path);

    if existing
        .iter()
        .map(|wt| common::normalize_path(&wt.path))
        .any(|existing_path| existing_path == target_normalized)
    {
        return Err(AppError::user(format!(
            "worktree path already exists in git metadata: {}",
            path.display()
        ))
        .into());
    }

    if path.exists() {
        return Err(AppError::user(format!(
            "destination path already exists: {}",
            path.display()
        ))
        .into());
    }

    Ok(())
}

fn ensure_parents_exist(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn run_git_add(git: &GitRunner, spec: &AddSpec) -> Result<()> {
    let mut args: Vec<OsString> = Vec::new();
    args.push("worktree".into());
    args.push("add".into());

    if spec.track {
        args.push("--track".into());
    }

    if let Some(branch) = &spec.branch {
        args.push("-b".into());
        args.push(branch.clone().into());
    }

    args.push(spec.path.to_string_lossy().into_owned().into());

    if let Some(commitish) = &spec.commitish {
        args.push(commitish.clone().into());
    }

    match git.run(args) {
        Ok(_) => Ok(()),
        Err(GitError::CommandFailed { stderr, .. }) => {
            let message = stderr.trim();
            if message.is_empty() {
                Err(AppError::git("git worktree add failed without error output").into())
            } else {
                Err(AppError::git(message.to_string()).into())
            }
        }
        Err(err) => Err(AppError::git(err.to_string()).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_to_relative_path_replaces_forbidden_characters() {
        let path = branch_to_relative_path("feat:bad*name");
        assert_eq!(path, PathBuf::from("feat_bad_name"));
    }

    #[test]
    fn branch_to_relative_path_normalizes_segments() {
        let path = branch_to_relative_path("feature//..//auth");
        let mut expected = PathBuf::new();
        expected.push("feature");
        expected.push("_");
        expected.push("_");
        expected.push("_");
        expected.push("auth");
        assert_eq!(path, expected);
    }

    #[test]
    fn infer_branch_from_track_parses_remote_branch() {
        assert_eq!(
            infer_branch_from_track("origin/feature/auth"),
            Some("feature/auth".to_string())
        );
        assert_eq!(infer_branch_from_track("origin-only"), None);
    }
}
