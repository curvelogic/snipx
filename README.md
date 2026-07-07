# SnipX

SnipX is a small text annotation language for writing structured and
semi-structured notes against fiction and other prose documents.

The central primitive is the **snippet**: a human-written reference to a
range of visible text in a target document. SnipX is designed to work
against plain text, markup, and rich text formats by resolving snippets
over a canonical visible-text stream rather than over source markup,
style runs, or editor metadata.

This document is a draft v0 specification. It deliberately favours a
small, hand-authorable core over implementation completeness.

## Variants

SnipX is the umbrella language. v0 defines three usage variants.

### Commentaria

Commentaria is the standalone `.snipx` form. Commentaria files are
SnipX-by-default and may target a document:

```snipx
@profile rtf-loose
@target <novel.rtf>

[Alice]+ is Alice.
Alice a Character;
  hair "red";
  friend Bob.

~[Alice] ::"First visible mention of Alice.".
```

`@target` selects the document context for snippet resolution. It does
not create an ambient subject. Whole-document statements must use `[]`
explicitly.

### Marginalia

Marginalia are host-provided note fields attached to a document,
chapter, selection, or other text-bearing context. Marginalia are
prose-by-default.
Unlabelled Markdown-style code fences, and fences labelled `snipx`, are
parsed as embedded SnipX blocks:

````text
Alice feels evasive in this scene.

```
[Alice] mood "guarded".
```

```js
console.log("not snipx");
```
````

For single-line SnipX inside prose-default marginalia, a line whose
first non-whitespace characters are `///` is parsed as one embedded
SnipX line:

```text
Alice seems evasive here.

/// [Alice] mood "guarded".
/// [door] motif Threshold.
```

The `///` marker is not part of the SnipX. It is only a marginalia
embedding marker, not a comment marker in commentaria files.

The host supplies an **ambient subject** for subjectless SnipX
statements. The ambient subject may be the whole document, a chapter, a
selection, or another host-defined target. The host also defines the
snippet resolution context for snippets inside the marginalia field;
this may be the same as, wider than, or narrower than the ambient
subject.

Treatment of prose outside SnipX fences is profile/tool-defined. A tool
may preserve it as prose, convert it to `note` facts on the ambient
subject, or ignore it during structured extraction.

### Intralinea

Intralinea, or simply "inlines", are SnipX annotations embedded directly
in the target document using `{{ ... }}` blocks:

```text
Alice promised to return before dawn. {{< a Promise}}
```

Intralinea blocks are annotation syntax, not part of the target text. They
are excluded from the canonical visible-text stream before snippets are
resolved, either by removal or by being hidden through features of the
tool hosting the text. Rich-text styling inside an intralinea block is
ignored; the block's visible characters are concatenated and parsed as
SnipX.

An intralinea block may contain full SnipX statements:

```text
Alice opened the door. {{ [Alice] a Character. [door] motif Threshold. }}
```

Embedded blocks may omit the final `.` when they contain a single final
statement. Standalone `.snipx` files require statement terminators.

## Visible Text Model

Snippets resolve against a canonical visible-text stream.

In v0:

- Visible text includes all visible document content, including
  headings, footnotes, and endnotes.
- Footnote and endnote text is inserted at its reference point in the
  canonical stream.
- Style and markup boundaries are ignored.
- Non-text objects are ignored. Captions count only because they are
  visible text.
- Editor apparatus such as comments, annotations, Scrivener notes,
  inspector synopsis text, and tracked-change UI is excluded by
  default.

Format profiles lightly define extraction behaviour:

- `plain`: decoded text content.
- `markdown`: rendered visible prose; markup is not target text.
- `html`: rendered text; hidden content, scripts, and styles excluded.
- `rtf`: visible body text; comments and editor annotations excluded.
- `scrivener`: manuscript text included; notes, synopsis, and inspector
  fields excluded unless a tool profile says otherwise.

Profile names are bare lowercase/dashed identifiers. A single
`@profile` directive selects both extraction and matching behaviour:

