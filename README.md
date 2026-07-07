# SnipX

SnipX is a small text annotation language for writing structured and
semi-structured notes against fiction and other prose documents.

The central primitive is the **snippet**: a human-written reference to a
range of visible text in a target document. SnipX is designed to work
against plain text, markup, and rich text formats by resolving snippets
over a canonical visible-text stream rather than over source markup,
style runs, or editor metadata.

This repository currently contains the draft language specification and
planning material for a Rust reference implementation.

## Documentation

- [Language specification](docs/language-spec.md): the draft v0 SnipX
  language, including commentaria, marginalia, intralinea, snippets,
  statements, visible-text matching, denotations, scope, and deferred
  issues.
- [Reference implementation design](docs/superpowers/specs/2026-07-07-reference-implementation-design.md):
  the current planning/design artefact for a crate-first Rust reference
  implementation.

The reference implementation design is not intended to be permanent
user-facing documentation. Once the implementation exists, it should be
replaced by normal project documentation under `docs/`.

## Status

SnipX is pre-implementation. The language specification is a draft v0
document and deliberately favours a small, hand-authorable core over
implementation completeness.
