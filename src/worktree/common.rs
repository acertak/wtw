use std::path::{Path, PathBuf};

use crate::git::worktree::WorktreeInfo;

pub fn normalize_path(path: &Path) -> PathBuf {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    strip_extended_prefix(canonical)
}

pub fn is_managed(info: &WorktreeInfo, base_dir: &Path) -> bool {
    if info.is_main {
        return true;
    }
    let info_path = normalize_path(&info.path);
    let base_dir = normalize_path(base_dir);
    info_path.starts_with(&base_dir)
}

pub fn display_name(info: &WorktreeInfo, base_dir: &Path) -> String {
    if info.is_main {
        return "@".to_string();
    }

    let info_path = normalize_path(&info.path);
    let base_dir = normalize_path(base_dir);

    if let Ok(relative) = info_path.strip_prefix(&base_dir) {
        if !relative.as_os_str().is_empty() {
            return components_to_string(relative);
        }
    }

    info_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| info_path.to_string_lossy().to_string())
}

fn strip_extended_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        const PREFIX: &str = r"\\?\";
        let display = path.to_string_lossy();
        if let Some(stripped) = display.strip_prefix(PREFIX) {
            PathBuf::from(stripped)
        } else {
            PathBuf::from(display.as_ref())
        }
    }
    #[cfg(not(windows))]
    {
        path
    }
}

fn components_to_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join(std::path::MAIN_SEPARATOR_STR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::worktree::WorktreeInfo;

    fn make_info(path: &str, branch: Option<&str>, is_main: bool) -> WorktreeInfo {
        WorktreeInfo {
            path: PathBuf::from(path),
            head: "deadbeef".to_string(),
            branch: branch.map(|s| s.to_string()),
            is_main,
            is_detached: false,
            locked: None,
            prunable: None,
        }
    }

    #[test]
    fn display_main_worktree_as_at() {
        let info = make_info("C:\\repo", Some("main"), true);
        let base_dir = PathBuf::from("C:\\repo\\worktrees");
        assert_eq!(display_name(&info, &base_dir), "@");
    }

    #[test]
    fn display_relative_worktree_name() {
        let info = make_info(
            "C:\\repo\\worktrees\\feature\\auth",
            Some("feature/auth"),
            false,
        );
        let base_dir = PathBuf::from("C:\\repo\\worktrees");
        assert_eq!(
            display_name(&info, &base_dir),
            format!("feature{}auth", std::path::MAIN_SEPARATOR)
        );
    }

    #[test]
    fn managed_worktree_within_base_dir() {
        let info = make_info(
            "C:\\repo\\worktrees\\feature\\auth",
            Some("feature/auth"),
            false,
        );
        let base_dir = PathBuf::from("C:\\repo\\worktrees");
        assert!(is_managed(&info, &base_dir));
    }

    #[test]
    fn unmanaged_worktree_outside_base_dir() {
        let info = make_info("D:\\temp\\feature", Some("feature"), false);
        let base_dir = PathBuf::from("C:\\repo\\worktrees");
        assert!(!is_managed(&info, &base_dir));
    }
}
