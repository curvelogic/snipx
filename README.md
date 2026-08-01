# SnipX

SnipX is a small text annotation language for writing structured and
semi-structured notes against fiction and other prose documents.

The central primitive is the **snippet**: a human-written reference to a
range of visible text in a target document. SnipX is designed to work
against plain text, markup, and rich text formats by resolving snippets
over a canonical visible-text stream rather than over source markup,
style runs, or editor metadata.

```snipx
@profile plain-loose
@target <novel.txt>

[Alice]+ is Alice.
Alice a Character;
  hair "red";
  friend Bob.

~[Alice] ::"First visible mention of Alice.".
```

This repository contains the draft v0 language specification and the
Rust reference implementation: a reusable `snipx-core` crate and a thin
`snipx` command-line driver.

## Building

The workspace builds with stable Rust:

```bash
cargo build --release
cargo test --workspace
```

The binary is produced at `target/release/snipx`.

## Command-line usage

The CLI operates on the three SnipX input forms — commentaria
(standalone `.snipx` files), marginalia (prose-by-default note fields),
and intralinea (`{{ ... }}` blocks embedded in a target document):

```bash
# Format a commentaria file to stdout (or in place with --write)
snipx fmt notes.snipx

# Export canonical JSON, resolving snippets against a target document
snipx export notes.snipx --target chapter.txt --pretty

# Marginalia and intralinea input forms
snipx export --as marginalia notes.txt --target chapter.txt
snipx export --as intralinea chapter-annotated.txt
```

Input form is selected with `--as commentaria|marginalia|intralinea`
(short aliases `-c`, `-m`, `-i`). Matching behaviour is selected with
`--profile plain|plain-loose|markdown|markdown-loose`. `--ambient`
supplies an ambient subject for subjectless statements, e.g.
`--ambient '[]'` for the whole document.

Exit codes: `0` success, `1` completed with errors, `2` invalid usage,
`3` input/output failure, `4` unsupported option.

## Documentation

- [Language specification](docs/language-spec.md): the draft v0 SnipX
  language, including commentaria, marginalia, intralinea, snippets,
  statements, visible-text matching, denotations, scope, and deferred
  issues.
- [Canonical JSON](docs/canonical-json.md): the machine-readable export
  format produced by the CLI, including span offset conventions.

## Status

SnipX is pre-0.1. The language specification is a draft v0 document and
deliberately favours a small, hand-authorable core over implementation
completeness. The reference implementation covers parsing (lossless
CST), conservative formatting, statement expansion, plain text and
Markdown visible-text extraction, snippet resolution, and canonical
JSON export. The JSON schema and Rust API may still change before v0.1.

## Licence

Licensed under either of the [Apache License, Version
2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT) at your option.
