# SnipX

SnipX is a small text annotation language for writing structured and
semi-structured notes against fiction and other prose documents.

Its central primitive is the **snippet**: a human-written reference to a range
of visible text in a target document. SnipX resolves snippets over a canonical
visible-text stream rather than source markup, style runs, or editor metadata.

This repository contains the draft v0 language specification and a working
Rust reference implementation.

## Implemented Features

- lossless parsing for commentaria, marginalia, and intralinea;
- typed syntax-tree queries and source-located diagnostics;
- conservative formatting that preserves prose outside SnipX regions;
- statement expansion, ambient subjects, and decoration sugar;
- exact and loose snippet matching, captures, ranges, and quantifiers;
- plain-text and Markdown visible-text extraction;
- canonical JSON export, including partial results and diagnostics;
- command-line checking, resolution, export, and formatting;
- snapshot, property, and fuzz-test coverage.

The reference implementation supports the `plain`, `plain-loose`, `markdown`,
and `markdown-loose` profiles. Rich-document profiles described by the
language specification are future work.

## Build

SnipX requires a stable Rust toolchain.

```bash
cargo build --workspace
cargo run -p snipx -- --help
```

The built CLI is available at `target/debug/snipx`. The examples below use
`snipx` for readability; replace it with `cargo run -q -p snipx --` when
running directly from a checkout.

## CLI

The CLI reads source from `PATH`, or from standard input when `PATH` is omitted
or is `-`. Commentaria is the default input form. Select another form with
`--as marginalia`, `--as intralinea`, `-m`, or `-i`; `-c` explicitly selects
commentaria.

### Check and export

```bash
snipx check --pretty notes.snipx
snipx export --pretty notes.snipx
```

Both commands emit canonical JSON. `check` is useful when diagnostics and exit
status are the primary result; `export` names the JSON-oriented workflow.
`--strict` makes warnings produce exit code 1.

### Resolve snippets

```bash
snipx resolve \
  --target chapter.md \
  --profile markdown-loose \
  --pretty \
  notes.snipx
```

`--target PATH` supplies the document whose visible text snippets reference.
The supported profiles are:

- `plain`
- `plain-loose`
- `markdown`
- `markdown-loose`

Use `--ambient EXPR` to provide an ambient subject for subjectless statements.
The value must be one complete SnipX subject expression; use `[]` for the whole
document.

### Format

Format standard input to standard output:

```bash
printf '%s\n' '[Alice]   a   Character.' | snipx fmt
```

Format a file in place:

```bash
snipx fmt --write notes.snipx
```

`--write` requires a real filesystem path. It rejects both a missing path and
the standard-input marker `-` before attempting to read standard input.

### Exit codes

| Code | Meaning |
| ---: | --- |
| 0 | Success |
| 1 | Error diagnostics, or warnings with `--strict` |
| 2 | Invalid command-line usage |
| 3 | Input/output failure |
| 4 | Unsupported feature or profile (reserved) |

The `check`, `resolve`, and `export` commands still emit partial canonical JSON
when parsing or resolution produces diagnostics.

## Library

The `snipx-core` crate exposes the reference implementation as a library. Its
public API includes parsing, typed AST access, formatting, expansion,
visible-text extraction, snippet matching and resolution, and canonical JSON
export. The `snipx` crate provides the CLI.

## Documentation

- [Language specification](docs/language-spec.md) defines the draft v0 syntax
  and semantics, including explicitly deferred features.
- [Canonical JSON](docs/canonical-json.md) documents the machine-readable
  export format, including span offset conventions.
- Public Rust API documentation can be generated with:

  ```bash
  cargo doc --workspace --all-features --no-deps
  ```

## Development

Run the same stable-toolchain gates used by CI:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Build the fuzz target with a nightly toolchain and `cargo-fuzz`:

```bash
cargo +nightly fuzz build parser
```

Project work is tracked with [Beads](https://github.com/gastownhall/beads).
Use `bd ready` to find available work and `bd show <id>` for details.

## Project Status

The planned v0 Rust reference implementation is complete and released as
v0.1.1. The language specification remains a draft. Later-version ideas—including rich-document extraction,
editable rich-text mappings, additional syntax, and RDF-like export—are listed
under [Deferred And Open Issues](docs/language-spec.md#deferred-and-open-issues)
in the language specification.

## Licence

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or
the [MIT license](LICENSE-MIT) at your option.