```snipx
@profile loose
@profile rtf-loose
@profile scrivener-loose
```

Directives are line-oriented, do not require periods, and must appear in
the header before the first statement. Specific tools may apply default
profiles when not specified.

## Snippets

A snippet is enclosed in square brackets.

```snipx
[Alice]
[The quick..jumped]
[..the end]
[Once upon..]
[]
```

### Basic Resolution

`[text]` resolves to the visible-text range whose text exactly matches
`text`.

`[start..end]` resolves to the inclusive range beginning with `start`
and ending with the first valid `end` after that selected `start`.
Endpoint strings are included in the resolved range.

Open-ended ranges are inclusive:

- `[..end]` means from document start through `end`.
- `[start..]` means from `start` through document end.
- `[]` always means the whole visible document.

Range matching is directional. For `[A..B]`, `B` is searched only after
the selected occurrence of `A`.

By default, a snippet must resolve to exactly one range. Zero matches or
multiple matches are resolution errors.

### Quantifiers

Snippet quantifiers follow regular-expression cardinality:

- `[x]` means exactly one match.
- `[x]+` means one or more matches.
- `[x]*` means zero or more matches.
- `[x]?` means zero or one match.

Quantified results are semantically unordered collections of ranges,
though tools may display them in document order. Matching is
non-overlapping and leftmost-first.

Zero matches for `*` are valid. Tools may warn in lint or strict modes.

### Matching Profiles

The default matching profile is exact after Unicode normalisation.

Exact matching:

- is case-sensitive;
- preserves whitespace significance;
- matches substrings, not whole words;
- does not imply word boundaries;
- normalises Unicode before comparison, using NFC in v0.

The initial loose profile:

- collapses all whitespace runs to one logical space, including tabs and
  newlines;
- normalises common typographic variants such as smart quotes, curly
  apostrophes, dashes, and ligatures;
- remains case-sensitive unless a later profile says otherwise.

Per-snippet profile overrides are deferred from v0.

### Quoted Snippet Text

Quoted snippet bodies allow literal syntax characters such as `[`, `]`,
and `..`:

```snipx
["[sic]"]
["[start]".."[end]"]
```

Quoted snippet bodies are very literal. Only the quote delimiter itself
needs escaping. This rule is intentionally different from ordinary
SnipX strings.

### Captures And Context

A snippet may include one captured range using `{...}`. Text outside the
capture participates in matching but is not part of the resolved target.

```snipx
[looked at {Alice}]
[{Alice} looked back]
[looked at {Alice} and smiled]
```

Rules:

- A snippet without `{}` captures the whole matched snippet.
- A snippet with one `{...}` matches the whole snippet body but resolves
  only the braced text.
- v0 allows at most one capture per snippet.
- Captures are not allowed inside range snippets.
- Quantifiers work with captures: `[said {Alice}]+` returns all captured
  `Alice` ranges from matches of `said Alice`.

Ordinal occurrence syntax such as `[Alice]#2` is omitted from v0.
Disambiguation should be textual/contextual.

### Resolution Errors

Tools must report:

- zero matches where one is required;
- multiple matches where one is required;
- zero matches for `+`;
- more than one match for `?`;
- malformed snippet syntax;
- malformed or unsupported intralinea/local subject syntax.

Tools may choose whether to continue processing independent statements
after errors.

Internally, resolved text spans are `[start, end)` offsets over the
normalised canonical visible-text stream. Offsets count Unicode scalar
values after normalisation. UI and host integrations may map these
positions to grapheme clusters or native document positions.

Authors cannot write raw offset references in v0.

## Statements

The canonical statement form is Turtle-like:

```snipx
subject predicate object.
```

Commas and semicolons follow Turtle-style carry-forward semantics:

```snipx
Alice a Character;
  hair "red", "brown";
  friend Bob, Clara.
```

is equivalent to:

