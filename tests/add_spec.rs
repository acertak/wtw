mod common;

use common::{TestRepo, normalize_path};
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

#[test]
fn add_creates_worktree_and_cd_returns_path() {
    let repo = TestRepo::new();
    let branch = "feature/auth";
    let worktree_path = repo.worktree_path_for(branch);

    repo.command()
        .args(["add", "-b", branch, "main"])
        .assert()
        .success()
        .stdout(predicate::str::contains(worktree_path.to_string_lossy()));

    assert!(
        worktree_path.exists(),
        "worktree directory should exist after add"
    );

    let cd_output = repo
        .command()
        .args(["cd", branch])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let cd_result = PathBuf::from(String::from_utf8(cd_output).unwrap().trim());
    let cd_path = normalize_path(&cd_result);
    assert_eq!(cd_path, normalize_path(&worktree_path));
}

#[test]
fn add_requires_branch_or_commit_argument() {
    let repo = TestRepo::new();
    let output = repo
        .command()
        .arg("add")
        .assert()
        .failure()
        .stderr(predicate::str::contains("branch or commit is required"))
        .get_output()
        .clone();
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn add_detects_existing_branch_conflicts() {
    let repo = TestRepo::new();
    let branch = "feature/duplicate";

    repo.command()
        .args(["add", "-b", branch, "main"])
        .assert()
        .success();

    repo.command()
        .args(["add", "-b", branch, "main"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("worktree for branch 'feature/duplicate' already exists"))
        .stderr(predicate::str::contains("worktree"));
}

#[test]
fn add_track_without_branch_requires_branch_name() {
    let repo = TestRepo::new();
    repo.command()
        .args(["add", "--track", "origin"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--track requires a branch name (use --branch or specify remote/branch)",
        ));
}

#[test]
fn post_create_hooks_copy_and_command_are_run() {
    let repo = TestRepo::new();
    fs::write(repo.path().join("template.txt"), "template").unwrap();
    repo.write_config(
        r#"version: "1.0"
defaults:
  base_dir: worktree
hooks:
  post_create:
    - type: copy
      from: template.txt
      to: copied.txt
    - type: command
      command: echo hook-run > hook.log
"#,
    );

    repo.command()
        .args(["add", "-b", "feature/hooks", "main"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Executing post-create hooks"))
        .stdout(predicate::str::contains("All hooks executed successfully"));

    let worktree_path = repo.worktree_path_for("feature/hooks");
    let copied = fs::read_to_string(worktree_path.join("copied.txt")).unwrap();
    assert_eq!(copied, "template");
    let hook_log = fs::read_to_string(worktree_path.join("hook.log")).unwrap();
    assert!(
        hook_log.contains("hook-run"),
        "hook command output should be captured"
    );
}

