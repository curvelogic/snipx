use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_file(name: &str, contents: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    path.push(format!("snipx-cli-{name}-{unique}.txt"));
    fs::write(&path, contents).expect("temp input should be writable");
    path
}

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

#[test]
fn export_pretty_prints_partial_json_and_returns_one_for_errors() {
    let target = temp_file("target", "Bob waited.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .args([
            "export",
            "--as",
            "commentaria",
            "--pretty",
            "--target",
            target.to_str().expect("temp path should be utf-8"),
        ])
        .write_stdin("[Alice] a Character.\n")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"snipxVersion\": \"0.0\""))
        .stdout(predicate::str::contains("SNIPPET_NOT_FOUND"));

    fs::remove_file(target).expect("temp target should be removable");
}

#[test]
fn resolve_reads_source_and_target_files() {
    let source = temp_file("source", "[Alice] a Character.\n");
    let target = temp_file("target", "Alice waited.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .args([
            "resolve",
            "-c",
            "--target",
            target.to_str().expect("temp path should be utf-8"),
            source.to_str().expect("temp path should be utf-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"spans\":[{\"start\":0,\"end\":5}]",
        ));

    fs::remove_file(source).expect("temp source should be removable");
    fs::remove_file(target).expect("temp target should be removable");
}

#[test]
fn check_returns_one_for_parse_errors_and_strict_warnings() {
    let mut parse_error = Command::cargo_bin("snipx").expect("snipx binary should build");
    parse_error
        .args(["check", "-c"])
        .write_stdin("[Alice] friend.")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("PARSE_ERROR"));

    let mut warning = Command::cargo_bin("snipx").expect("snipx binary should build");
    warning
        .args(["check", "-m"])
        .write_stdin("```\n[Alice] a Character.\n")
        .assert()
        .success();

    let mut strict = Command::cargo_bin("snipx").expect("snipx binary should build");
    strict
        .args(["check", "-m", "--strict"])
        .write_stdin("```\n[Alice] a Character.\n")
        .assert()
        .code(1);
}

#[test]
fn common_cli_errors_use_documented_exit_codes() {
    let mut usage = Command::cargo_bin("snipx").expect("snipx binary should build");
    usage.args(["check", "-c", "-m"]).assert().code(2);

    let mut missing = Command::cargo_bin("snipx").expect("snipx binary should build");
    missing
        .args(["check", "/definitely/missing/snipx-input"])
        .assert()
        .code(3);
}

#[test]
fn cli_resolves_markdown_and_strict_mode_rejects_html_warnings() {
    let source = temp_file("markdown-source", "[Alice] a Character.\n");
    let clean_target = temp_file("markdown-target", "# Alice\n\nShe waited.\n");
    let html_target = temp_file("markdown-html-target", "Alice <span>waited</span>.\n");

    let mut clean = Command::cargo_bin("snipx").expect("snipx binary should build");
    clean
        .args([
            "resolve",
            "-c",
            "--profile",
            "markdown",
            "--target",
            clean_target.to_str().expect("temp path should be utf-8"),
            source.to_str().expect("temp path should be utf-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"profile\":\"markdown\""))
        .stdout(predicate::str::contains("\"start\":0,\"end\":5"));

    let mut warning = Command::cargo_bin("snipx").expect("snipx binary should build");
    warning
        .args([
            "export",
            "-c",
            "--profile",
            "markdown",
            "--strict",
            "--target",
            html_target.to_str().expect("temp path should be utf-8"),
            source.to_str().expect("temp path should be utf-8"),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("RAW_HTML_OMITTED"));

    for path in [source, clean_target, html_target] {
        fs::remove_file(path).expect("temp input should be removable");
    }
}

#[test]
fn ambient_subject_is_available_to_marginalia_commands() {
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .args(["export", "-m", "--ambient", "[]"])
        .write_stdin("/// a Character.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\":\"wholeDocument\""));
}

#[test]
fn ambient_numbers_must_be_finite_json_numbers() {
    for expression in ["NaN", "inf", "-inf", "Infinity", "1e999"] {
        let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

        command
            .args(["export", "-m", "--ambient", expression])
            .write_stdin("/// a Character.\n")
            .assert()
            .code(2)
            .stderr(predicate::str::contains("ambient number must be finite"));
    }
}

#[test]
fn ambient_values_must_consume_one_complete_expression() {
    for expression in ["[Alice]junk", "\"unterminated", "Alice Bob"] {
        let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

        command
            .args(["export", "-m", "--ambient", expression])
            .write_stdin("/// a Character.\n")
            .assert()
            .code(2)
            .stderr(predicate::str::contains("invalid ambient expression"));
    }
}

#[test]
fn ambient_values_use_the_core_expression_grammar() {
    for (expression, expected) in [
        ("Alice", "\"kind\":\"name\",\"value\":\"Alice\""),
        ("[Alice]", "\"kind\":\"snippet\",\"source\":\"[Alice]\""),
        (
            "~[Alice]",
            "\"kind\":\"textSpanSnippet\",\"source\":\"[Alice]\"",
        ),
        (
            "<chapter.txt>",
            "\"kind\":\"uri\",\"value\":\"chapter.txt\"",
        ),
        ("\"note\"", "\"kind\":\"string\",\"value\":\"note\""),
        ("true", "\"kind\":\"boolean\",\"value\":true"),
        ("-1", "\"kind\":\"number\",\"value\":-1.0"),
    ] {
        let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

        command
            .args(["export", "-m", "--ambient", expression])
            .write_stdin("/// a Character.\n")
            .assert()
            .success()
            .stdout(predicate::str::contains(expected));
    }
}
