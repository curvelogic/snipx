# ADR 0002: TypeScript implementation (snipx-ts)

Status: Proposed (awaiting ratification by Greg)

## Context

Sift, the first embedding host, is TypeScript end to end, so snipx needs
a second implementation that runs natively in that world. The epic
(snipx-x5i) scopes it to canonical-JSON parity only: parse, expand,
visible-text extraction (all four profiles), resolution, and
`export_json` equivalence, proven by running the same `conformance/`
corpus as the Rust runner. The formatter, CST fidelity guarantees,
lossless trees, and a CLI are explicitly out of scope.

The central design problem is offsets. JavaScript strings are UTF-16,
but the canonical JSON contract (mirroring the Rust reference) uses
UTF-8 byte offsets for source spans and Unicode scalar offsets for
visible-text spans. Any implementation that lets raw `.length` or
`.indexOf` values reach the export boundary will pass on ASCII and fail
on the corpus's `offsets/` cases (astral plane, combining sequences,
ligature folding), which exist precisely to catch that.

## Decision

**Package.** A pnpm workspace package at `packages/snipx-ts`, named
`@curvelogic/snipx`, beside `crates/`. `pnpm-workspace.yaml` and a
minimal private root `package.json` (carrying the pinned
`packageManager`) live at the repo root. TypeScript strict mode
(including `noUncheckedIndexedAccess` and
`exactOptionalPropertyTypes`); vitest for tests; the only runtime
dependency is a CommonMark parser.

**Layout mirrors the Rust crate** so cross-implementation review is a
file-by-file diff of semantics, not archaeology:

| snipx-ts module   | Rust counterpart            |
| ----------------- | --------------------------- |
| `indexMaps.ts`    | (implicit in Rust's `&str`) |
| `syntax.ts`       | `syntax.rs` (rowan tree)    |
| `parser.ts`       | `parser.rs`                 |
| `ast.ts`          | `ast.rs`                    |
| `snippet.ts`      | `snippet.rs`                |
| `expand.ts`       | `expand.rs`                 |
| `match.ts`        | `match.rs`                  |
| `visibleText.ts`  | `visible_text.rs`           |
| `resolve.ts`      | `resolve.rs`                |
| `export.ts`       | `json.rs`                   |

`export.ts` exports `SPEC_VERSION = "0.1"`, mirroring the Rust
`SPEC_VERSION`; the conformance runner asserts it against the corpus
manifest exactly as the Rust runner does.

**Three-way index maps.** `indexMaps.ts` is the first-class answer to
the UTF-16 problem: for a given string, `buildIndexMap` precomputes O(1)
conversions between UTF-16 code-unit indices, Unicode scalar indices,
and UTF-8 byte offsets. The discipline it enforces:

- The parser and syntax tree work in UTF-16 code units internally (the
  Rust original works in UTF-8 bytes; both advance by whole code
  points, so the algorithms are unit-agnostic).
- The matcher and local-subject resolver work over arrays of code
  points, so every visible-text span is born as a Unicode scalar offset
  and needs no conversion.
- `export.ts` is the single conversion boundary: fact spans, resolution
  source spans, and diagnostic spans are converted UTF-16 → UTF-8 bytes
  through the source text's map; `RAW_HTML_OMITTED` spans through the
  target text's map; visible-text spans pass through unchanged.

No span may cross into JSON output without going through a map.

**CommonMark engine: commonmark.js.** The Rust reference pins
pulldown-cmark 0.13 with no extensions enabled, i.e. pure CommonMark.
Candidates were commonmark.js and micromark. commonmark.js is chosen
because:

- It is the CommonMark reference implementation, so on spec-conformant
  input its parse agrees with pulldown-cmark by construction; both
  target the same spec version with no extensions.
- Its AST walker maps one-to-one onto the enumerated visible-text
  rules: text/code literals arrive already entity-decoded (as
  pulldown's `Text`/`Code` events do), and block boundaries (paragraph,
  heading, block quote, code block, list item) are container
  enter/leave events.
- The only positions the extraction needs are for raw-HTML warnings,
  and commonmark.js records `sourcepos` for both block and inline HTML
  nodes; block HTML is split per source line to match pulldown-cmark's
  per-line `Html` events, each span running to the next line start.
- micromark's token stream has excellent offsets but reconstructing
  *decoded* visible text from its low-level tokens (character
  references, escapes, flow chunking) re-implements a layer
  commonmark.js already provides, adding fidelity risk exactly where
  parity matters.

**Divergence-as-spec-ruling policy.** Where the chosen engine genuinely
diverges from pulldown-cmark 0.13 on a corpus case, the implementation
must not fudge output to match. The divergence is documented precisely
in the PR as a proposed spec ruling, and the case goes into the
runner's explicit skip-list with a comment referencing the divergence.
An empty skip-list is the goal and the current state; the skip-list is
a visible debt register, not an escape hatch.

**Licence.** `"MIT OR Apache-2.0"`, matching the Rust workspace.

## Consequences

- Every PR runs both implementations against the same 80-case corpus in
  CI (Rust job untouched; a new Node 22 + pnpm job typechecks, runs
  unit tests, and runs the TS conformance runner).
- The index maps are unit-tested directly (astral, combining,
  three-byte BMP), as is the loose folding table, because those are the
  places where a silent UTF-16 bug would otherwise hide behind ASCII
  corpora.
- Consumers get a library API (`exportJson`) that produces canonical
  JSON structurally identical to the Rust CLI's, with the
  `implementation` block naming `snipx-ts`.
- The parity bar is the corpus, not the module internals: refactors on
  either side are safe while the corpus passes, which is the point of
  defining conformance at the `export_json` boundary (ADR 0001).
- Future spec version bumps must update `SPEC_VERSION` in both
  implementations and the corpus manifest together.