```snipx
Alice a Character.
Alice hair "red".
Alice hair "brown".
Alice friend Bob.
Alice friend Clara.
```

Blank lines are whitespace only. Standalone `.snipx` files require `.`
statement terminators.

Nested parenthesised statements and `-` last-subject shorthand are
omitted from v0.

### Subjects And Objects

Subjects and objects may be:

- snippets;
- text-span snippets prefixed with `~`;
- capitalised identifiers;
- URI literals in `<...>`;
- quoted strings;
- triple-quoted strings;
- numbers;
- booleans.

Lowercase identifiers are predicates/properties. Capitalised identifiers
are entities/classes. This capitalisation rule is semantic in v0.

Bare words are entities/classes or identifiers. Literal text values must
be quoted.

```snipx
Alice a Character.
Alice mood "frightened".
Alice sameAs <https://example.org/alice>.
```

URI literals are legal ordinary values/entities:

```snipx
Alice sameAs <https://example.org/people/alice>.
```

Namespaces and prefix declarations are deferred.

Lists/arrays are deferred. Use comma-separated repeated objects in v0:

```snipx
Alice alias "Al", "A.";
  trait Brave, Impulsive.
```

### Predicates

Predicates may be lowercase identifiers:

```snipx
Alice friend Bob;
  bornIn Oxford.
```

Natural-language predicates may be backtick-quoted phrases:

```snipx
Alice `was born in` Oxford;
  `is afraid of` TheDark.
```

Backtick-quoted predicates are ordinary predicates. They exist to make
hand-written marginalia more natural and to avoid ambiguity about where
a multi-word predicate ends and its object begins. Implementations may
map them to canonical predicate identifiers, but v0 does not require
that.

### Strings

Ordinary quoted strings use standard escapes:

```snipx
Alice note "Line one\nLine two \"quoted\".".
```

Triple-quoted strings support multiline text and dedent common
indentation:

```snipx
[Alice] note """
  This is a longer note.

  It can contain paragraphs.
""".
```

Dates must be represented as strings in v0.

### Comments

Standalone `.snipx` files support C-style comments:

```snipx
// line comment
/* block comment */
```

`#` is reserved and is not a v0 comment marker.

## Denotation And Text Spans

In commentaria, marginalia, and intralinea SnipX statements, a bare snippet
refers to the thing denoted by the matched text by default.

```snipx
[Alice] a Character.
```

The `~` sigil refers to the text span itself:

```snipx
~[Alice] ::"First visible mention.";
  italic true.
```

This distinction lets the same language express facts about fictional
entities and facts about the document surface.

Denotational snippets may create implicit denotations. They do not need
to be pre-bound to named entities:

```snipx
[the red door] motif Threshold.
```

Repeated matches from the same denotational snippet are presumed to
share the same denotation:

```snipx
[Alice]+ a Character.
```

Different snippet expressions that resolve to the same span refer to the
same mention and therefore the same default denotation unless explicitly
distinguished by a future extension.

Quantified denotational snippets collapse to denotation(s). Quantified
text-span snippets distribute over matched spans:

```snipx
[Alice]+ a Character.       // one fact about Alice
~[Alice]+ highlight true.   // one fact per matching text span
```

The built-in `is` predicate binds a snippet denotation to a named
entity:

```snipx
[Alice]+ is Alice.
Alice a Character;
  hair "red".
```

`is` is reserved for this binding role in v0.

Snippets may also appear as objects:

```snipx
[Alice] loves [Bob].
~[Alice] before ~[Bob].
```

## Fact Scope And Names

SnipX statements are interpreted inside a **fact scope**. A fact scope is
the set of statements, names, and resolved denotations that a tool
treats as belonging together.

v0 does not prescribe how fact scopes are chosen. Host/tool profiles
define them. A commentaria processor might use one scope per `.snipx`
file, while a Scrivener profile might accumulate statements from many
document notes into one project-level scope, or keep separate scopes for
draft sections, folders, or documents.

Within a chosen fact scope, capitalised names and URI literals may be
used as stable entity identifiers:

