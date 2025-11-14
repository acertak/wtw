use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::error::AppError;
use crate::git::rev::RepoContext;

use super::types::Config;

const CONFIG_FILE_NAME: &str = ".wtp.yml";

pub fn load_config(repo: &RepoContext) -> Result<Config> {
    let path = repo.main_root().join(CONFIG_FILE_NAME);
    if !path.exists() {
        return Ok(Config::default());
    }

    ensure_is_file(&path)?;

    let content = fs::read_to_string(&path).map_err(|err| {
        AppError::config(format!(
            "failed to read config file {}: {}",
            path.display(),
            err
        ))
    })?;

    let mut config: Config = serde_yaml::from_str(&content).map_err(|err| {
        AppError::config(format!(
            "failed to parse config file {}: {}",
            path.display(),
            err
        ))
    })?;

    if config.version.trim().is_empty() {
        config.version = super::types::DEFAULT_VERSION.to_owned();
    }

    Ok(config)
}

fn ensure_is_file(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path).map_err(|err| {
        AppError::config(format!(
            "failed to inspect config file {}: {}",
            path.display(),
            err
        ))
    })?;
    if !metadata.is_file() {
        return Err(AppError::config(format!(
            "configuration path is not a regular file: {}",
            path.display()
        ))
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn returns_default_when_config_missing() {
        let (_dir, repo) = temp_repo();
        let config = load_config(&repo).expect("load config");
        assert_eq!(config.version, types::DEFAULT_VERSION);
        assert_eq!(
            config.defaults.base_dir,
            PathBuf::from(types::DEFAULT_BASE_DIR)
        );
    }

    #[test]
    fn blank_version_is_replaced_with_default() {
        let (dir, repo) = temp_repo();
        let config_path = repo.main_root().join(super::CONFIG_FILE_NAME);
        fs::write(config_path, "version: \"  \"\n").expect("write config");

        let config = load_config(&repo).expect("load config");
        assert_eq!(config.version, types::DEFAULT_VERSION);

        drop(dir);
    }

    #[test]
    fn directory_config_is_rejected() {
        let (_dir, repo) = temp_repo();
        let config_path = repo.main_root().join(super::CONFIG_FILE_NAME);
        fs::create_dir_all(&config_path).expect("create dir");

        let err = load_config(&repo).expect_err("expected failure");
        let message = format!("{err}");
        assert!(message.contains("not a regular file"), "{}", message);
    }

    fn temp_repo() -> (TempDir, RepoContext) {
        let dir = TempDir::new().expect("temp repo");
        init_git(dir.path());
        let repo = RepoContext::discover(Some(dir.path().to_path_buf())).expect("repo context");
        (dir, repo)
    }

    fn init_git(path: &Path) {
        run_git(path, &["init", "-q"]);
        run_git(path, &["config", "user.email", "wtw@example.com"]);
        run_git(path, &["config", "user.name", "wtw-test"]);
        // minimal commit so that rev-parse behaves identically to real repos
        fs::write(path.join("README.md"), "init").expect("write file");
        run_git(path, &["add", "README.md"]);
        run_git(path, &["commit", "-q", "-m", "init"]);
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command.current_dir(dir);
        command.args(args);
        let status = command.status().expect("git status");
        assert!(status.success(), "git {:?} failed: {:?}", args, status);
    }
}
