# Changelog

All notable changes to snipx are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Conformance corpus (`conformance/`): 80 cases defining conformance at
  the canonical-JSON export boundary, with the comparison contract in
  `MANIFEST.json` (structural comparison, diagnostic codes normative,
  message strings informative, `implementation` block excluded; ADR 0001).
  The Rust runner executes the corpus as part of the workspace test suite;
  `SNIPX_CONFORMANCE_REGEN=1` regenerates expectations for review.
- TypeScript implementation `@curvelogic/snipx` (`packages/snipx-ts`)
  scoped to canonical-JSON parity — parsing, expansion, visible-text
  extraction for all four profiles, resolution, and export — with explicit
  UTF-16/scalar/UTF-8 index maps and full conformance-corpus parity
  (ADR 0002). CI runs both implementations against the corpus.

- `snipx lint`: fragility diagnostics for resolved snippets, warning
  when anchors are likely to break or re-bind under target edits —
  `FRAGILE_SHORT_ANCHOR`, `FRAGILE_NEAR_DUPLICATE`, and
  `FRAGILE_CAPTURE_CONTEXT`. Warnings only; resolution results and
  exit codes are unchanged except under `--strict`. Codes are
  provisional pending ratification of ADR 0004
  (docs/adr/0004-fragility-diagnostics.md).

### Changed

- The language specification now carries a two-part version (currently
  0.1) with a changelog section; `snipxVersion` in canonical JSON
  output mirrors it, changing from `"0.0"` to `"0.1"`.
- Markdown visible text now parses GitHub-style footnotes and tables
  (ADR 0003). Footnote definition text is inserted at each reference
  point and no longer appears at the definition site; undefined
  references stay literal. Table rows are newline-delimited and cells
  space-separated. Documents using footnote or table syntax extract
  different visible text than before, so offsets over such documents
  shift.

### Fixed

- Text-span snippets (`~[...]`) now distribute one fact per matched
  span, as the spec's "Denotation And Text Spans" section requires
  (Cartesian product when both subject and object are text-span
  snippets). Each distributed `textSpanSnippet` fact value carries its
  concrete `span` in visible-text scalar offsets; quantified
  denotational snippets still collapse to a single fact.

## [0.1.1] - 2026-08-02

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

[Unreleased]: https://github.com/curvelogic/snipx/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/curvelogic/snipx/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/curvelogic/snipx/releases/tag/v0.1.0
