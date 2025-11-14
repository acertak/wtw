use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow};

#[derive(Debug, Clone)]
pub struct RepoContext {
    worktree_root: PathBuf,
    main_root: PathBuf,
    repo_name: String,
}

impl RepoContext {
    pub fn discover(explicit: Option<PathBuf>) -> Result<Self> {
        let base_dir = match explicit {
            Some(path) => resolve_explicit_path(path)?,
            None => env::current_dir().context("failed to obtain current directory")?,
        };

        let worktree_root = git_rev_parse(&base_dir, ["rev-parse", "--show-toplevel"])
            .with_context(|| {
                format!(
                    "failed to determine git worktree root from {}",
                    base_dir.display()
                )
            })?;

        let common_dir = git_rev_parse(&worktree_root, ["rev-parse", "--git-common-dir"])
            .with_context(|| {
                format!(
                    "failed to determine git common directory from {}",
                    worktree_root.display()
                )
            })?;

        let main_root = resolve_main_root(&worktree_root, &common_dir)?;
        let repo_name = main_root
            .file_name()
            .map(|os| os.to_string_lossy().to_string())
            .unwrap_or_else(|| main_root.display().to_string());

        Ok(Self {
            worktree_root,
            main_root,
            repo_name,
        })
    }

    pub fn worktree_root(&self) -> &Path {
        &self.worktree_root
    }

    pub fn main_root(&self) -> &Path {
        &self.main_root
    }

    pub fn repo_name(&self) -> &str {
        &self.repo_name
    }

    pub fn is_main_worktree(&self) -> bool {
        normalize_path(&self.worktree_root) == normalize_path(&self.main_root)
    }
}

fn resolve_explicit_path(path: PathBuf) -> Result<PathBuf> {
    let expanded = if path.is_relative() {
        env::current_dir()
            .context("failed to obtain current directory")?
            .join(path)
    } else {
        path
    };

    let metadata = fs::metadata(&expanded)
        .with_context(|| format!("--repo path not found: {}", expanded.display()))?;

    let directory = if metadata.is_dir() {
        expanded
    } else {
        expanded
            .parent()
            .map(|parent| parent.to_path_buf())
            .ok_or_else(|| anyhow!("--repo path must be a directory: {}", expanded.display()))?
    };

    Ok(directory)
}

fn git_rev_parse(
    base_dir: &Path,
    args: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<PathBuf> {
    let mut command = Command::new("git");
    command.current_dir(base_dir);
    for arg in args {
        command.arg(arg.as_ref());
    }

    let output = command
        .output()
        .context("failed to execute git rev-parse command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim().split('\n').next().unwrap_or_default().trim();
        if message.is_empty() {
            return Err(anyhow!(
                "git rev-parse exited with status {}",
                output.status
            ));
        }
        return Err(anyhow!(message.to_string()));
    }

    let stdout =
        String::from_utf8(output.stdout).context("git rev-parse output contained invalid UTF-8")?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("git rev-parse returned an empty path"));
    }

    let mut resolved = PathBuf::from(trimmed);
    if !resolved.is_absolute() {
        resolved = base_dir.join(&resolved);
    }

    Ok(normalize_path(&resolved))
}

fn resolve_main_root(worktree_root: &Path, common_dir: &Path) -> Result<PathBuf> {
    let mut candidate = common_dir.to_path_buf();

    if !candidate.is_absolute() {
        candidate = worktree_root.join(candidate);
    }

    let canonical = normalize_path(&candidate);
    if canonical
        .file_name()
        .map(|name| name == ".git")
        .unwrap_or(false)
    {
        return canonical
            .parent()
            .map(|parent| parent.to_path_buf())
            .ok_or_else(|| anyhow!("git common dir has no parent: {}", canonical.display()));
    }

    Ok(canonical)
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
