mod common;

use assert_cmd::Command as AssertCommand;
use common::TestRepo;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn repo_flag_allows_running_outside_git_directory() {
    let repo = TestRepo::new();
    let elsewhere = TempDir::new().expect("temp");
    let repo_path = repo.path().to_path_buf();
    #[allow(deprecated)]
    let mut cmd = AssertCommand::cargo_bin("wtw").expect("wtw binary");
    let output = cmd
        .current_dir(elsewhere.path())
        .args([
            "--repo",
            repo_path.to_string_lossy().as_ref(),
            "list",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    #[derive(serde::Deserialize)]
    struct ListEntry {
        #[allow(dead_code)]
        name: String,
    }

    let entries: Vec<ListEntry> = serde_json::from_slice(&output).unwrap();
    assert_eq!(entries.len(), 1);
}

#[test]
fn invalid_config_reports_exit_code_two() {
    let repo = TestRepo::new();
    repo.write_config("version: [");
    let output = repo
        .command()
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to parse config file"))
        .get_output()
        .clone();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn git_failure_returns_exit_code_three() {
    let repo = TestRepo::new();
    let output = repo
        .command()
        .args(["add", "-b", "feature/missing", "does-not-exist"])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn help_option_displays_usage_information() {
    #[allow(deprecated)]
    let mut cmd = AssertCommand::cargo_bin("wtw").expect("wtw binary");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Windows-native worktree helper"))
        .stdout(predicate::str::contains("Usage: wtw"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("shell-init"));
}

#[test]
fn version_option_prints_package_version() {
    #[allow(deprecated)]
    let mut cmd = AssertCommand::cargo_bin("wtw").expect("wtw binary");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn verbose_flag_emits_debug_logging() {
    let repo = TestRepo::new();
    repo.command()
        .args(["--verbose", "list", "--json"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Executing git command"));
}

