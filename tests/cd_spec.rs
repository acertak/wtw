mod common;

use common::{TestRepo, normalize_path};
use predicates::prelude::*;

#[test]
fn cd_at_resolves_repository_root() {
    let repo = TestRepo::new();
    let output = repo
        .command()
        .args(["cd", "@"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let cd_path = normalize_path(std::path::Path::new(
        String::from_utf8(output).unwrap().trim(),
    ));
    assert_eq!(cd_path, normalize_path(repo.path()));
}

#[test]
fn cd_accepts_display_name() {
    let repo = TestRepo::new();
    repo.create_branch("feature/cd");
    repo.command().args(["add", "feature/cd"]).assert().success();
    let display_name = format!("feature{}cd", std::path::MAIN_SEPARATOR);

    let output = repo
        .command()
        .arg("cd")
        .arg(&display_name)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let cd_path = normalize_path(std::path::Path::new(
        String::from_utf8(output).unwrap().trim(),
    ));
    assert_eq!(cd_path, normalize_path(&repo.worktree_path_for("feature/cd")));
}

#[test]
fn cd_unknown_worktree_suggests_available_options() {
    let repo = TestRepo::new();
    repo.create_branch("feature/known");
    repo.command().args(["add", "feature/known"]).assert().success();

    repo.command()
        .args(["cd", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("worktree 'missing' not found"))
        .stderr(predicate::str::contains("Available worktrees"))
        .stderr(predicate::str::contains("Run 'wtw list'"));
}