```snipx
Alice a Character.
Alice hair "red".
Alice sameAs <https://example.org/people/alice>.
```

Whether the same name in two host contexts is automatically treated as
the same entity is profile-defined. Tools may use one shared name scope,
separate name scopes, or another host-specific strategy. The same
statement asserted from two different marginalia fields may therefore be
one repeated fact or two context-local facts, depending on the active
profile.

Snippet denotations do not, by themselves, provide global identity
across different host contexts. A denotational snippet creates or uses
whatever denotation the active profile assigns to the matched mention(s)
in that resolution context. To make mentions in different chapters,
documents, or notes refer to the same entity in profiles with a shared
name scope, bind them to a shared name or URI:

```snipx
[Alice]+ is Alice.
Alice a Character.
```

If two different characters are both called Alice, use distinct names
and contextual snippets:

```snipx
[the elder {Alice}]+ is AliceElder.
[the younger {Alice}]+ is AliceYounger.
```

SnipX v0 does not define cardinality, OWL-style inference, or automatic
entity resolution. It also does not prescribe which entities are
automatically created, or in which scope they are created. Tools may
offer suggestions or diagnostics for ambiguous names, but scope and
identity policy belong to the host/tool profile.

## Built-In Predicates And Sugar

v0 defines a small built-in vocabulary:

- `a`: type/class predicate.
- `is`: binds a snippet denotation to a named entity.
- `note`: textual annotation.
- `sameAs`: entity/resource equivalence.
- `=`: synonym for `sameAs`.

`=` is a predicate synonym, not a general infix comparison operator:

```snipx
Alice = <https://example.org/alice>.
```

The decoration `::"..."` is sugar for `note "..."` on the immediately
preceding subject or object:

```snipx
[Alice] ::"Alice Smith, 30yo protagonist".
Alice friend Bob ::"childhood friend", Clara ::"rival".
```

Decorations compose with `,` and `;` normally when attached directly to a
subject or object. In v0, `::` attaches only to subjects and objects, not
predicates, and accepts only quoted strings.

Other predicates, including formatting/editing predicates such as
`bold`, `italic`, `colour`, or `replaceWith`, are user/profile-defined.
The core rule is that formatting facts are ordinary predicates on text
span subjects:

```snipx
~[Alice] italic true.
```

Formal negation and uncertainty are deferred from v0.

## Ambient Subjects

Some contexts provide an ambient subject. A subjectless statement begins
with a predicate or a standalone decoration and is filled with the
ambient subject.

```snipx
a Character;
  hair "red".
::"Opening chapter note".
```

If the ambient subject is `[]`, this is equivalent to:

```snipx
[] a Character;
  hair "red".
[] ::"Opening chapter note".
```

Ambient subject filling applies only when a statement begins with a
predicate or standalone decoration. It does not preserve a prior explicit
subject across `.`.

For example, in an ambient context:

```snipx
Alice friend Bob.
hair "red".
```

`hair "red".` applies to the ambient subject, not to `Alice` or `Bob`.

`;` works in ambient form. Once the ambient subject is filled for the
first predicate, normal carry-forward applies within that statement
chain:

```snipx
a Character;
  hair "red";
  friend Bob.
```

Standalone commentaria files have no ambient subject by default.
`@target` selects a snippet-resolution context but does not create an
ambient subject.

Marginalia and intralinea shortcuts provide ambient subjects.

## Intralinea Local Subjects

Intralinea blocks may set a local ambient subject using scope markers.
Single arrows select sentence scope; double arrows select paragraph
scope.

```text
{{< a Promise. }}              sentence start to marker
{{ a Promise >}}               marker to sentence end
{{<> a Promise. }}             whole current sentence

{{<< theme Entrapment. }}      paragraph start to marker
{{ theme Entrapment >>}}       marker to paragraph end
{{<<>> theme Entrapment. }}    whole current paragraph
```

