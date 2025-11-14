use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::error::AppError;
use crate::git::runner::GitRunner;

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub head: String,
    pub branch: Option<String>,
    pub is_main: bool,
    pub is_detached: bool,
    pub locked: Option<String>,
    pub prunable: Option<String>,
}

pub fn list_worktrees(git: &GitRunner) -> Result<Vec<WorktreeInfo>> {
    let output = git
        .run(["worktree", "list", "--porcelain"])
        .map_err(|err| AppError::git(err.to_string()))?;
    Ok(parse_worktree_list(output.stdout()))
}

fn parse_worktree_list(output: &str) -> Vec<WorktreeInfo> {
    let mut worktrees = Vec::new();
    let mut current = PartialWorktree::default();

    for line in output.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            if let Some(info) = current.finish() {
                worktrees.push(info);
            }
            current = PartialWorktree::default();
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(info) = current.finish() {
                worktrees.push(info);
            }
            current = PartialWorktree::default();
            current.path = Some(normalize_path(Path::new(path)));
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            current.head = Some(head.to_string());
        } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            current.branch = Some(branch.to_string());
        } else if line == "detached" {
            current.is_detached = true;
        } else if let Some(reason) = line.strip_prefix("locked ") {
            current.locked = Some(reason.trim().to_string());
        } else if line == "locked" {
            current.locked = Some(String::new());
        } else if let Some(reason) = line.strip_prefix("prunable ") {
            current.prunable = Some(reason.trim().to_string());
        } else if line == "prunable" {
            current.prunable = Some(String::new());
        }
    }

    if let Some(info) = current.finish() {
        worktrees.push(info);
    }

    if let Some(first) = worktrees.first_mut() {
        first.is_main = true;
    }

    worktrees
}

#[derive(Debug, Default)]
struct PartialWorktree {
    path: Option<PathBuf>,
    head: Option<String>,
    branch: Option<String>,
    is_detached: bool,
    locked: Option<String>,
    prunable: Option<String>,
}

impl PartialWorktree {
    fn finish(self) -> Option<WorktreeInfo> {
        let path = self.path?;
        let head = self.head.unwrap_or_default();
        let branch = if self.is_detached { None } else { self.branch };

        Some(WorktreeInfo {
            path,
            head,
            branch,
            is_main: false,
            is_detached: self.is_detached,
            locked: self.locked,
            prunable: self.prunable,
        })
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parse_marks_first_entry_as_main() {
        let temp = TempDir::new().expect("temp dir");
        let main = temp.path().join("repo-main");
        let feature = temp.path().join("repo-worktrees").join("feature-auth");
        fs::create_dir_all(&main).expect("main dir");
        fs::create_dir_all(&feature).expect("feature dir");

        let output = format!(
            "worktree {}\nHEAD 0123456789abcdef\nbranch refs/heads/main\n\n\
             worktree {}\nHEAD deadbeefcafebabe\nbranch refs/heads/feature/auth\n",
            main.display(),
            feature.display()
        );

        let infos = super::parse_worktree_list(&output);
        assert_eq!(infos.len(), 2);
        assert!(infos[0].is_main);
        assert_eq!(infos[1].branch.as_deref(), Some("feature/auth"));
        assert!(!infos[1].is_main);
    }

    #[test]
    fn parse_handles_detached_and_metadata_flags() {
        let temp = TempDir::new().expect("temp dir");
        let detached = temp.path().join("detached");
        fs::create_dir_all(&detached).expect("detached dir");

        let output = format!(
            "worktree {}\nHEAD 0123456789abcdef\ndetached\nlocked because testing\nprunable dirty\n",
            detached.display()
        );

        let infos = super::parse_worktree_list(&output);
        assert_eq!(infos.len(), 1);
        let info = &infos[0];
        assert!(info.is_detached);
        assert!(info.branch.is_none());
        assert_eq!(info.locked.as_deref(), Some("because testing"));
        assert_eq!(info.prunable.as_deref(), Some("dirty"));
    }
}
