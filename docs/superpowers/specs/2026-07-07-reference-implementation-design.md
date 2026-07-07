# Reference Implementation Design

This document specifies the planned Rust reference implementation for
SnipX. It is a planning/design artefact. Once the implementation exists,
this document should be superseded by normal project documentation under
`docs/`.

## Goal

Build a crate-first Rust reference implementation with a reusable
`snipx-core` library and a small `snipx` command-line driver. The first
priority is to make the whole language grammar explicit and testable, so
syntax inconsistencies surface before resolver and export behaviour
hardens.

## Architecture

The implementation should be a Rust workspace with two initial crates:

- `snipx-core`: the primary implementation surface.
- `snipx`: a thin CLI driver built on `snipx-core`.

`snipx-core` owns parsing, typed AST/query views, formatting, statement
expansion, visible-text extraction, snippet matching, resolution,
diagnostics, and canonical JSON data structures.

`snipx` owns command-line argument parsing, file and standard input/output
handling, exit codes, and rendering formatted text or JSON output.

Host-specific integrations, including Scrivener, RTF, DOCX, EPUB,
editor plugins, graph databases, and richer document source maps, are
outside the reference implementation. They should be separate tools or
crates built on top of `snipx-core`.

## Parser Contract

The parser must cover commentaria, marginalia, and intralinea input
forms from the first parser milestone. This is intentional: all three
forms exert pressure on the grammar, and the implementation should
discover syntax ambiguity early.

The parser contract is:

- use Rowan to produce a lossless concrete syntax tree;
- preserve comments, whitespace, fences, markers, and malformed regions;
- expose stable syntax kinds;
- recover from malformed input where practical;
- produce diagnostics with source ranges;
- provide typed AST/query views over the lossless tree.

The design does not prescribe lexer or parser mechanics. A separate
tokeniser, event parser, Pratt parser, or other hand-rolled approach is
an implementation choice as long as the Rowan and diagnostic contract is
met.

## Formatting

Formatting is part of v0 implementation, not a deferred extra. The core
crate should expose formatting APIs, and the CLI should expose:

```text
snipx fmt
```

The formatter must be conservative. It may rewrite SnipX syntax regions,
but it must preserve marginalia prose and intralinea host document text
byte-for-byte. For intralinea input, this means only the content inside
`{{ ... }}` blocks may be formatted. For marginalia input, only SnipX
fence contents and `///` SnipX lines may be formatted.

`snipx fmt` should default to writing formatted output to stdout. If
`--write` is included in v0, it must be explicit and covered by tests.

## CLI Surface

The initial CLI commands are:

```text
snipx check
snipx resolve
snipx export
snipx fmt
```

Input form is independent of command verb:

```text
--as commentaria    -c
--as marginalia     -m
--as intralinea     -i
```

The long form is preferred in documentation. The short flags are
convenience aliases. If both `--as` and a short input-form flag are
supplied, they must agree. Supplying more than one short input-form flag
is an error.

Initial options:

```text
--as <form>          input form: commentaria, marginalia, intralinea
-c                  alias for --as commentaria
-m                  alias for --as marginalia
-i                  alias for --as intralinea
--target <path-uri> target document for snippet resolution
--profile <name>    profile name; defaults are tool-defined
--ambient <expr>    ambient subject expression for subjectless statements
--pretty            pretty-print JSON output
--strict            treat warnings as errors
--write             write formatter output in place, if implemented in v0
```

The command-line driver should support only local files and standard
input/output in v0. Network fetching and host-specific project loading
are outside the reference implementation.

## Core API Shape

The Rust API should be crate-first and option-object based rather than a
large collection of loosely related free functions. The exact names may
evolve during pre-0.1 implementation, but the public surface should make
these operations straightforward:

- parse SnipX input in a selected form;
- format SnipX input conservatively;
- expand statements and sugar;
- extract visible text from supported target profiles;
- resolve snippets against visible text;
- export canonical JSON with facts, resolutions, diagnostics, and
  provenance.

The API is an important deliverable, but semver stability is not
promised before v0.1.

Dependencies are advisory at design time. Likely crates include `rowan`,
`serde`, `serde_json`, `clap`, `unicode-normalization`, `insta`, and
later `pulldown-cmark` for Markdown visible-text extraction.

## Input Forms

Commentaria input is SnipX-by-default. The input may contain `@target`
and `@profile` directives. Command-line `--target` and `--profile`
values override or supply directive values according to CLI policy; JSON
output reports the effective target and profile.

Marginalia input is prose-by-default. The CLI parses unlabelled fences,
`snipx` fences, and `///` single-line SnipX entries. Prose outside SnipX
blocks is preserved for source mapping and formatting purposes, but v0
export does not automatically convert prose to `note` facts unless a
later profile explicitly requests it. The CLI may receive an ambient
subject with `--ambient`.

Intralinea input is a target document containing `{{ ... }}` blocks. The
CLI removes those blocks from the canonical visible-text stream before
resolving snippets. The stripped visible text is the target context for
explicit snippets and local intralinea subjects.

## Milestones

### 1. Workspace And CI