Local subjects are denotational by default. Prefix the marker with `~`
to use the text span itself:

```text
{{~< highlight true. }}
{{~<> highlight true. }}
{{~<<>> italic true. }}
```

Sentence boundaries use a simple default rule in v0: `.`, `?`, or `!`
followed by whitespace or end of text marks a sentence boundary.
Profiles may improve this with abbreviation handling or host sentence
segmentation.

Paragraph boundaries use host paragraph structure when available. In
plain text and Markdown, one or more blank lines separate paragraphs.

Literal `{{` in intralinea-enabled documents has no v0 escape. This is a
known limitation and open issue.

## Targets And Profiles

Standalone commentaria may declare a target:

```snipx
@target <novel.rtf>
@target <chapters/01.md>
@target <file:///Users/me/book/novel.rtf>
```

`@target` accepts relative paths/URIs and absolute URIs. It is optional;
tools may supply the target out of band.

v0 commentaria files target one document per file.

The `.snipx` extension is reserved for standalone SnipX/commentaria.
Marginalia and intralinea SnipX are host-defined and do not require a separate
extension.

## Reference Implementation

The v0 reference implementation should consist of:

- `snipx-core`: a reusable Rust crate implementing the language core;
- `snipx`: a small command-line driver built on `snipx-core`.

The crate is the primary implementation surface. The command-line driver
exists to expose and test parsing, visible-text extraction, snippet
resolution, diagnostics, and canonical JSON serialisation.

Host-specific integrations, including Scrivener, RTF, DOCX, EPUB,
editor plugins, graph databases, and richer document source maps, should
be separate tools or crates built on top of `snipx-core`.

### Crate Responsibilities

`snipx-core` should provide:

- lossless parsing for commentaria, marginalia, and intralinea input
  forms;
- a typed syntax/AST layer over the lossless parse tree;
- statement expansion for `.`, `;`, `,`, ambient subjects, and `::`
  decoration sugar;
- canonical visible-text extraction for plain text and Markdown;
- exact and loose snippet matching for the v0 profiles;
- snippet resolution to `[start, end)` offsets in the canonical
  visible-text stream;
- denotation and text-span value construction;
- diagnostic production with source locations;
- canonical JSON data structures and serialisation.

The crate should not decide host policy that belongs outside the core
language. Callers provide:

- the input form: commentaria, marginalia, or intralinea;
- the target document, if snippets must resolve against an external
  text;
- the active profile;
- the ambient subject, if the input context has one;
- the fact scope and name-scope policy;
- any host-specific mapping from canonical offsets back to native
  document positions.

### Parser Architecture

The reference parser should be lossless and Rowan-based. SnipX syntax is
small, but comments, whitespace, embedded forms, quoted snippets,
triple-quoted strings, decorations, and Turtle-style carry-forward are
important enough that the concrete syntax tree should be preserved.

The intended pipeline is:

```text
source text
  -> lossless Rowan syntax tree
  -> typed AST/query layer
  -> expanded statements
  -> resolved snippets
  -> resolved facts and diagnostics
```

This design should support future formatting, editor integration,
partial parsing, and good diagnostics without changing the public
language.

### Command-Line Driver

The `snipx` command-line driver should provide three initial commands:

```text
snipx check
snipx resolve
snipx export
```

Input form is independent of command verb and is selected with `--as` or
one of the equivalent short flags:

```text
--as commentaria    -c
--as marginalia     -m
--as intralinea     -i
```

The long form is preferred in documentation. The short flags are
convenience aliases.

Examples:

```sh
snipx check notes.snipx --as commentaria --target novel.md
snipx check notes.snipx -c --target novel.md

snipx resolve notes.txt --as marginalia --target chapter.md --ambient '[]'
snipx resolve notes.txt -m --target chapter.md --ambient '[]'

snipx export chapter.md --as intralinea --pretty
snipx export chapter.md -i --pretty
```

`check` parses the input and reports diagnostics. If a target is
available, `check` should also resolve snippets so that unresolved,
ambiguous, or unsupported snippets are reported.

