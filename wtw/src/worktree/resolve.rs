use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::Config;
use crate::error::AppError;
use crate::git::rev::RepoContext;
use crate::git::runner::GitRunner;
use crate::git::worktree::{WorktreeInfo, list_worktrees};
use crate::worktree::common;

pub fn run(
    repo: &RepoContext,
    git: &GitRunner,
    config: &Config,
    target: Option<String>,
) -> Result<()> {
    let target = target
        .ok_or_else(|| AppError::user("worktree name is required"))
        .map_err(anyhow::Error::from)?;
    let target = sanitize_target(&target);
    if target.is_empty() {
        return Err(AppError::user("worktree name is required").into());
    }

    let worktrees = list_worktrees(git)?;
    let base_dir = common::normalize_path(&config.resolved_base_dir(repo.main_root()));
    let repo_name = repo.repo_name().to_string();

    let resolved = resolve_path(&worktrees, &base_dir, &repo_name, &target)
        .ok_or_else(|| worktree_not_found(&target, &worktrees, &base_dir, &repo_name))
        .map_err(anyhow::Error::from)?;

    let mut stdout = io::stdout().lock();
    let normalized = common::normalize_path(&resolved);
    writeln!(stdout, "{}", normalized.display())?;
    Ok(())
}

fn sanitize_target(target: &str) -> String {
    target.trim().trim_end_matches('*').to_string()
}

fn resolve_path(
    worktrees: &[WorktreeInfo],
    base_dir: &Path,
    repo_name: &str,
    target: &str,
) -> Option<PathBuf> {
    for info in worktrees {
        let path = info.path.clone();

        if matches_main(info, repo_name, target) {
            return Some(path);
        }

        if !common::is_managed(info, base_dir) {
            continue;
        }

        if matches_branch(info, target) {
            return Some(path);
        }

        if matches_display_name(info, base_dir, target) {
            return Some(path);
        }

        if matches_directory_name(info, target) {
            return Some(path);
        }
    }

    None
}

fn matches_main(info: &WorktreeInfo, repo_name: &str, target: &str) -> bool {
    if !info.is_main {
        return false;
    }

    if target == "@" || target.eq_ignore_ascii_case("root") {
        return true;
    }

    if target.eq_ignore_ascii_case(repo_name) {
        return true;
    }

    if let Some(branch) = &info.branch {
        if branch == target {
            return true;
        }
    }

    false
}

fn matches_branch(info: &WorktreeInfo, target: &str) -> bool {
    info.branch
        .as_ref()
        .map(|branch| branch == target)
        .unwrap_or(false)
}

fn matches_display_name(info: &WorktreeInfo, base_dir: &Path, target: &str) -> bool {
    if info.is_main {
        return false;
    }

    let display_name = common::display_name(info, base_dir);
    display_name == target
}

fn matches_directory_name(info: &WorktreeInfo, target: &str) -> bool {
    info.path
        .file_name()
        .map(|name| name.to_string_lossy() == target)
        .unwrap_or(false)
}

fn worktree_not_found(
    target: &str,
    worktrees: &[WorktreeInfo],
    base_dir: &Path,
    repo_name: &str,
) -> AppError {
    let mut available = Vec::new();
    for info in worktrees {
        if !common::is_managed(info, base_dir) {
            continue;
        }
        available.push(common::display_name(info, base_dir));
    }

    if let Some(main) = worktrees.iter().find(|info| info.is_main) {
        available.insert(0, "@".to_string());
        if let Some(branch) = &main.branch {
            if !available.iter().any(|name| name == branch) {
                available.push(branch.clone());
            }
        }
        if !available
            .iter()
            .any(|name| name.eq_ignore_ascii_case(repo_name))
        {
            available.push(repo_name.to_string());
        }
    }

    available.sort();
    available.dedup();

    let suggestion = if available.is_empty() {
        String::from("Run 'wtw list' to see available worktrees.")
    } else {
        format!(
            "Available worktrees: {}\nRun 'wtw list' to see available worktrees.",
            available.join(", ")
        )
    };

    AppError::user(format!("worktree '{}' not found\n{}", target, suggestion))
}

