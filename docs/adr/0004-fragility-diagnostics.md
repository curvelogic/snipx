# ADR 0004: Fragility diagnostics for snippets

## Status

Proposed (awaiting ratification by Greg).

The diagnostic codes named below are **provisional**. They surface in
the canonical JSON `diagnostics` array, but they only become normative
(covered by the canonical-JSON stability contract) on ratification of
this ADR. Until then they may be renamed, retuned, or removed without a
spec version bump.

## Context

The language spec defers "optional diagnostics for fragile snippets"
(docs/language-spec.md, deferred features). Fragility under document
edits is the main adoption risk for SnipX: snippet anchors are exact
text matches against the target's visible text, so an edit to the
target can silently break a snippet (`SNIPPET_NOT_FOUND`), silently
re-bind it to a different occurrence, or make it ambiguous
(`SNIPPET_AMBIGUOUS`) — and v0 has no ordinal syntax to disambiguate.
Those failures appear only *after* the edit; nothing warns the author
at annotation time that an anchor is likely to break.

The reference implementation already has the machinery to detect the
risky cases cheaply, purely from already-resolved data:

- the matcher normalises both needle and haystack (NFC always; loose
  profiles additionally fold whitespace runs, typographic dashes and
  quotes, and ligatures — see `crates/snipx-core/src/match.rs`), so
  "would this snippet gain matches under loose normalisation?" is a
  single extra match pass;
- resolutions record every matched span, so occurrence counts are
  available;
- snippet structure (pattern parts, captures, range endpoints) is
  preserved on the resolved statements.

Existing diagnostics carry SCREAMING_SNAKE codes and `error` or
`warning` severity in the canonical JSON `diagnostics` array; warnings
never affect the exit code unless `--strict` is passed. That is the
surface fragility diagnostics should reuse.

## Decision

Add a **lint-only** fragility analysis with a deliberately small
initial set of three warnings. The analysis is a pure function over
already-resolved data (`crates/snipx-core/src/fragility.rs`); it never
changes resolution results, facts, or resolutions.

### Diagnostic set (provisional codes)

**`FRAGILE_SHORT_ANCHOR`** — a resolved snippet's anchor text is
shorter than **5 Unicode scalar values** after normalisation. Short
anchors are weak: they are the most likely to collide with new text
introduced by edits, and the least likely to survive rewording. The
unit is Unicode scalar values of the normalised needle (the same unit
as resolution spans and `visibleText.length`), never bytes. The
threshold 5 is chosen so that a typical word ("Alice") passes while
fragments shorter than a typical word ("Bob", "the") warn; it is a
tuning constant (`fragility::SHORT_ANCHOR_THRESHOLD`), not a spec
value. For range snippets each non-empty endpoint is an independent
anchor and is checked separately; empty needles (whole-document and
open range endpoints) are exempt because they cannot dangle.

**`FRAGILE_NEAR_DUPLICATE`** — a snippet resolved under a strict
profile (`plain`, `markdown`) matches **more** spans when re-matched
under the loose variant of the same profile (`plain-loose`,
`markdown-loose`). The extra loose matches are near-duplicates of the
anchor that differ only by whitespace runs, typographic punctuation,
or ligatures — exactly the classes of text that routine editing
(smart-quote conversion, re-wrapping, copy-editing) changes. Such an
edit would flip the snippet to a different occurrence or make it
ambiguous. Snippets resolved under a loose profile cannot warn (the
comparison is a no-op by construction); that is intended — loose
resolution has already absorbed the variation.

**`FRAGILE_CAPTURE_CONTEXT`** — for a snippet with a capture, the
context on either side of the capture occurs at more positions in the
target than the snippet has resolved spans. The context is the
snippet's real anchor (the capture text is expected to vary), so
context that also occurs elsewhere means an edit near those other
positions can create a competing match and re-bind or ambiguate the
capture. Each non-empty side (prefix, suffix) is counted independently
against the normalised visible text under the resolution profile.

Candidates considered and deferred: ordinal-style "match is not the
first occurrence" hints (needs ordinal syntax first), edit-distance
near-miss scanning (quadratic, not cheap), and cross-document anchor
stability (out of scope for a single-target pipeline).

### Severity and effect

All three are `warning` severity, always — never errors. They must not
change resolution results, facts, resolutions, or exit codes. The
existing `--strict` flag already promotes warnings to exit code 1;
fragility warnings participate in that mechanism and get no dedicated
flag.

### Where they surface

In the canonical JSON `diagnostics` array (code, severity, message,
source span of the snippet), the same channel as every other
diagnostic — not on stderr. They are emitted only by the new CLI
subcommand **`snipx lint`**, which behaves exactly like `snipx check`
(same arguments, same envelope-plus-`diagnostics` output view, same
exit codes) plus the fragility analysis. `check`, `resolve`, and
`export` are unchanged, so existing `--strict` pipelines do not start
failing on fragile-but-valid documents.

## Consequences

- Authors get edit-fragility feedback at annotation time via
  `snipx lint`, with `--strict` available for CI enforcement, at the
  cost of one extra subcommand to document.
- The analysis adds at most a few extra match passes per resolved
  snippet (one loose re-match, one count per capture context side);
  cost is proportional to the existing resolution work.
- Because the codes ride in the canonical `diagnostics` array,
  downstream tools see them with zero schema change; the provisional
  status above governs their stability until ratification.
- Both thresholded checks are heuristics: `FRAGILE_SHORT_ANCHOR` will
  flag some perfectly unique short anchors, and unusual documents can
  evade `FRAGILE_NEAR_DUPLICATE` (edits can always invent duplicates
  the lint cannot foresee). Lint-only, warning-only placement keeps
  the false-positive cost low.
- The three checks are independent, so the set can grow (or shrink)
  code by code without reworking the surface.
