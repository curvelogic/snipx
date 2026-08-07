# @curvelogic/snipx

TypeScript implementation of [SnipX](../../README.md), the text annotation
language, scoped to canonical-JSON parity with the Rust reference
implementation: parsing (commentaria, marginalia, intralinea), statement
expansion, visible-text extraction (`plain`, `plain-loose`, `markdown`,
`markdown-loose`), snippet resolution, and canonical JSON export. The
formatter, lossless CST access, and CLI are Rust-only.

Conformance is proven by running the repository's shared
[conformance corpus](../../conformance/README.md) — the same cases the Rust
implementation runs — under the contract in `conformance/MANIFEST.json`.

## Offsets

JavaScript strings are UTF-16, but the canonical JSON contract uses Unicode
scalar offsets for visible-text spans and UTF-8 byte offsets for source
spans. All spans crossing the export boundary go through explicit index
maps (`src/indexMaps.ts`) converting between UTF-16 code units, Unicode
scalars, and UTF-8 bytes. See
[ADR 0002](../../docs/adr/0002-typescript-implementation.md) for the
architecture.

## Develop

```bash
pnpm install          # from the repository root
pnpm exec tsc --noEmit
pnpm test             # unit tests + conformance corpus
```

## Licence

MIT OR Apache-2.0, matching the Rust crates.
