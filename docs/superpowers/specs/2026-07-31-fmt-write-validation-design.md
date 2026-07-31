# Early `fmt --write` Validation

## Goal

Make `snipx fmt --write` report invalid write-mode input immediately, without
attempting to read standard input first.

## Behavior

`fmt --write` requires a real filesystem path. Both of these invocations are
usage errors:

```text
snipx fmt --write
snipx fmt --write -
```

They must exit with code 2 and report `--write requires a path argument` even
while their standard input remains open. An explicit `-` continues to mean
standard input for non-write formatting, but it is not a valid in-place write
target.

Formatting from standard input without `--write`, and formatting a real path
in place with `--write`, remain unchanged.

## Design

Keep validation in the `fmt` command handler because the constraint depends on
the relationship between `--write` and the positional path. At the beginning of
`run_fmt`, after input-form option validation but before `read_input`, resolve
the optional write target:

- when `--write` is false, no write target is needed;
- when `--write` is true and the path is absent or exactly `-`, return the
  existing usage error;
- when `--write` is true and a real path is present, retain that path for the
  later filesystem write.

This is deliberately narrower than adding a custom Clap parser or refactoring
all CLI input sources. It preserves the existing command-line contract and
changes only when the existing error is emitted.

## Error Handling

The existing `CliError::usage` path remains responsible for the diagnostic and
exit code. No file read, stdin read, formatting, stdout write, or filesystem
write occurs after invalid write-mode arguments are detected.

Filesystem read and write errors for valid paths keep their current handling.

## Testing

Add CLI integration coverage for both invalid forms. Each test launches the
binary with piped standard input that remains open, then uses a bounded wait to
prove the process exits instead of blocking. The assertions cover:

- exit code 2;
- the existing error text;
- no stdout output.

Existing tests continue to cover successful formatting from standard input and
successful in-place formatting of a real file. Run the full workspace test
suite and formatting checks after implementation.
