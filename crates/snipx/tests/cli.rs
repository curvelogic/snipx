use assert_cmd::cargo::CommandCargoExt;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::io::Read;
use std::process::{Command as ProcessCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
        .stdout(predicate::str::contains("lint"))
        .stdout(predicate::str::contains("fmt"));
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("snipx-cli-{name}-{unique}"));
    fs::create_dir_all(&path).expect("temp dir should be creatable");
    path
}

#[test]
fn target_directive_resolves_relative_to_the_source_file() {
    let dir = temp_dir("target-directive");
    fs::write(dir.join("chapter.txt"), "Alice waited.").expect("target should be writable");
    let notes = dir.join("notes.snipx");
    fs::write(&notes, "@target <chapter.txt>\n\n[Alice] a Character.\n")
        .expect("notes should be writable");

    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");
    command
        .arg("export")
        .arg(&notes)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"uri\":\"chapter.txt\""))
        .stdout(predicate::str::contains(
            "\"spans\":[{\"start\":0,\"end\":5}]",
        ));
}

#[test]
fn cli_target_overrides_the_target_directive() {
    let dir = temp_dir("target-override");
    fs::write(dir.join("real.txt"), "Alice waited.").expect("target should be writable");
    let notes = dir.join("notes.snipx");
    fs::write(&notes, "@target <missing.txt>\n\n[Alice] a Character.\n")
        .expect("notes should be writable");

    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");
    command
        .arg("export")
        .arg(&notes)
        .arg("--target")
        .arg(dir.join("real.txt"))
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"spans\":[{\"start\":0,\"end\":5}]",
        ));
}

#[test]
fn missing_target_directive_file_is_an_io_error() {
    let dir = temp_dir("target-missing");
    let notes = dir.join("notes.snipx");
    fs::write(&notes, "@target <missing.txt>\n\n[Alice] a Character.\n")
        .expect("notes should be writable");

    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");
    command
        .arg("export")
        .arg(&notes)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("missing.txt"));
}

#[test]
fn unsupported_profile_directive_exits_with_code_four() {
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");
    command
        .arg("export")
        .write_stdin("@profile rtf-loose\n\nAlice a Character.\n")
        .assert()
        .code(4)
        .stdout(predicate::str::contains("UNSUPPORTED_PROFILE"));
}

#[test]
fn check_reports_diagnostics_without_facts_or_resolutions() {
    let target = temp_file("check-target", "Alice waited.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("check")
        .arg("--target")
        .arg(&target)
        .write_stdin("[Alice] a Character.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"diagnostics\""))
        .stdout(predicate::str::contains("\"facts\"").not())
        .stdout(predicate::str::contains("\"resolutions\"").not())
        .stdout(predicate::str::contains("\"visibleText\"").not());
}

#[test]
fn check_still_exits_one_on_resolution_errors() {
    let target = temp_file("check-error-target", "Bob waited.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("check")
        .arg("--target")
        .arg(&target)
        .write_stdin("[Alice] a Character.\n")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("SNIPPET_NOT_FOUND"));
}

#[test]
fn lint_reports_fragility_warnings_without_failing() {
    let target = temp_file("lint-target", "Bob waited.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("lint")
        .arg("--target")
        .arg(&target)
        .write_stdin("[Bob] a Character.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("FRAGILE_SHORT_ANCHOR"))
        .stdout(predicate::str::contains("\"severity\":\"warning\""))
        .stdout(predicate::str::contains("\"facts\"").not())
        .stdout(predicate::str::contains("\"resolutions\"").not());
}

#[test]
fn lint_strict_promotes_fragility_warnings_to_exit_one() {
    let target = temp_file("lint-strict-target", "Bob waited.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("lint")
        .arg("--strict")
        .arg("--target")
        .arg(&target)
        .write_stdin("[Bob] a Character.\n")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("FRAGILE_SHORT_ANCHOR"));
}

#[test]
fn check_does_not_report_fragility_warnings() {
    let target = temp_file("check-no-lint-target", "Bob waited.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("check")
        .arg("--target")
        .arg(&target)
        .write_stdin("[Bob] a Character.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("FRAGILE_").not());
}

#[test]
fn resolve_reports_resolutions_without_facts() {
    let target = temp_file("resolve-target", "Alice waited.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("resolve")
        .arg("--target")
        .arg(&target)
        .write_stdin("[Alice] a Character.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"resolutions\""))
        .stdout(predicate::str::contains("\"visibleText\""))
        .stdout(predicate::str::contains("\"facts\"").not());
}

#[test]
fn export_reports_facts_resolutions_and_diagnostics() {
    let target = temp_file("export-target", "Alice waited.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("export")
        .arg("--target")
        .arg(&target)
        .write_stdin("[Alice] a Character.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"facts\""))
        .stdout(predicate::str::contains("\"resolutions\""))
        .stdout(predicate::str::contains("\"diagnostics\""));
}

#[test]
fn version_flag_reports_the_crate_version() {
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
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
fn fmt_write_rejects_invalid_paths_before_reading_stdin() {
    let warmup_status = ProcessCommand::cargo_bin("snipx")
        .expect("snipx binary should build")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("snipx warmup should run");
    assert!(warmup_status.success());

    for args in [vec!["fmt", "--write"], vec!["fmt", "--write", "-"]] {
        let mut command = ProcessCommand::cargo_bin("snipx").expect("snipx binary should build");
        let mut child = command
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("snipx binary should start");
        let _open_stdin = child.stdin.take().expect("stdin should be piped");
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.try_wait().expect("child status should be readable") {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().expect("blocked child should be killable");
                child.wait().expect("killed child should be waitable");
                panic!("fmt --write blocked reading stdin for arguments {args:?}");
            }
            thread::sleep(Duration::from_millis(10));
        };
        let mut stdout = String::new();
        child
            .stdout
            .take()
            .expect("stdout should be piped")
            .read_to_string(&mut stdout)
            .expect("stdout should be readable");
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .expect("stderr should be piped")
            .read_to_string(&mut stderr)
            .expect("stderr should be readable");

        assert_eq!(status.code(), Some(2));
        assert_eq!(stdout, "");
        assert!(stderr.contains("--write requires a path argument"));
    }
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
        .stdout(predicate::str::contains("\"snipxVersion\": \"0.1\""))
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

#[test]
fn quantified_text_span_snippets_export_one_fact_per_span() {
    let target = temp_file("distribute-target", "Alice met Alice.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("export")
        .arg("--target")
        .arg(&target)
        .write_stdin("~[Alice]+ highlight true.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"span\":{\"start\":0,\"end\":5}"))
        .stdout(predicate::str::contains(
            "\"span\":{\"start\":10,\"end\":15}",
        ));
}
