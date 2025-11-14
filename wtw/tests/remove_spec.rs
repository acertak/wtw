mod common;

use common::TestRepo;
use predicates::prelude::*;
use std::path::Path;

#[test]
fn remove_with_branch_deletes_branch_and_worktree() {
    let repo = TestRepo::new();
    let branch = "feature/remove";
    let worktree_path = repo.worktree_path_for(branch);

    repo.command()
        .args(["add", "-b", branch, "main"])
        .assert()
        .success();

    repo.command()
        .args(["remove", "--with-branch", "--force-branch", branch])
        .assert()
        .success()
        .stdout(predicate::str::contains(worktree_path.to_string_lossy()));

    assert!(
        !worktree_path.exists(),
        "worktree directory should be removed"
    );
    assert!(
        !branch_exists(repo.path(), branch),
        "branch should be deleted when --with-branch is supplied"
    );
}

#[test]
fn remove_only_targets_current_base_dir() {
    let repo = TestRepo::new();
    repo.create_branch("feature/legacy");
    repo.command()
        .args(["add", "feature/legacy"])
        .assert()
        .success();

    // Change base_dir so that existing worktree falls outside managed scope
    repo.write_config(
        r#"version: "1.0"
defaults:
  base_dir: alt-worktrees
"#,
    );

    repo.command()
        .args(["remove", "feature/legacy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));

    assert!(
        repo.worktree_path_for("feature/legacy").exists(),
        "worktree should remain because it is unmanaged under new base_dir"
    );
}

#[test]
fn remove_rejects_current_worktree() {
    let repo = TestRepo::new();
    repo.create_branch("feature/current");
    repo.command()
        .args(["add", "feature/current"])
        .assert()
        .success();
    let worktree_path = repo.worktree_path_for("feature/current");

    repo.command_in(&worktree_path)
        .args(["remove", "feature/current"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot remove the current worktree"));

    assert!(worktree_path.exists(), "current worktree must remain intact");
}

#[test]
fn remove_force_branch_requires_with_branch_flag() {
    let repo = TestRepo::new();
    repo.command()
        .args(["remove", "--force-branch", "feature/missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--force-branch requires --with-branch",
        ));
}

fn branch_exists(repo_path: &Path, branch: &str) -> bool {
    std::process::Command::new("git")
        .current_dir(repo_path)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .status()
        .expect("git show-ref")
        .success()
}

