use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_the_available_subcommands() {
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("check"))
        .stdout(predicate::str::contains("resolve"))
        .stdout(predicate::str::contains("export"))
        .stdout(predicate::str::contains("fmt"));
}

#[test]
fn check_subcommand_parses_successfully() {
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command.arg("check").assert().success();
}
