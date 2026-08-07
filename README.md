# SnipX

SnipX is a small text annotation language for writing structured and
semi-structured notes against fiction and other prose documents.

Its central primitive is the **snippet**: a human-written reference to a range
of visible text in a target document. SnipX resolves snippets over a canonical
visible-text stream rather than source markup, style runs, or editor metadata.

This repository contains the draft v0 language specification, the Rust
reference implementation, a TypeScript implementation, and a conformance
corpus that holds every implementation to the same canonical-JSON
behaviour.

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

### Lint for fragile snippets

```bash
snipx lint --target chapter.md notes.snipx
```

`lint` behaves exactly like `check` but additionally warns about resolved
snippets that are likely to break or re-bind when the target is edited:
`FRAGILE_SHORT_ANCHOR` (anchor shorter than 5 Unicode scalars),
`FRAGILE_NEAR_DUPLICATE` (near-duplicate matches appear under loose
normalisation), and `FRAGILE_CAPTURE_CONTEXT` (capture context also occurs
elsewhere). Fragility diagnostics are always warnings, never change resolution
results, and affect the exit code only with `--strict`. The codes are
provisional pending ratification of
[ADR 0004](docs/adr/0004-fragility-diagnostics.md).

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

The `check`, `lint`, `resolve`, and `export` commands still emit partial
canonical JSON when parsing or resolution produces diagnostics.

## Library

The `snipx-core` crate exposes the reference implementation as a library. Its
public API includes parsing, typed AST access, formatting, expansion,
visible-text extraction, snippet matching and resolution, and canonical JSON
export. The `snipx` crate provides the CLI.

## TypeScript Implementation

`packages/snipx-ts` contains `@curvelogic/snipx`, a TypeScript
implementation scoped to canonical-JSON parity: parsing, expansion,
visible-text extraction (all four profiles), snippet resolution, and JSON
export. The formatter, lossless CST access, and CLI remain Rust-only. It
proves parity by running the same conformance corpus as the Rust
implementation.

```bash
pnpm install
pnpm -C packages/snipx-ts exec tsc --noEmit
pnpm -C packages/snipx-ts test
```

## Conformance

The [conformance corpus](conformance/README.md) defines conformance for
snipx implementations at the JSON export boundary: each case pairs an
export request with the expected export document, compared structurally
under the contract in `conformance/MANIFEST.json` (diagnostic codes
normative, message strings informative, the `implementation` block
excluded). Both implementations run the full corpus in their ordinary test
suites; see [ADR 0001](docs/adr/0001-conformance-corpus.md) for the design.

## Documentation

- [Language specification](docs/language-spec.md) defines the draft v0 syntax
  and semantics, including explicitly deferred features.
- [Canonical JSON](docs/canonical-json.md) documents the machine-readable
  export format, including span offset conventions.
- [Conformance corpus](conformance/README.md) describes the corpus layout,
  comparison contract, and how to run or regenerate it.
- [Architecture decision records](docs/adr/) capture cross-cutting
  decisions; ADRs marked Proposed await ratification.
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

For the TypeScript package:

```bash
pnpm install
pnpm -C packages/snipx-ts exec tsc --noEmit
pnpm -C packages/snipx-ts test
```

Build the fuzz target with a nightly toolchain and `cargo-fuzz`:

```bash
cargo +nightly fuzz build parser
```

Project work is tracked with [Beads](https://github.com/gastownhall/beads).
Use `bd ready` to find available work and `bd show <id>` for details.

## Project Status

The planned v0 Rust reference implementation is complete and released as
v0.1.1. A TypeScript implementation with full conformance parity lives in
`packages/snipx-ts`; the conformance corpus holds both implementations to
the same canonical-JSON behaviour. The language specification remains a draft. Later-version ideas—including rich-document extraction,
editable rich-text mappings, additional syntax, and RDF-like export—are listed
under [Deferred And Open Issues](docs/language-spec.md#deferred-and-open-issues)
in the language specification.

## Licence

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or
the [MIT license](LICENSE-MIT) at your option.
