# Canonical JSON

JSON is the canonical machine output of the reference implementation.
`snipx check`, `snipx lint`, `snipx resolve`, and `snipx export` emit
one JSON document on stdout; `--pretty` changes formatting only, never
the schema. The commands report nested slices of the same document:
`check` emits the envelope and `diagnostics` only, `lint` is `check`
plus fragility warnings in the same `diagnostics` array, `resolve`
adds `visibleText` and `resolutions`, and `export` adds `facts`. The
schema may still evolve, but this document describes the current
contract for downstream tools.

## Document shape

```json
{
  "snipxVersion": "0.1",
  "implementation": { "name": "snipx", "version": "0.1.0" },
  "input": { "form": "commentaria", "path": "notes.snipx" },
  "target": { "uri": "chapter.txt", "profile": "plain" },
  "visibleText": { "normalisation": "NFC", "length": 18422 },
  "facts": [
    {
      "subject": { "kind": "snippet", "source": "[Alice]" },
      "predicate": { "kind": "predicate", "value": "a" },
      "object": { "kind": "name", "value": "Character" },
      "source": {
        "statement": { "start": 0, "end": 20 },
        "subject": { "start": 0, "end": 7 },
        "predicate": { "start": 8, "end": 9 },
        "object": { "start": 10, "end": 19 }
      }
    }
  ],
  "resolutions": [
    {
      "source": "[Alice]",
      "sourceSpan": { "start": 0, "end": 7 },
      "spans": [{ "start": 0, "end": 5 }]
    }
  ],
  "diagnostics": [
    {
      "code": "SNIPPET_NOT_FOUND",
      "severity": "error",
      "message": "Snippet did not match: [Alice]",
      "span": { "start": 0, "end": 7 }
    }
  ]
}
```

- `snipxVersion` is the language-specification version this output
  conforms to, mirroring the version declared in
  [the specification](language-spec.md). It is independent of the
  implementation crate version reported under `implementation`.
- `facts` are expanded subject–predicate–object triples. Statements
  containing unresolved snippets still produce facts, carrying
  `unresolvedSnippet` values, rather than disappearing silently.
- `resolutions` records the matched text span(s) for each successfully
  resolved snippet value. Facts join to resolutions via the snippet
  `source` text and `sourceSpan`.
- `diagnostics` carry stable upper-snake codes and `error` or `warning`
  severity. Warnings do not affect the exit code unless `--strict` is
  supplied. The `FRAGILE_*` codes emitted by `snipx lint` are
  provisional pending ratification of
  [ADR 0004](adr/0004-fragility-diagnostics.md); they become part of
  the stable contract only on ratification.

## Value kinds

`subject`, `predicate`, and `object` are tagged unions on `kind`:
`name`, `predicate`, `string`, `number`, `boolean`, `uri`, `snippet`,
`textSpanSnippet`, `wholeDocument`, `unresolvedSnippet`, and
`unresolvedNumber`. Snippet kinds carry the snippet `source` text
(including any `~` sigil stripped into the kind and any quantifier);
the other kinds carry a decoded `value`.

A resolved `textSpanSnippet` value also carries the concrete matched
`span` it distributes over. Text-span snippets distribute: a statement
whose subject or object is a text-span snippet emits one fact per
matched span (the Cartesian product when both sides are text-span
snippets), each fact's value carrying its own `span`. Denotational
`snippet` values collapse to a single fact and never carry `span`.
`span` is omitted when resolution did not run (no target text).

## Span offset conventions

Two different offset units appear in the document. They share the
`{ "start": n, "end": n }` shape, and every span is half-open
`[start, end)`.

**Source spans are byte offsets.** `facts[].source.*`,
`resolutions[].sourceSpan`, and `diagnostics[].span` all address the
raw SnipX *input* text (the commentaria file, marginalia field, or
intralinea document) as UTF-8 byte offsets. They are suitable for
slicing the original input and for editor integrations that work in
bytes.

**Resolution spans are Unicode scalar offsets.** `resolutions[].spans`
and the `span` field on `textSpanSnippet` fact values address the
*canonical visible text* of the target document — after extraction and
NFC normalisation — counted in Unicode scalar values (Rust `char`s),
not bytes. `visibleText.length` is measured in the same
unit. UI and host integrations may map these positions to grapheme
clusters or native document positions.

Do not mix the two: a resolution span cannot be used to slice the SnipX
source, and a diagnostic span cannot be used to slice the target
document's visible text.