`resolve` parses the input, extracts or receives the target visible text,
resolves snippets, and emits resolution-oriented JSON. This output is
intended for debugging snippets and profiles rather than for consuming
facts.

`export` parses, expands, resolves, and emits canonical SnipX JSON facts
plus diagnostics.

### CLI Options

The initial command-line options should include:

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
```

If both `--as` and a short input-form flag are supplied, they must agree.
Supplying more than one short input-form flag is an error.

The command-line driver should support only local files and standard
input/output in v0. Network fetching and host-specific project loading
are outside the reference implementation.

### Input Forms In The CLI

Commentaria input is SnipX-by-default. The input may contain `@target`
and `@profile` directives. Command-line `--target` and `--profile`
values override or supply directive values according to tool policy; the
CLI should report the effective target and profile in JSON output.

Marginalia input is prose-by-default. The CLI parses unlabelled fences,
`snipx` fences, and `///` single-line SnipX entries. Prose outside SnipX
blocks is preserved in diagnostics and source locations, but v0 export
does not automatically convert prose to `note` facts unless a later
profile explicitly requests it. The CLI may receive an ambient subject
with `--ambient`.

Intralinea input is a target document containing `{{ ... }}` blocks. The
CLI removes those blocks from the canonical visible-text stream before
resolving snippets. The stripped visible text is the target context for
explicit snippets and local intralinea subjects.

### Reference Profiles

The reference implementation should support these profiles initially:

```text
plain
plain-loose
markdown
markdown-loose
```

`plain` and `plain-loose` operate over decoded text content.

`markdown` and `markdown-loose` operate over rendered visible prose:
Markdown formatting syntax is not target text. Headings, block quotes,
list item text, code block text, inline code text, link text, and image
alt text are visible text. Link destinations, image destinations,
reference definitions, and raw HTML tags are not visible text. Raw HTML
content handling may be conservative in v0; unsupported constructs that
would affect visible-text extraction should produce diagnostics.

Scrivener, RTF, DOCX, PDF, EPUB, and other rich-text profiles are
outside the reference implementation.

### Canonical JSON

The reference implementation should define JSON as the only canonical
machine output in v0. `--pretty` changes formatting only; it does not
change the schema.

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
    "profile": "markdown-loose"
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

Facts should preserve enough provenance to explain where they came from:

```json
{
  "subject": {
    "kind": "entity",
    "id": "denotation:local:d1",
    "source": {
      "snippet": "[Alice]+",
      "spans": [
        {
          "start": 120,
          "end": 125,
          "text": "Alice"
        }
      ]
    }
  },
  "predicate": {
    "kind": "predicate",
    "value": "a"
  },
  "object": {
    "kind": "name",
    "value": "Character"
  },
  "origin": {
    "path": "notes.snipx",
    "line": 4,
    "column": 1
  }
}
```

The exact schema may evolve during implementation, but v0 should keep
JSON as the compatibility contract for other tools.

### Diagnostics And Exit Codes

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

## Deferred And Open Issues

Deferred from v0:

- per-snippet profile overrides;
- explicit word-boundary syntax;
- ordinal occurrence syntax;
- multiple captures per snippet;
- captures inside range snippets;
- namespace and prefix declarations;
- list/array syntax;
- date literals;
- formal negation and uncertainty;
- nested incantations;
- last-subject shorthand;
- raw offset references;
- `^^` type sugar;
- escaping literal `{{` in intralinea-enabled prose.

Opportunities for later versions:

- richer extraction profiles for DOCX, PDF, EPUB, and Scrivener project
  structure;
- host-specific mappings from canonical visible-text offsets back to
  editable rich-text ranges;
- profile vocabularies for formatting and document edits;
- optional diagnostics for fragile snippets;
- optional export to RDF-like triples/quads;
- explicit support for claims, sources, narrator/character attribution,
  uncertainty, contradiction, and inference in literary analysis.
