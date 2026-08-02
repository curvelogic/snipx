# Changelog

All notable changes to snipx are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Snippet bodies are now lexed once, by the parser: the matcher consumes
  the structured CST instead of re-lexing strings. Diagnostics and JSON
  output are unchanged for valid documents; a few pathological inputs
  (embedded quotes mid-body, quoted braces in range endpoints) now follow
  the spec's "quotes delimit only when they wrap an entire body or
  endpoint" rule instead of the old string re-lexer's approximations.

## [0.1.0] - 2026-08-01

First public release of the SnipX reference implementation.

### Added

- Lossless parser for all three v0 input forms: commentaria,
  marginalia, and intralinea, with recoverable diagnostics.
- Conservative formatter that rewrites SnipX regions only and
  preserves host prose byte-for-byte (`snipx fmt`, `--write`).
- Statement expansion: Turtle-style `;`/`,` carry-forward, ambient
  subjects, `::"..."` decoration sugar, string escape decoding, and
  triple-string dedenting.
- Intralinea local subjects: sentence and paragraph scope markers
  (`<`, `>`, `<>`, `<<`, `>>`, `<<>>`, with `~` text-span variants)
  expand, anchor, and resolve against the stripped host text.
- Plain and Markdown visible-text extraction (`plain`, `plain-loose`,
  `markdown`, `markdown-loose`) with NFC normalisation and loose
  typographic matching.
- Snippet resolution: exact and loose matching, captures, closed and
  open ranges, quantifiers, and cardinality diagnostics.
- Commentaria `@target` and `@profile` directives, with command-line
  values taking precedence.
- Canonical JSON export with facts, resolutions, source-located
  diagnostics, and partial results for unresolved snippets;
  documented in docs/canonical-json.md.
- CLI commands `check`, `resolve`, `export` (nested slices of the
  canonical document) and `fmt`, with documented exit codes.
- Property tests, fuzz harness, snapshot suites, and CI.

[Unreleased]: https://github.com/curvelogic/snipx/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/curvelogic/snipx/releases/tag/v0.1.0
