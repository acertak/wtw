mod common;

use common::{TestRepo, normalize_path};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ListEntry {
    name: String,
    branch: Option<String>,
    abs_path: String,
    is_main: bool,
    is_current: bool,
    status: String,
}

#[test]
fn list_json_includes_main_worktree() {
    let repo = TestRepo::new();
    let output = repo
        .command()
        .args(["list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let entries: Vec<ListEntry> =
        serde_json::from_slice(&output).expect("list --json returned invalid JSON");
    assert_eq!(entries.len(), 1);
    let main = entries.first().unwrap();
    assert_eq!(main.name, "@");
    assert_eq!(main.branch.as_deref(), Some("main"));
    assert_eq!(main.status, "clean");
    assert!(main.is_main);
    assert!(main.is_current);
    let abs_path = normalize_path(Path::new(&main.abs_path));
    assert_eq!(abs_path, normalize_path(repo.path()));
}

#[test]
fn list_reports_dirty_status_and_upstream() {
    let repo = TestRepo::new();
    let branch = "feature/list";
    repo.create_branch(branch);
    repo.command().args(["add", branch]).assert().success();

    let worktree_path = repo.worktree_path_for(branch);
    std::fs::write(worktree_path.join("dirty.txt"), "dirty").unwrap();
    common::run_git(&worktree_path, &["add", "dirty.txt"]);
    std::fs::write(worktree_path.join("dirty.txt"), "dirty+untracked").unwrap(); // leave staged dirty
    common::run_git(&worktree_path, &["branch", "--set-upstream-to=main"]);

    let table = String::from_utf8(
        repo.command()
            .arg("list")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();

    assert!(
        table.contains("PATH") && table.contains("UPSTREAM"),
        "table should include headers"
    );
    assert!(
        table.contains("dirty"),
        "STATUS column should show dirty worktree"
    );
    assert!(
        table.contains("main"),
        "UPSTREAM column should show configured upstream"
    );
}

#[test]
fn list_marks_current_worktree_with_asterisk() {
    let repo = TestRepo::new();
    let branch = "feature/current";
    repo.create_branch(branch);
    repo.command().args(["add", branch]).assert().success();
    let worktree_path = repo.worktree_path_for(branch);

    let output = String::from_utf8(
        repo.command_in(&worktree_path)
            .arg("list")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();

    let display_name = format!("feature{sep}current*", sep = std::path::MAIN_SEPARATOR);
    assert!(
        output.contains(&display_name),
        "expected '{}' to be marked current in:\n{}",
        display_name,
        output
    );
}

#[test]
fn list_json_reflects_current_and_main_flags() {
    let repo = TestRepo::new();
    repo.create_branch("feature/json");
    repo.command().args(["add", "feature/json"]).assert().success();
    let worktree_path = repo.worktree_path_for("feature/json");

    let output = repo
        .command_in(&worktree_path)
        .args(["list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<ListEntry> = serde_json::from_slice(&output).unwrap();

    assert_eq!(entries.len(), 2);
    let main = entries.iter().find(|e| e.is_main).unwrap();
    assert!(!main.is_current);
    let feature = entries
        .iter()
        .find(|e| e.branch.as_deref() == Some("feature/json"))
        .unwrap();
    assert!(feature.is_current);
}