Create the Rust workspace, initial crates, and comprehensive GitHub
Actions CI. CI should run at least:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --all-features`;
- `cargo test --workspace --all-features`.

### 2. Full Syntax Parser

Implement the lossless Rowan parser for commentaria, marginalia, and
intralinea. Parser coverage includes snippets, quoted snippets,
captures, ranges, quantifiers, directives, statements, identifiers,
natural-language predicates, URI literals, strings, triple strings,
decorations, comments, fences, `///` lines, intralinea blocks, and local
subject markers.

### 3. AST/Query Layer And Formatter

Add typed views over the CST for language constructs needed by later
passes. Add conservative formatting in the core crate and expose
`snipx fmt` in the CLI.

### 4. Expansion And Diagnostics

Implement statement expansion for `.`, `;`, `,`, ambient subjects, and
`::` decoration sugar. Diagnostics must have stable codes, severity,
message, source location, and optional related spans.

### 5. Plain Text Extraction And Resolution

Implement `plain` and `plain-loose` visible-text extraction and matching.
Resolution must cover exact and loose matching, ranges, open ranges,
captures, quantifiers, ambiguity, zero matches, and unresolved snippets.

### 6. Canonical JSON And CLI Hardening

Implement `check`, `resolve`, and `export` over the parser, expansion,
resolution, and JSON model. Export is partial and diagnostic-rich:
statements containing unresolved snippets still produce facts where
possible, carrying unresolved snippet values and diagnostics rather than
disappearing silently.

### 7. Markdown Extraction

Add `markdown` and `markdown-loose` visible-text extraction after the
plain text path is working. Markdown extraction should target rendered
visible prose: headings, block quotes, list item text, code block text,
inline code text, link text, and image alt text are visible; link and
image destinations, reference definitions, and raw HTML tags are not
visible.

### 8. Beads Issue Structure

Represent implementation work in Beads. Create parent epics for the
major milestones and child task Beads for every executable task in the
implementation plan. The Superpowers implementation plan remains useful
for agent execution; Beads is the durable project task state.

## Canonical JSON

JSON is the only canonical machine output for v0. `--pretty` changes
formatting only; it does not change the schema.

Other exports, including RDF, JSON-LD, Cypher, YAML, or graph-database
loaders, are outside the reference implementation. They may be built as
separate tools consuming canonical SnipX JSON.

The JSON output should include:

- SnipX language version;
- implementation version;
- input form;
- effective profile;
- target URI or path, if any;
- canonical visible-text metadata;
- resolved facts;
- unresolved facts or values when resolution fails;
- resolved snippets and spans when relevant;
- diagnostics;
- source locations for statements, snippets, and generated facts.

A minimal shape:

```json
{
  "snipxVersion": "0.0",
  "implementation": {
    "name": "snipx",
    "version": "0.0.0"
  },
  "input": {
    "form": "commentaria",
    "path": "notes.snipx"
  },
  "target": {
    "uri": "novel.md",
    "profile": "plain-loose"
  },
  "visibleText": {
    "normalisation": "NFC",
    "length": 18422
  },
  "facts": [],
  "resolutions": [],
  "diagnostics": []
}
```

The exact schema may evolve during pre-0.1 implementation, but v0 should
keep JSON as the compatibility contract for other tools.

## Diagnostics And Exit Codes

Diagnostics should have stable codes, severity, message, source
location, and optional related spans. They should be suitable for both
human display and JSON output.

Initial exit codes:

```text
0 success
1 completed with errors
2 invalid command-line usage
3 input/output failure
4 unsupported profile, input form, or output option
```

Warnings do not affect the exit code unless `--strict` is supplied.

## Testing Strategy

Testing should be heavy around syntax and formatting because the
lossless parser is the early contract.

Required test groups:

- parser fixtures for commentaria, marginalia, and intralinea;
- formatter fixtures proving host prose and host document text are
  preserved byte-for-byte outside SnipX regions;
- expansion tests for `;`, `,`, ambient subjects, and `::`;
- resolver tests for exact matching, loose matching, captures, ranges,
  open ranges, quantifiers, ambiguity, and missing matches;
- JSON snapshot tests for `check`, `resolve`, and `export`;
- CLI integration tests for commands, flags, exit codes, stdout/stderr,
  and invalid form flag combinations;
- Markdown extraction fixtures once milestone 7 begins.

Property-style parser tests should be included once the parser has a
stable enough surface. Initial properties:

- parsing arbitrary text in any input form must not panic;
- formatting valid SnipX regions must produce parseable SnipX;
- parse-format-parse should preserve the typed syntax meaning for
  supported constructs;
- formatting must not change marginalia prose or intralinea host text
  outside SnipX regions.

Fuzz testing should be added for the parser and formatter when the basic
fixture suite is in place. Fuzz failures should become regression
fixtures.

## Non-Goals

The reference implementation does not include:

- Scrivener, RTF, DOCX, PDF, EPUB, or HTML extraction;
- network fetching;
- editor plugins;
- graph database output;
- RDF, JSON-LD, Cypher, YAML, or OWL-style inference;
- host-specific fact scope or name-scope policy beyond accepting caller
  configuration.
