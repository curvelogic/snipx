# snipx conformance corpus

This corpus defines conformance for snipx implementations at the
`export_json` boundary: a case is an `ExportRequest`, the expectation is
the resulting `ExportDocument`, compared structurally. Nothing about the
CST, formatter, or module shape is conformance material. See
`docs/adr/0001-conformance-corpus.md` for the design rationale.

## Layout

Each case lives in `cases/<area>/<slug>/`:

- `request.json` — the export request:
  - `source` (string, required): the snipx source text.
  - `inputForm` (required): `"commentaria"`, `"marginalia"`, or
    `"intralinea"`.
  - `targetText` (string) or `targetFile` (path relative to the case
    directory): the target document, if any.
  - `profile` (optional): `"plain"`, `"plain-loose"`, `"markdown"`, or
    `"markdown-loose"`. When absent, a commentaria `@profile` directive is
    honoured, falling back to `plain`.
  - `path`, `targetUri` (optional strings).
  - `ambientSubject` (optional): tagged value, e.g.
    `{"kind": "name", "value": "doc"}`; kinds `name`, `string`, `uri`,
    `number`, `boolean`.
- `expected.json` — the expected `ExportDocument`, stored **without** the
  `implementation` block.

## Comparison contract

Declared machine-readably in `MANIFEST.json`:

- Structural JSON comparison; object field order never matters.
- The `implementation` block is excluded entirely.
- Diagnostic **codes** are normative and compared; **message** strings
  (including `related[].message`) are informative and never compared. They
  are stored only to keep cases human-reviewable.
- `facts` and `resolutions` are order-sensitive; `diagnostics` are compared
  as an order-insensitive multiset.
- `MANIFEST.json`'s `caseCount` must equal the number of case directories;
  runners assert this so silently missing cases fail loudly.

## Running and regenerating

The Rust reference runner is `crates/snipx-core/tests/conformance.rs` and
runs with the ordinary workspace test suite:

```
cargo test -p snipx-core --test conformance
```

To regenerate all `expected.json` files from the current Rust reference
(required after intentional behaviour changes; diffs must be human-reviewed
before adoption):

```
SNIPX_CONFORMANCE_REGEN=1 cargo test -p snipx-core --test conformance
```

After adding or removing cases, update `caseCount` in `MANIFEST.json`.
