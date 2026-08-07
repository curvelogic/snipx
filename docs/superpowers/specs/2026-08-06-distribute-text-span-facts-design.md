# Distribute Quantified Text-Span Facts Per Matched Span

**Issue:** snipx-2x0
**Spec basis:** language-spec.md, "Denotation And Text Spans": quantified
denotational snippets collapse to one fact; quantified text-span
snippets distribute one fact per matching span
(`~[Alice]+ highlight true.` → one fact per span).

## Problem

The implementation emits one fact total for a quantified text-span
snippet. The matched spans are recoverable only by joining against the
`resolutions` array. This is a spec non-conformance.

## Behaviour

- When a text-span snippet (`~[...]`) in subject or object position
  resolves successfully, the statement is replicated once per matched
  span. Each replica's text-span value carries the concrete span it
  refers to.
- When both subject and object are text-span snippets, distribution is
  the Cartesian product: 2 subject spans × 3 object spans → 6 facts.
- Zero matches for `~[x]*` or `~[x]?` produce zero facts: the statement
  drops out without a diagnostic, consistent with existing cardinality
  rules (`+` and bare snippets already error on zero matches before
  distribution is reached).
- The rule is uniform, not special-cased on quantifiers: an unquantified
  `~[Alice]` resolves to exactly one span and produces one fact as
  today, now carrying its span inline.
- Denotational snippets are unchanged: quantified or not, they collapse
  to a single fact with no span on the value.
- The `resolutions` array is unchanged: one entry per snippet occurrence
  listing all matched spans.
- Decoration facts (`::"note"`) whose subject is a text-span snippet
  distribute the same way — they are ordinary expanded statements.
- The no-target-text path is unchanged: text-span values stay
  span-less and one fact is emitted, as today.

### Out of scope

- Text-span local subjects (`~<`, `~<>`, …) resolve to exactly one span
  by construction; their fact values stay span-less. Adding inline spans
  there for uniformity is a possible follow-up.

## JSON representation

`textSpanSnippet` values in `facts` gain an optional `span` field in
visible-text scalar offsets (the same unit as `resolutions[].spans`):

```json
{
  "subject": {
    "kind": "textSpanSnippet",
    "source": "[Alice]+",
    "span": { "start": 12, "end": 17 }
  },
  "predicate": { "kind": "predicate", "value": "highlight" },
  "object": { "kind": "boolean", "value": true }
}
```

`span` is omitted when resolution did not run (no target text), so
existing output for that path is byte-identical.

## Implementation

- **`expand.rs`** — add a post-resolution variant
  `Value::ResolvedTextSpan { snippet: SnippetValue, span: TextSpan }`.
  `expand` itself never produces it.
- **`resolve.rs`** — rework the per-statement loop to rebuild the
  statements vector. Resolving a value reports whether it is a
  distributing text-span snippet and, if so, its matched spans; the
  loop then emits one statement per element of the subject × object
  span product (a non-distributing side contributes exactly its
  original value). Cardinality diagnostics and `Unresolved` fallbacks
  are unchanged and short-circuit distribution.
- **`json.rs`** — `JsonValue::TextSpanSnippet { source, span: Option<JsonSpan> }`
  with `skip_serializing_if`; map `Value::ResolvedTextSpan` to it.
- **`docs/canonical-json.md`** — document the `span` field, its offset
  unit, and the distribution rule.
- **`CHANGELOG.md`** — entry under Unreleased.

## Testing

TDD throughout:

- `resolution.rs`: quantified subject distributes (one statement per
  span); Cartesian product when both sides distribute; zero-match `*`
  drops the statement; unquantified `~[x]` carries its span;
  denotational quantified snippet still collapses; decoration on a
  distributed subject.
- `json_snapshots.rs`: snapshot covering a distributed export with
  `span` fields.
- `cli.rs`: end-to-end check that `~[Alice]+` emits one fact per span.
