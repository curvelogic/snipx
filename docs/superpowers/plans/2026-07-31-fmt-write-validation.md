# Early `fmt --write` Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make invalid `snipx fmt --write` invocations exit with a usage error before reading standard input.

**Architecture:** Resolve and validate the optional write target at the start of `run_fmt`, before `read_input`, and reuse that result when selecting output behavior. Prove the ordering with CLI integration tests whose piped stdin remains open and whose child process has a bounded timeout.

**Tech Stack:** Rust 2021, Clap 4, `assert_cmd` 2, `predicates` 3

## Global Constraints

- `fmt --write` without a path and `fmt --write -` must exit with code 2 and report `--write requires a path argument`.
- Invalid write-mode arguments must be rejected while standard input remains open.
- Non-write stdin formatting and valid in-place file formatting must remain unchanged.
- Do not add dependencies or refactor unrelated CLI behavior.

---

### Task 1: Validate the write target before reading input

**Files:**
- Modify: `crates/snipx/tests/cli.rs:1-145`
- Modify: `crates/snipx/src/main.rs:191-213`

**Interfaces:**
- Consumes: `FmtArgs { write: bool, path: Option<PathBuf> }`, `CliError::usage(&str)`, and `read_input(Option<&Path>)`.
- Produces: unchanged `run_fmt(FmtArgs) -> Result<u8, CliError>` behavior with a validated `Option<&PathBuf>` write target.

- [x] **Step 1: Write the failing CLI regression test**

Update the test imports and add this test beside `fmt_write_updates_path_in_place`:

```rust
use assert_cmd::cargo::CommandCargoExt;
use std::io::Read;
use std::process::{Command as ProcessCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[test]
fn fmt_write_rejects_invalid_paths_before_reading_stdin() {
    for args in [vec!["fmt", "--write"], vec!["fmt", "--write", "-"]] {
        let mut command =
            ProcessCommand::cargo_bin("snipx").expect("snipx binary should build");
        let mut child = command
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("snipx binary should start");
        let _open_stdin = child.stdin.take().expect("stdin should be piped");
        let deadline = Instant::now() + Duration::from_secs(2);
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
```

The test explicitly retains the pipe writer in `_open_stdin`, polls the real
child process with a two-second deadline, and kills it before panicking if it
blocks. This makes the current behavior fail deterministically instead of
hanging the suite.

- [x] **Step 2: Run the regression test to verify it fails**

Run:

```bash
cargo test -p snipx --test cli fmt_write_rejects_invalid_paths_before_reading_stdin -- --exact
```

Expected: FAIL after the two-second timeout because the current implementation
blocks in `read_input` and is killed instead of exiting with code 2.

- [x] **Step 3: Move write-target validation before the input read**

Change `run_fmt` to resolve the write target once and use it for output:

```rust
fn run_fmt(args: FmtArgs) -> Result<u8, CliError> {
    let input_form = select_input_form(&args.input)?;
    let write_path = if args.write {
        Some(
            args.path
                .as_ref()
                .filter(|path| path.as_path() != Path::new("-"))
                .ok_or_else(|| CliError::usage("--write requires a path argument"))?,
        )
    } else {
        None
    };
    let input = read_input(args.path.as_deref())?;
    let result = format(&input, FormatOptions { input_form });
    let has_errors = result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == snipx_core::Severity::Error);

    if let Some(path) = write_path {
        fs::write(path, result.output).map_err(|source| CliError::io(path, source))?;
    } else {
        write!(io::stdout(), "{}", result.output).map_err(CliError::stdout)?;
    }

    Ok(u8::from(has_errors))
}
```

- [x] **Step 4: Run the focused tests to verify the change**

Run:

```bash
cargo test -p snipx --test cli fmt_write
```

Expected: PASS for the new invalid-path test and the existing in-place write
test.

- [x] **Step 5: Run formatting and full verification**

Run:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-features
```

Expected: both commands exit successfully with no test failures.

- [x] **Step 6: Commit the implementation**

```bash
git add crates/snipx/src/main.rs crates/snipx/tests/cli.rs docs/superpowers/plans/2026-07-31-fmt-write-validation.md
git commit -m "Validate fmt write target before stdin"
```