#[cfg(test)]
mod tests {
    use crate::git::worktree::WorktreeInfo;
    use crate::worktree::common;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    #[test]
    fn sanitize_target_trims_whitespace_and_wildcards() {
        assert_eq!(super::sanitize_target("  feature/*  "), "feature/");
        assert_eq!(super::sanitize_target("@ "), "@");
    }

    #[test]
    fn resolve_path_matches_main_aliases_and_branch_names() {
        let fixture = Fixture::new();
        let worktrees = fixture.worktrees.clone();
        let base_dir = fixture.base_dir.clone();
        let repo = fixture.repo_name.clone();

        let resolved_main = super::resolve_path(&worktrees, &base_dir, &repo, "@").unwrap();
        assert_eq!(common::normalize_path(&resolved_main), fixture.main_path);

        let resolved_repo = super::resolve_path(&worktrees, &base_dir, &repo, &repo).unwrap();
        assert_eq!(common::normalize_path(&resolved_repo), fixture.main_path);

        let resolved_branch =
            super::resolve_path(&worktrees, &base_dir, &repo, "feature/auth").unwrap();
        assert_eq!(common::normalize_path(&resolved_branch), fixture.feature_path);
    }

    #[test]
    fn resolve_path_matches_display_names() {
        let fixture = Fixture::new();
        let worktrees = fixture.worktrees.clone();
        let base_dir = fixture.base_dir.clone();

        let resolved =
            super::resolve_path(&worktrees, &base_dir, &fixture.repo_name, &fixture.feature_display)
                .unwrap();
        assert_eq!(common::normalize_path(&resolved), fixture.feature_path);
    }

    #[test]
    fn worktree_not_found_lists_available_options() {
        let fixture = Fixture::new();
        let err =
            super::worktree_not_found("ghost", &fixture.worktrees, &fixture.base_dir, "repo");
        let message = format!("{err}");
        assert!(
            message.contains("Available worktrees"),
            "expected suggestions, got: {message}"
        );
        assert!(message.contains("Run 'wtw list'"));
        assert!(message.contains("@"));
    }

    #[derive(Debug)]
    struct Fixture {
        #[allow(dead_code)]
        temp: TempDir,
        base_dir: PathBuf,
        repo_name: String,
        worktrees: Vec<WorktreeInfo>,
        main_path: PathBuf,
        feature_path: PathBuf,
        feature_display: String,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = TempDir::new().expect("temp dir");
            let repo_root = temp.path().join("repo");
            fs::create_dir_all(&repo_root).expect("repo root");
            let base_dir = repo_root.join("worktrees");
            fs::create_dir_all(&base_dir).expect("base dir");
            let feature_dir = base_dir.join("feature").join("auth");
            fs::create_dir_all(&feature_dir).expect("feature dir");
            let bugfix_dir = base_dir.join("bugfix").join("one");
            fs::create_dir_all(&bugfix_dir).expect("bugfix dir");

            let main = make_info(&repo_root, Some("main"), true);
            let feature = make_info(&feature_dir, Some("feature/auth"), false);
            let bugfix = make_info(&bugfix_dir, Some("bugfix/one"), false);

            let feature_display = common::display_name(&feature, &base_dir);

            Self {
                temp,
                base_dir,
                repo_name: "repo".to_string(),
                worktrees: vec![main.clone(), feature.clone(), bugfix],
                main_path: common::normalize_path(&main.path),
                feature_path: common::normalize_path(&feature.path),
                feature_display,
            }
        }
    }

    fn make_info(path: &Path, branch: Option<&str>, is_main: bool) -> WorktreeInfo {
        WorktreeInfo {
            path: path.to_path_buf(),
            head: "0123456789abcdef".to_string(),
            branch: branch.map(|b| b.to_string()),
            is_main,
            is_detached: branch.is_none(),
            locked: None,
            prunable: None,
        }
    }
}
