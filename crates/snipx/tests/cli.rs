use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
fn fmt_writes_to_stdout_from_stdin() {
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .args(["fmt", "--as", "commentaria"])
        .write_stdin("[Alice]   a   Character.\n")
        .assert()
        .success()
        .stdout("[Alice] a Character.\n");
}

#[test]
fn fmt_accepts_short_input_form_aliases() {
    for (alias, input, expected) in [
        ("-c", "[Alice]   a   Character.\n", "[Alice] a Character.\n"),
        (
            "-m",
            "Prose  stays.\n/// [Alice]   a   Character.\n",
            "Prose  stays.\n/// [Alice] a Character.\n",
        ),
        (
            "-i",
            "Alice  promised. {{<   a   Promise}}",
            "Alice  promised. {{< a Promise}}",
        ),
    ] {
        let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

        command
            .args(["fmt", alias])
            .write_stdin(input)
            .assert()
            .success()
            .stdout(expected);
    }
}

#[test]
fn conflicting_input_forms_fail() {
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .args(["fmt", "--as", "commentaria", "-m"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("input form"));
}

#[test]
fn agreeing_input_forms_are_allowed() {
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .args(["fmt", "--as", "commentaria", "-c"])
        .write_stdin("[Alice]   a   Character.\n")
        .assert()
        .success()
        .stdout("[Alice] a Character.\n");
}

#[test]
fn repeated_agreeing_input_forms_are_allowed() {
    for args in [
        vec!["fmt", "-c", "-c"],
        vec!["fmt", "--as", "commentaria", "--as", "commentaria"],
    ] {
        let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

        command
            .args(args)
            .write_stdin("[Alice]   a   Character.\n")
            .assert()
            .success()
            .stdout("[Alice] a Character.\n");
    }
}

#[test]
fn fmt_write_updates_path_in_place() {
    let mut path = std::env::temp_dir();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    path.push(format!("snipx-cli-fmt-{unique}.snipx"));

    fs::write(&path, "[Alice]   a   Character.\n").expect("temp input should be writable");

    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");
    command
        .args([
            "fmt",
            "--as",
            "commentaria",
            "--write",
            path.to_str().expect("temp path should be utf-8"),
        ])
        .assert()
        .success()
        .stdout("");

    let output = fs::read_to_string(&path).expect("formatted temp input should be readable");
    fs::remove_file(&path).expect("temp input should be removable");

    assert_eq!(output, "[Alice] a Character.\n");
}
