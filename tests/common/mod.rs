use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_cmd::Command;
use tempfile::TempDir;

/// Temporary Git repository tailored for WTW integration tests.
#[allow(dead_code)]
pub struct TestRepo {
    temp: TempDir,
    worktrees_dir: PathBuf,
}

#[allow(dead_code)]
impl TestRepo {
    /// Create a new repository with an initial commit and `.wtp.yml`.
    pub fn new() -> Self {
        let temp = TempDir::new().expect("create temp dir");
        run_git(temp.path(), &["init", "-q"]);
        run_git(temp.path(), &["config", "user.name", "wtw-test"]);
        run_git(temp.path(), &["config", "user.email", "wtw@example.com"]);

        fs::write(temp.path().join("README.md"), "hello world").unwrap();
        run_git(temp.path(), &["add", "README.md"]);
        run_git(temp.path(), &["commit", "-q", "-m", "init"]);

        let worktrees_dir = temp.path().join("worktree");
        fs::create_dir_all(&worktrees_dir).unwrap();
        let default_config = "version: \"1.0\"\ndefaults:\n  base_dir: worktree\n";
        fs::write(temp.path().join(".wtp.yml"), default_config).unwrap();
        run_git(temp.path(), &["add", ".wtp.yml"]);
        run_git(temp.path(), &["commit", "-q", "-m", "add wtp config"]);

        Self {
            temp,
            worktrees_dir,
        }
    }

    pub fn path(&self) -> &Path {
        self.temp.path()
    }

    pub fn worktrees_dir(&self) -> PathBuf {
        self.worktrees_dir.clone()
    }

    pub fn config_path(&self) -> PathBuf {
        self.path().join(".wtp.yml")
    }

    pub fn write_config(&self, contents: &str) {
        fs::write(self.config_path(), contents).unwrap();
    }

    pub fn command(&self) -> Command {
        #[allow(deprecated)]
        let mut cmd = Command::cargo_bin("wtw").expect("wtw binary");
        cmd.current_dir(self.path());
        cmd
    }

    pub fn command_in(&self, dir: &Path) -> Command {
        #[allow(deprecated)]
        let mut cmd = Command::cargo_bin("wtw").expect("wtw binary");
        cmd.current_dir(dir);
        cmd
    }

    pub fn git(&self, args: &[&str]) {
        run_git(self.path(), args);
    }

    pub fn create_branch(&self, name: &str) {
        self.git(&["branch", name]);
    }

    pub fn worktree_path_for(&self, name: &str) -> PathBuf {
        let mut path = self.worktrees_dir();
        for segment in name.split(['/', '\\']) {
            path.push(segment);
        }
        path
    }
}

#[allow(dead_code)]
pub fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub fn run_git(dir: &Path, args: &[&str]) {
    let status = StdCommand::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .expect("git");
    assert!(
        status.success(),
        "git {:?} failed in {} with {:?}",
        args,
        dir.display(),
        status.code()
    );
}
