use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::config::Config;
use crate::error::AppError;
use crate::git::rev::RepoContext;
use crate::git::runner::{GitError, GitRunner};
use crate::git::worktree::{WorktreeInfo, list_worktrees};
use crate::worktree::common;

#[derive(Debug, Clone, Copy)]
pub struct ListOptions {
    pub json: bool,
}

pub fn run(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    options: ListOptions,
) -> Result<()> {
    let worktrees = list_worktrees(git)?;
    let base_dir = common::normalize_path(&config.resolved_base_dir(repo.main_root()));
    let current_worktree = common::normalize_path(repo.worktree_root());

    let rows = build_rows(&worktrees, git, &base_dir, &current_worktree)?;

    if options.json {
        output_json(&rows)?;
    } else {
        output_table(&rows)?;
    }

    Ok(())
}

fn build_rows(
    worktrees: &[WorktreeInfo],
    git: &GitRunner,
    base_dir: &Path,
    current_worktree: &Path,
) -> Result<Vec<DisplayRow>> {
    let mut rows = Vec::with_capacity(worktrees.len());

    for info in worktrees {
        let abs_path = common::normalize_path(&info.path);
        let is_current = abs_path == current_worktree;

        let status = determine_status(git, &abs_path)?;
        let upstream = determine_upstream(git, &abs_path)?;

        let name = common::display_name(info, base_dir);
        let branch_display = render_branch(info);
        let head_short = render_head(&info.head);
        let abs_path_display = abs_path.to_string_lossy().to_string();

        rows.push(DisplayRow {
            name,
            branch_display,
            branch: info.branch.clone(),
            head: head_short,
            status,
            upstream,
            abs_path: abs_path_display,
            is_main: info.is_main,
            is_current,
        });
    }

    Ok(rows)
}

fn determine_status(git: &GitRunner, worktree_path: &Path) -> Result<String> {
    let output = git
        .run_in(worktree_path, ["status", "--short"])
        .map_err(|err| AppError::git(err.to_string()))?;

    if output.stdout().trim().is_empty() {
        Ok("clean".to_string())
    } else {
        Ok("dirty".to_string())
    }
}

fn determine_upstream(git: &GitRunner, worktree_path: &Path) -> Result<Option<String>> {
    match git.run_in(
        worktree_path,
        ["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    ) {
        Ok(output) => {
            let value = output.stdout().trim();
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value.to_string()))
            }
        }
        Err(GitError::CommandFailed { .. }) => Ok(None),
        Err(err) => Err(AppError::git(err.to_string()).into()),
    }
}

fn render_branch(info: &WorktreeInfo) -> String {
    match &info.branch {
        Some(branch) => branch.clone(),
        None => "detached".to_string(),
    }
}

fn render_head(head: &str) -> String {
    const HEAD_LEN: usize = 8;
    if head.len() <= HEAD_LEN {
        head.to_string()
    } else {
        head[..HEAD_LEN].to_string()
    }
}

fn output_table(rows: &[DisplayRow]) -> Result<()> {
    let mut path_width = "PATH".len();
    let mut branch_width = "BRANCH".len();
    let mut status_width = "STATUS".len();
    let mut upstream_width = "UPSTREAM".len();
    let mut abs_path_width = "ABS_PATH".len();

    let rendered_names: Vec<String> = rows
        .iter()
        .map(|row| {
            let mut name = row.name.clone();
            if row.is_current {
                name.push('*');
            }
            name
        })
        .collect();

    for (idx, row) in rows.iter().enumerate() {
        let name_len = rendered_names[idx].len();
        path_width = path_width.max(name_len);
        branch_width = branch_width.max(row.branch_display.len());
        status_width = status_width.max(row.status.len());
        let upstream_len = row.upstream.as_deref().map_or(1, |s| s.len());
        upstream_width = upstream_width.max(upstream_len);
        abs_path_width = abs_path_width.max(row.abs_path.len());
    }

    let mut stdout = io::stdout().lock();
    writeln!(
        stdout,
        "{:<path_width$} {:<branch_width$} {:<8} {:<status_width$} {:<upstream_width$} {:<abs_path_width$}",
        "PATH", "BRANCH", "HEAD", "STATUS", "UPSTREAM", "ABS_PATH"
    )?;
    writeln!(
        stdout,
        "{:-<path_width$} {:-<branch_width$} {:-<8} {:-<status_width$} {:-<upstream_width$} {:-<abs_path_width$}",
        "", "", "", "", "", ""
    )?;

    for (idx, row) in rows.iter().enumerate() {
        let name = &rendered_names[idx];
        let upstream = row.upstream.as_deref().unwrap_or("-");
        writeln!(
            stdout,
            "{:<path_width$} {:<branch_width$} {:<8} {:<status_width$} {:<upstream_width$} {:<abs_path_width$}",
            name, row.branch_display, row.head, row.status, upstream, row.abs_path
        )?;
    }

    Ok(())
}

fn output_json(rows: &[DisplayRow]) -> Result<()> {
    let json_entries: Vec<JsonWorktree> = rows
        .iter()
        .map(|row| JsonWorktree {
            name: row.name.clone(),
            branch: row.branch.clone(),
            head: row.head.clone(),
            status: row.status.clone(),
            upstream: row.upstream.clone(),
            path: row.name.clone(),
            abs_path: row.abs_path.clone(),
            is_main: row.is_main,
            is_current: row.is_current,
        })
        .collect();

    let mut stdout = io::stdout().lock();
    let json = serde_json::to_string_pretty(&json_entries).context("failed to serialize JSON")?;
    stdout.write_all(json.as_bytes())?;
    stdout.write_all(b"\n")?;
    Ok(())
}

#[derive(Debug)]
struct DisplayRow {
    name: String,
    branch_display: String,
    branch: Option<String>,
    head: String,
    status: String,
    upstream: Option<String>,
    abs_path: String,
    is_main: bool,
    is_current: bool,
}

#[derive(Debug, Serialize)]
struct JsonWorktree {
    name: String,
    branch: Option<String>,
    head: String,
    status: String,
    upstream: Option<String>,
    path: String,
    abs_path: String,
    is_main: bool,
    is_current: bool,
}
