mod common;

use common::TestRepo;
use predicates::prelude::*;

#[test]
fn shell_init_pwsh_emits_wrapper_function() {
    TestRepo::new()
        .command()
        .args(["shell-init", "pwsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("function wtw"))
        .stdout(predicate::str::contains("Register-ArgumentCompleter"));
}

#[test]
fn shell_init_cmd_is_not_supported_yet() {
    TestRepo::new()
        .command()
        .args(["shell-init", "cmd"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("shell 'cmd' is not supported yet"));
}

