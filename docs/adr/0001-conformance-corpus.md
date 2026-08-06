# ADR 0001: Canonical-JSON conformance corpus

Status: Proposed (awaiting ratification by Greg)

## Context

Sift (the first embedding host, TypeScript end to end) needs a second
implementation of snipx held to the Rust reference's behaviour. The epic
(snipx-n9t) locked in two decisions: conformance is defined at the
`export_json` boundary (a case is an `ExportRequest`, the expectation is the
`ExportDocument`, compared structurally; the CST, formatter, and module shape
are not conformance material), and diagnostic codes are normative while
message strings are informative. This ADR records the concrete corpus design
built on those decisions.

## Decision

**Layout.** A top-level `conformance/` directory owned by this repo:

```
conformance/
  MANIFEST.json          corpus version, pinned spec version, case count,
                         machine-readable comparison contract
  README.md              contributor documentation
  cases/<area>/<slug>/   one directory per case
    request.json         the ExportRequest (camelCase; target text inline
                         or via "targetFile" relative to the case dir)
    expected.json        the expected ExportDocument
```

**Expected documents are stored without the `implementation` block.** The
comparison contract excludes it (otherwise every release invalidates the
corpus), and storing it would put churn in every case file on every version
bump. The Rust runner strips it from actual output before comparison and at
regeneration time.

**Diagnostic messages are stored but never compared.** Codes are normative;
messages are informative. Messages stay in `expected.json` because they make
cases reviewable by humans; the runner strips `message` fields (on
diagnostics and their `related` spans) from both sides before comparison, so
message drift can never fail conformance. Stale stored messages are refreshed
on regeneration.

**Array ordering.** `facts` and `resolutions` are compared
order-sensitively: their order mirrors statement order in the source, which
is meaningful. `diagnostics` are compared as an order-insensitive multiset
(keyed on code, severity, span, and related spans): emission order is an
implementation detail no consumer should rely on, and a second
implementation will interleave extraction/resolution diagnostics
differently. Object field order is insensitive throughout (structural JSON
comparison).

**Regeneration is explicit and reviewed.** `expected.json` is produced by
the Rust reference via `SNIPX_CONFORMANCE_REGEN=1 cargo test -p snipx-core
--test conformance` and must be human-reviewed before adoption; the runner
never writes expectations in normal runs. `MANIFEST.json` carries a
`caseCount` that the runner asserts against the discovered cases, so a case
directory accidentally dropped (or an empty checkout) fails loudly rather
than silently passing a smaller corpus.

**Versioning.** `MANIFEST.json` pins `specVersion` (currently `0.1`,
matching `SPEC_VERSION` in `snipx-core`) and carries its own
`corpusVersion` so corpus evolution can be tracked independently of spec
and implementation versions. The runner asserts the manifest's
`specVersion` matches the implementation's `SPEC_VERSION`, so a spec bump
forces a deliberate corpus review.

## Consequences

- Any implementation (Rust today, TypeScript next) proves parity by running
  the same `conformance/` tree; the contract is explicit in `MANIFEST.json`
  rather than implicit in one runner's code.
- Behaviour changes in the reference implementation surface as reviewable
  `expected.json` diffs in the same PR that changes behaviour.
- Diagnostics may be reordered freely by implementations without breaking
  conformance, but any change to a diagnostic *code* is a conformance
  break by construction — which is exactly the locked-in intent.
- The `implementation` block and message wording are formally
  non-conformance surface; tooling must not rely on them.
