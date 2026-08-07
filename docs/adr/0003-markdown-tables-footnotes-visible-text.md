# ADR 0003: Markdown tables and footnotes in visible text

Status: Proposed (awaiting ratification by Greg)

## Context

The Markdown extractor built `Parser::new(source)` with no
pulldown-cmark options, so footnote and table syntax was never parsed:
footnote markers and definitions passed through as literal text, table
rows were emitted as raw `| a | b |` lines, and the `TableRow` match
arms in `visible_text.rs` were dead code.

The spec's visible-text model (docs/language-spec.md, "Visible Text
Model") says visible text is what a reader sees: it "includes all
visible document content", and "footnote and endnote text is inserted
at its reference point in the canonical stream".

## Decision

### Footnotes

Enable `ENABLE_FOOTNOTES` (GitHub-style footnotes) and inline
definitions at their reference points:

- The definition's extracted text is inserted at the reference point,
  delimited by newlines like every other block boundary. Multi-block
  definitions keep their internal newline separation.
- The definition block itself emits nothing at its definition site;
  unreferenced definitions are omitted entirely.
- Multiple references to the same footnote insert the definition text
  at every reference point.
- Undefined references are not footnote syntax under GitHub-style
  parsing; their characters (`[^name]`) remain literal visible text,
  exactly as before this change. No diagnostic is emitted.
- Duplicate definitions: the first definition of a label wins; later
  duplicates are ignored (mirroring reference-resolution behaviour for
  link definitions).
- Cyclic references (a footnote that transitively references itself)
  cannot re-inline the footnote currently being inserted; the inner
  reference contributes nothing. This bounds recursion.

### Tables

Enable `ENABLE_TABLES` and emit table content: a reader sees table
text, so excluding it would contradict the visible-text philosophy the
same way excluding headings would.

- Rows (including the header row) are newline-delimited, consistent
  with every other block boundary in the extractor.
- Cells within a row are separated by a single space; empty cells do
  not produce doubled separators.

The formerly dead `TableRow` arms are now live, joined by `Table`,
`TableHead`, and `TableCell` handling.

## Consequences

- Snippets can now match footnote and table prose, and offsets over
  documents containing that syntax shift relative to the old literal
  pass-through. Visible-text spans remain Unicode-scalar offsets into
  the extracted stream; source-located diagnostics (e.g. raw HTML
  inside a footnote definition) keep UTF-8 byte offsets into the
  original source because event ranges are carried through inlining.
- Inlining moves footnote text away from its source position, so a
  phrase that spans a reference point (e.g. "Alice went" around
  `Alice[^1] went`) no longer matches contiguously; this is inherent
  to the spec's insertion model.
- Both Markdown profiles (`markdown`, `markdown-loose`) extract
  identical text; plain profiles are unaffected.
