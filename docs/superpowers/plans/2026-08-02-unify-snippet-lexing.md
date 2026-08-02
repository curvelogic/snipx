# Unify Snippet Body Lexing (snipx-2mv) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the parser's CST the single lexical authority for snippet bodies: expand extracts a structured `SnippetValue` from the CST, and the matcher consumes that structure, deleting the four independent string re-lexers (`strip_capture`, `range_separator`, `has_unquoted_capture`, `unquote` in match.rs, and `snippet_parts` in resolve.rs).

**Architecture:** A new `snippet` module in `snipx-core` defines `SnippetPart`/`SnippetValue`/`Cardinality` and builds them from a `Snippet`/`RangeSnippet` syntax node. `expand::Value::Snippet`/`TextSpanSnippet` carry `SnippetValue` instead of `String`. `match_snippet` takes `&[SnippetPart]`. The parser gains one change: the `..` range separator becomes its own token instead of hiding inside `Text`.

**Tech Stack:** Rust workspace (`crates/snipx-core`, `crates/snipx`), rowan CST, insta snapshot tests.

## Global Constraints

- Tracked as beads issue **snipx-2mv**; do not use TodoWrite/markdown TODOs.
- Preserve JSON output byte-for-byte: `JsonValue::Snippet { source }` etc. keep the same source strings (trimmed snippet syntax, `~` stripped for text-span snippets).
- Preserve diagnostic **codes and messages** for every currently-tested input (listed per task below). Where the old string lexer deviates from the spec on pathological inputs, the new structural behaviour wins (spec: docs/language-spec.md "Quoted Snippet Text": *"Quotes delimit only when they wrap an entire snippet body or an entire range endpoint. Anywhere else inside a snippet body, quote characters are ordinary literal text… quote-escape processing applies only in the delimiting position."*).
- Quote decoding in snippet bodies decodes ONLY `\"` → `"` (see `match.rs::unquote` today). Never reuse `expand::unescape` (that is for ordinary strings).
- Quality gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`. Snapshots via insta inline: regenerate with `INSTA_UPDATE=force cargo test -p snipx-core --test parser_snapshots` then re-run normally, or `cargo insta review` if available. Inspect every snapshot diff — only range-snippet token structure may change.
- Work on a feature branch (e.g. `claude/unify-snippet-lexing`), commit per task. Conservative git profile: no push without user approval (the delivery-pr-default memory says push + PR at completion is the default for finished branches).

## Current-behaviour reference (read before any task)

- `crates/snipx-core/src/parser.rs:1067-1147` `parse_snippet`: emits `Snippet`/`RangeSnippet` node containing `LBrack`, raw `Text` tokens, `QuotedSnippetPart` (wrapping a `String` node: `Quote`,`Text`,`Quote` tokens; escapes consumed pairwise), `Capture` nodes (`LBrace`,`Text`,`RBrace`), invalid captures wrapped in an `Error` node (with a ParseError diagnostic already pushed), optional trailing `Quantifier` node (child `Text` token `+`/`*`/`?`), `RBrack`. The `..` separator is currently plain `Text` — Task 1 fixes that.
- `crates/snipx-core/src/match.rs`: string-based `match_snippet(body, …)`. Semantics to preserve: needle normalisation (`normalize`, `loose_replacement`), non-overlap via `last_end`, capture range mapped through normalisation by prefix char-counting, empty needle → whole-document span, open/closed range pairing in `match_range`.
- `crates/snipx-core/src/resolve.rs:343-354` `snippet_parts`: strips `[`…`]`, reads trailing quantifier; `None` → `InvalidSnippet` "Invalid snippet syntax: {source}".
- Error messages that MUST survive (all `DiagnosticCode::InvalidSnippet`):
  - "A snippet may contain at most one capture"
  - "Capture is not terminated"
  - "Capture may not be empty"
  - "Capture boundaries collapse during text normalisation"
  - "Quoted snippet text is not terminated"
  - "A range snippet may contain only one range separator"
  - "Captures are not allowed inside range snippets"
  - "Invalid snippet syntax: {source}" (unterminated snippet, from resolve)

## File Structure

- Modify: `crates/snipx-core/src/parser.rs` (parse_snippet only)
- Create: `crates/snipx-core/src/snippet.rs` (SnippetPart, SnippetValue, Cardinality, from_node)
- Modify: `crates/snipx-core/src/match.rs` (structured matcher; delete string lexers)
- Modify: `crates/snipx-core/src/expand.rs` (Value carries SnippetValue)
- Modify: `crates/snipx-core/src/resolve.rs` (consume structure; delete snippet_parts/Cardinality)
- Modify: `crates/snipx-core/src/json.rs` (unwrap `.source`)
- Modify: `crates/snipx-core/src/lib.rs` (module + exports)
- Tests: `crates/snipx-core/tests/parser_snapshots.rs`, new `crates/snipx-core/tests/snippet_structure.rs`, `crates/snipx-core/tests/resolution.rs`
- Modify: `CHANGELOG.md`

---

### Task 1: Parser emits the `..` range separator as a `Dot` token

**Files:**
- Modify: `crates/snipx-core/src/parser.rs:1083-1124` (parse_snippet main loop)
- Test: `crates/snipx-core/tests/parser_snapshots.rs`

**Interfaces:**
- Produces: inside a `RangeSnippet` node, each `..` appears as one token `Dot` with text `".."` between `Text`/`QuotedSnippetPart` parts. Plain `Snippet` nodes never contain `Dot` tokens. Formatter is unaffected (it echoes token text verbatim, `format.rs:57` etc.).

- [ ] **Step 1: Write the failing snapshot test**

Add to `crates/snipx-core/tests/parser_snapshots.rs`:

```rust
#[test]
fn range_snippet_tokenizes_separator() {
    let parsed = parse(
        "[\"A..a\"..End] a Scene.\n",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    insta::assert_snapshot!(
        parsed.debug_tree(),
        @r###"
ROOT
  STATEMENT
    SUBJECT
      RANGE_SNIPPET
        L_BRACK "["
        QUOTED_SNIPPET_PART
          STRING
            QUOTE "\""
            TEXT "A..a"
            QUOTE "\""
        DOT ".."
        TEXT "End"
        R_BRACK "]"
    WHITESPACE " "
    PREDICATE
      IDENT
        TEXT "a"
    WHITESPACE " "
    OBJECT_LIST
      OBJECT
        IDENT
          TEXT "Scene"
    DOT "."
  WHITESPACE "\n"
"###
    );
}
```

(If the actual tree differs in incidental structure — e.g. `QUOTE` token text rendering — accept the insta-corrected version, but the `DOT ".."` token between the quoted part and `TEXT "End"` is the requirement. Quoted `..` must stay inside the STRING text.)

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p snipx-core --test parser_snapshots range_snippet_tokenizes_separator`
Expected: FAIL — snapshot mismatch, `..` currently inside a `TEXT` token.

- [ ] **Step 3: Implement the separator tokenization**

In `parse_snippet` (`parser.rs`), the loop body currently reads:

```rust
            } else if ch == '{' {
```

Insert a branch BEFORE the final `else { self.advance_char(); }` arm (after the `'{'` branch), guarded by `is_range` so plain snippets are untouched:

```rust
            } else if is_range && ch == '.' && self.source[self.pos + 1..].starts_with('.') {
                if self.pos > text_start {
                    self.token_from(SyntaxKind::Text, text_start, self.pos);
                }
                self.token(SyntaxKind::Dot, "..");
                self.pos += 2;
                text_start = self.pos;
```

Note `token(kind, text)` does not advance `self.pos` (see `LBrack` handling at parser.rs:1079-1080), hence the explicit `self.pos += 2`.

- [ ] **Step 4: Run the new test and the full snapshot suites**

Run: `cargo test -p snipx-core --test parser_snapshots && cargo test -p snipx-core --test formatter_snapshots`
Expected: new test PASSES. Existing range-snippet snapshots fail only by `TEXT "a..b"` splitting into `TEXT "a"` / `DOT ".."` / `TEXT "b"`. Regenerate (`INSTA_UPDATE=force cargo test -p snipx-core --test parser_snapshots`), inspect the diff, re-run cleanly. Formatter snapshots must pass unchanged (tokens echo verbatim).

- [ ] **Step 5: Run the whole workspace suite and commit**

Run: `cargo test --workspace`
Expected: PASS — nothing downstream reads snippet-internal tokens yet (expand flattens with `node.to_string()`).

```bash
git add crates/snipx-core/src/parser.rs crates/snipx-core/tests/parser_snapshots.rs
git commit -m "Tokenize range separator inside range snippets"
```

---

### Task 2: `snippet` module — structured snippet values from the CST

**Files:**
- Create: `crates/snipx-core/src/snippet.rs`
- Modify: `crates/snipx-core/src/lib.rs` (add `pub mod snippet;` alongside existing modules, and `pub use snippet::{Cardinality, SnippetPart, SnippetValue};`)
- Test: create `crates/snipx-core/tests/snippet_structure.rs`

**Interfaces:**
- Produces (used by Tasks 3–5):

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum SnippetPart {
    /// Raw unquoted body text, matched verbatim.
    Text(String),
    /// A quoted run. `raw` includes the delimiters and undecoded escapes;
    /// `decoded` strips the delimiters and decodes only `\"` -> `"`.
    Quoted { raw: String, decoded: String, terminated: bool },
    /// `{...}` capture; `text` is the raw inner text.
    Capture { text: String, terminated: bool },
    /// An unquoted, top-level `..` in a range snippet.
    RangeSeparator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality { ExactlyOne, OneOrMore, ZeroOrMore, ZeroOrOne }

#[derive(Debug, Clone, PartialEq)]
pub struct SnippetValue {
    /// Trimmed source syntax as today (`[Alice]+`), `~` already stripped
    /// by the caller for text-span snippets. Feeds JSON output and
    /// diagnostic messages unchanged.
    pub source: String,
    pub parts: Vec<SnippetPart>,
    pub cardinality: Cardinality,
    /// False when the closing `]` is missing.
    pub terminated: bool,
}

impl SnippetValue {
    /// `node` must be a `Snippet` or `RangeSnippet` node.
    pub fn from_node(node: &SyntaxNode, source: String) -> SnippetValue
}
```

- [ ] **Step 1: Write the module**

`crates/snipx-core/src/snippet.rs` (full content; the type definitions above plus):

```rust
use rowan::NodeOrToken;

use crate::syntax::{SyntaxKind, SyntaxNode};

// ... SnippetPart, Cardinality, SnippetValue definitions from the interface block ...

impl SnippetValue {
    pub fn from_node(node: &SyntaxNode, source: String) -> SnippetValue {
        let mut parts = Vec::new();
        let mut cardinality = Cardinality::ExactlyOne;
        let mut terminated = false;

        for element in node.children_with_tokens() {
            match element {
                NodeOrToken::Token(token) => match token.kind() {
                    SyntaxKind::LBrack => {}
                    SyntaxKind::RBrack => terminated = true,
                    SyntaxKind::Dot => parts.push(SnippetPart::RangeSeparator),
                    _ => parts.push(SnippetPart::Text(token.text().to_owned())),
                },
                NodeOrToken::Node(child) => match child.kind() {
                    SyntaxKind::QuotedSnippetPart => parts.push(quoted_part(&child)),
                    SyntaxKind::Capture => parts.push(capture_part(&child)),
                    SyntaxKind::Quantifier => {
                        cardinality = match child.to_string().as_str() {
                            "+" => Cardinality::OneOrMore,
                            "*" => Cardinality::ZeroOrMore,
                            "?" => Cardinality::ZeroOrOne,
                            _ => Cardinality::ExactlyOne,
                        };
                    }
                    // Invalid captures (second capture, capture in a range)
                    // are wrapped in an Error node by the parser; surface
                    // them so the matcher reports the same InvalidSnippet
                    // errors the string lexer used to.
                    SyntaxKind::Error => match child
                        .descendants()
                        .find(|inner| inner.kind() == SyntaxKind::Capture)
                    {
                        Some(capture) => parts.push(capture_part(&capture)),
                        None => parts.push(SnippetPart::Text(child.to_string())),
                    },
                    _ => parts.push(SnippetPart::Text(child.to_string())),
                },
            }
        }

        SnippetValue { source, parts, cardinality, terminated }
    }
}

/// Snippet quoting is very literal: only the quote delimiter itself is
/// escaped, so decoding maps `\"` to `"` and nothing else.
fn quoted_part(node: &SyntaxNode) -> SnippetPart {
    let raw = node.to_string();
    let mut quotes = 0usize;
    let mut content = String::new();
    for element in node.descendants_with_tokens() {
        if let NodeOrToken::Token(token) = element {
            match token.kind() {
                SyntaxKind::Quote => quotes += 1,
                SyntaxKind::Text => content.push_str(token.text()),
                _ => {}
            }
        }
    }
    SnippetPart::Quoted {
        raw,
        decoded: content.replace("\\\"", "\""),
        terminated: quotes >= 2,
    }
}

fn capture_part(node: &SyntaxNode) -> SnippetPart {
    let mut text = String::new();
    let mut terminated = false;
    for element in node.children_with_tokens() {
        if let NodeOrToken::Token(token) = element {
            match token.kind() {
                SyntaxKind::Text => text.push_str(token.text()),
                SyntaxKind::RBrace => terminated = true,
                _ => {}
            }
        }
    }
    SnippetPart::Capture { text, terminated }
}
```

Adjust `Quote` counting if `parse_triple_string` uses different token shapes (check its emission before finalising) — the invariant is: `terminated` is true iff the closing delimiter is present, `decoded` is the inner text with only `\"` decoded.

- [ ] **Step 2: Write the tests**

`crates/snipx-core/tests/snippet_structure.rs`:

```rust
use snipx_core::{
    parse, Cardinality, InputForm, ParseOptions, SnippetPart, SnippetValue, SyntaxKind,
};

fn snippet_value(snippet: &str) -> SnippetValue {
    let parsed = parse(
        &format!("{snippet} a Character.\n"),
        ParseOptions { input_form: InputForm::Commentaria },
    );
    let node = parsed
        .syntax()
        .descendants()
        .find(|node| matches!(node.kind(), SyntaxKind::Snippet | SyntaxKind::RangeSnippet))
        .expect("input contains a snippet");
    let source = node.to_string();
    SnippetValue::from_node(&node, source)
}

#[test]
fn plain_text_snippet() {
    let value = snippet_value("[Alice]");
    assert_eq!(value.parts, vec![SnippetPart::Text("Alice".into())]);
    assert_eq!(value.cardinality, Cardinality::ExactlyOne);
    assert!(value.terminated);
}

#[test]
fn quantifier_and_capture() {
    let value = snippet_value("[looked at {Alice} and smiled]+");
    assert_eq!(
        value.parts,
        vec![
            SnippetPart::Text("looked at ".into()),
            SnippetPart::Capture { text: "Alice".into(), terminated: true },
            SnippetPart::Text(" and smiled".into()),
        ]
    );
    assert_eq!(value.cardinality, Cardinality::OneOrMore);
}

#[test]
fn quoted_part_decodes_only_quote_escape() {
    let value = snippet_value(r#"["say \"hi\"\n"]"#);
    assert_eq!(
        value.parts,
        vec![SnippetPart::Quoted {
            raw: r#""say \"hi\"\n""#.into(),
            decoded: r#"say "hi"\n"#.into(),
            terminated: true,
        }]
    );
}

#[test]
fn range_snippet_splits_on_separator() {
    let value = snippet_value(r#"["A..a"..End]"#);
    assert_eq!(value.parts.len(), 3);
    assert!(matches!(value.parts[0], SnippetPart::Quoted { .. }));
    assert_eq!(value.parts[1], SnippetPart::RangeSeparator);
    assert_eq!(value.parts[2], SnippetPart::Text("End".into()));
}

#[test]
fn unterminated_snippet_and_capture_are_flagged() {
    let value = snippet_value("[Alice");
    assert!(!value.terminated);

    let value = snippet_value("[A {to B");
    assert!(value
        .parts
        .iter()
        .any(|part| matches!(part, SnippetPart::Capture { terminated: false, .. })));
}

#[test]
fn empty_snippet_has_no_parts() {
    let value = snippet_value("[]");
    assert!(value.parts.is_empty());
    assert!(value.terminated);
}
```

- [ ] **Step 3: Run the tests, fix until green**

Run: `cargo test -p snipx-core --test snippet_structure`
Expected: PASS. (First run will fail to compile until lib.rs exports are in place — that is the red step.) If a tree-shape assumption is wrong (e.g. quoted-part token layout), print `parsed.debug_tree()` in a scratch test to correct `from_node`, not the test's intent.

- [ ] **Step 4: Gates and commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all`

```bash
git add crates/snipx-core/src/snippet.rs crates/snipx-core/src/lib.rs crates/snipx-core/tests/snippet_structure.rs
git commit -m "Add structured snippet extraction from the CST"
```

---

### Task 3: Structured matcher (`match_snippet_parts`)

**Files:**
- Modify: `crates/snipx-core/src/match.rs`
- Modify: `crates/snipx-core/src/lib.rs` (export `match_snippet_parts` temporarily)
- Test: `crates/snipx-core/tests/resolution.rs`

**Interfaces:**
- Consumes: `SnippetPart` from Task 2.
- Produces: `pub fn match_snippet_parts(parts: &[SnippetPart], visible_text: &VisibleText, profile: Profile) -> Result<Vec<TextSpan>, Diagnostic>`. The old string `match_snippet` stays untouched until Task 4 (both suites run side by side to prove parity).

- [ ] **Step 1: Write the failing tests**

Add to `crates/snipx-core/tests/resolution.rs` (imports: `match_snippet_parts`, `SnippetPart`, `SnippetValue`, `SyntaxKind`, `ParseOptions`, `InputForm`):

```rust
fn body_parts(body: &str) -> Vec<SnippetPart> {
    let parsed = parse(
        &format!("[{body}] a Character.\n"),
        ParseOptions { input_form: InputForm::Commentaria },
    );
    let node = parsed
        .syntax()
        .descendants()
        .find(|node| matches!(node.kind(), SyntaxKind::Snippet | SyntaxKind::RangeSnippet))
        .expect("body parses as a snippet");
    let source = node.to_string();
    SnippetValue::from_node(&node, source).parts
}

#[test]
fn structured_matcher_agrees_with_string_matcher() {
    let visible = extract_visible_text(
        "Alice met Alice. She said \"sic\" loudly, from A to B.",
        Profile::Plain,
    )
    .unwrap();

    for body in [
        "Alice",
        " Alice ",
        "met..loudly",
        "..B",
        "Alice met..",
        "",
        "\"\\\"sic\\\"\"",
        "said {\"sic\"} loudly",
        "met {Alice}",
    ] {
        assert_eq!(
            match_snippet_parts(&body_parts(body), &visible, Profile::Plain),
            match_snippet(body, &visible, Profile::Plain),
            "{body:?}"
        );
    }
}

#[test]
fn structured_matcher_preserves_invalid_snippet_errors() {
    let visible = extract_visible_text("A to B", Profile::Plain).unwrap();

    for (body, message) in [
        ("{A}..B", "Captures are not allowed inside range snippets"),
        ("A..B..C", "A range snippet may contain only one range separator"),
        ("A {} B", "Capture may not be empty"),
        ("A {b} {c}", "A snippet may contain at most one capture"),
        ("A {to B", "Capture is not terminated"),
        ("\"unterminated", "Quoted snippet text is not terminated"),
    ] {
        let diagnostic = match_snippet_parts(&body_parts(body), &visible, Profile::Plain)
            .unwrap_err();
        assert_eq!(diagnostic.code, DiagnosticCode::InvalidSnippet, "{body:?}");
        assert_eq!(diagnostic.message, message, "{body:?}");
    }
}
```

Note `"said {\"sic\"} loudly"`: the CST puts a quoted part *inside* the capture? No — `parse_capture` consumes everything to `}` as raw text, so the capture text is `"sic"` including quote characters, matching the string lexer (quotes inside a capture are literal pattern text). If the parity test disagrees, trust the string matcher's result for these inputs and fix the structured side.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p snipx-core --test resolution structured_matcher`
Expected: compile FAIL (`match_snippet_parts` unresolved).

- [ ] **Step 3: Implement the structured matcher**

In `match.rs`, add (keeping every existing function for now):

```rust
use crate::snippet::SnippetPart;

pub fn match_snippet_parts(
    parts: &[SnippetPart],
    visible_text: &VisibleText,
    profile: Profile,
) -> Result<Vec<TextSpan>, Diagnostic> {
    let separators = parts
        .iter()
        .filter(|part| matches!(part, SnippetPart::RangeSeparator))
        .count();
    if separators > 1 {
        return Err(invalid(
            DiagnosticCode::InvalidSnippet,
            "A range snippet may contain only one range separator",
        ));
    }
    if separators == 1 {
        if parts
            .iter()
            .any(|part| matches!(part, SnippetPart::Capture { .. }))
        {
            return Err(invalid(
                DiagnosticCode::InvalidSnippet,
                "Captures are not allowed inside range snippets",
            ));
        }
        let split = parts
            .iter()
            .position(|part| matches!(part, SnippetPart::RangeSeparator))
            .expect("separator counted above");
        let start = endpoint_needle(&parts[..split])?;
        let end = endpoint_needle(&parts[split + 1..])?;
        return match_range_needles(&start, &end, visible_text, profile);
    }

    let (pattern, capture) = assemble_pattern(parts)?;
    match_pattern(&pattern, capture, visible_text, profile)
}

/// Spec ("Quoted Snippet Text"): quotes delimit only when they wrap an
/// entire snippet body or an entire range endpoint; anywhere else they
/// are literal target text.
fn assemble_pattern(
    parts: &[SnippetPart],
) -> Result<(String, Option<std::ops::Range<usize>>), Diagnostic> {
    if let [SnippetPart::Quoted { decoded, terminated, .. }] = parts {
        if !terminated {
            return Err(invalid(
                DiagnosticCode::InvalidSnippet,
                "Quoted snippet text is not terminated",
            ));
        }
        return Ok((decoded.clone(), None));
    }

    let mut pattern = String::new();
    let mut capture = None;
    for part in parts {
        match part {
            SnippetPart::Text(text) => pattern.push_str(text),
            SnippetPart::Quoted { raw, terminated, .. } => {
                if !terminated {
                    return Err(invalid(
                        DiagnosticCode::InvalidSnippet,
                        "Quoted snippet text is not terminated",
                    ));
                }
                pattern.push_str(raw);
            }
            SnippetPart::Capture { text, terminated } => {
                if capture.is_some() {
                    return Err(invalid(
                        DiagnosticCode::InvalidSnippet,
                        "A snippet may contain at most one capture",
                    ));
                }
                if !terminated {
                    return Err(invalid(
                        DiagnosticCode::InvalidSnippet,
                        "Capture is not terminated",
                    ));
                }
                if text.is_empty() {
                    return Err(invalid(
                        DiagnosticCode::InvalidSnippet,
                        "Capture may not be empty",
                    ));
                }
                let start = pattern.chars().count();
                pattern.push_str(text);
                capture = Some(start..pattern.chars().count());
            }
            SnippetPart::RangeSeparator => unreachable!("handled by caller"),
        }
    }
    Ok((pattern, capture))
}

fn endpoint_needle(parts: &[SnippetPart]) -> Result<String, Diagnostic> {
    let (needle, _) = assemble_pattern(parts)?;
    Ok(needle)
}
```

`match_pattern` is the body of today's `match_capture` with `strip_capture`/`unquote` removed: signature `fn match_pattern(pattern: &str, capture: Option<std::ops::Range<usize>>, visible_text: &VisibleText, profile: Profile) -> Result<Vec<TextSpan>, Diagnostic>`; it starts at today's `let loose = …` line with `needle = normalize(pattern, loose)` and keeps the empty-needle whole-document return, the capture-through-normalisation mapping (which reads `pattern` and `capture` directly), the collapse error, and the `last_end` loop verbatim.

`match_range_needles(start: &str, end: &str, …)` is today's `match_range` with the two `unquote` calls deleted and every `match_capture(x, …)` call replaced by `match_pattern(x, None, …)`; the four `(is_empty, is_empty)` branches are otherwise unchanged. (Note this fixes the old double-unquote/capture-in-endpoint drift: endpoints are now matched literally, per spec.)

Export in `lib.rs`: extend line 27 to `pub use r#match::{match_snippet, match_snippet_parts, TextSpan};`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p snipx-core --test resolution`
Expected: PASS, including the parity test against the old string matcher.

- [ ] **Step 5: Gates and commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all`

```bash
git add crates/snipx-core/src/match.rs crates/snipx-core/src/lib.rs crates/snipx-core/tests/resolution.rs
git commit -m "Add structured snippet matcher alongside string matcher"
```

---

### Task 4: Thread `SnippetValue` through expand/resolve/json; delete the string lexers

**Files:**
- Modify: `crates/snipx-core/src/expand.rs` (Value enum + value_from_node)
- Modify: `crates/snipx-core/src/resolve.rs` (resolve_value; delete `snippet_parts` and the private `Cardinality`)
- Modify: `crates/snipx-core/src/match.rs` (delete old `match_snippet`, `match_capture`, `strip_capture`, `range_separator`, `has_unquoted_capture`, `unquote`; rename `match_snippet_parts` → `match_snippet`, `match_range_needles` → `match_range`)
- Modify: `crates/snipx-core/src/json.rs`, `crates/snipx-core/src/lib.rs`
- Test: `crates/snipx-core/tests/resolution.rs`

**Interfaces:**
- Produces: `Value::Snippet(SnippetValue)`, `Value::TextSpanSnippet(SnippetValue)`; `pub fn match_snippet(parts: &[SnippetPart], visible_text: &VisibleText, profile: Profile) -> Result<Vec<TextSpan>, Diagnostic>`. `Value::Unresolved(String)` unchanged.

- [ ] **Step 1: Update the Value enum and extraction in expand.rs**

```rust
use crate::snippet::SnippetValue;
// in Value:
    Snippet(SnippetValue),
    TextSpanSnippet(SnippetValue),
```

In `value_from_node` (expand.rs:245-253), replace the Snippet arm:

```rust
        SyntaxKind::Snippet | SyntaxKind::RangeSnippet => {
            let value_text = node.to_string();
            let syntax = value_text.trim();
            let (text_span, source) = match syntax.strip_prefix('~') {
                Some(rest) => (true, rest),
                None => (false, syntax),
            };
            let snippet = SnippetValue::from_node(&value_node, source.to_owned());
            Some(if text_span {
                Value::TextSpanSnippet(snippet)
            } else {
                Value::Snippet(snippet)
            })
        }
```

- [ ] **Step 2: Rewrite resolve_value in resolve.rs**

Delete `snippet_parts` and the private `Cardinality` enum (resolve.rs:335-354); import `crate::snippet::Cardinality` and `crate::snippet::SnippetValue`. Replace the body of `resolve_value` from `let source = match value` down to the `resolutions.push`:

```rust
    let snippet = match value {
        Value::Snippet(snippet) | Value::TextSpanSnippet(snippet) => snippet.clone(),
        _ => return,
    };

    let spans = match match_snippet(&snippet.parts, visible_text, profile) {
        // An unterminated snippet with no more specific lexical defect
        // keeps the historical generic diagnostic.
        Ok(_) if !snippet.terminated => {
            diagnostics.push(diagnostic(
                DiagnosticCode::InvalidSnippet,
                format!("Invalid snippet syntax: {}", snippet.source),
                source_span,
            ));
            *value = Value::Unresolved(snippet.source);
            return;
        }
        Ok(spans) => spans,
        Err(mut error) => {
            if error.span.is_none() {
                error.span = source_span;
            }
            diagnostics.push(error);
            *value = Value::Unresolved(snippet.source);
            return;
        }
    };

    let error_code = match snippet.cardinality {
        Cardinality::ExactlyOne if spans.is_empty() => Some(DiagnosticCode::SnippetNotFound),
        Cardinality::ExactlyOne if spans.len() > 1 => Some(DiagnosticCode::SnippetAmbiguous),
        Cardinality::OneOrMore if spans.is_empty() => Some(DiagnosticCode::SnippetNotFound),
        Cardinality::ZeroOrOne if spans.len() > 1 => Some(DiagnosticCode::SnippetAmbiguous),
        _ => None,
    };
    if let Some(code) = error_code {
        let message = match code {
            DiagnosticCode::SnippetNotFound => {
                format!("Snippet did not match: {}", snippet.source)
            }
            DiagnosticCode::SnippetAmbiguous => {
                format!("Snippet matched more than allowed: {}", snippet.source)
            }
            _ => unreachable!(),
        };
        diagnostics.push(diagnostic(code, message, source_span));
        *value = Value::Unresolved(snippet.source);
        return;
    }

    resolutions.push(SnippetResolution {
        source: snippet.source,
        source_span,
        spans,
    });
```

- [ ] **Step 3: Finish the mechanical renames**

- match.rs: delete `strip_capture`, `range_separator`, `has_unquoted_capture`, `unquote`, `match_capture`, and the old string `match_snippet`; rename `match_snippet_parts` → `match_snippet` and `match_range_needles` → `match_range`.
- json.rs:377-378: `Value::Snippet(snippet) => JsonValue::Snippet { source: snippet.source }`, same for `TextSpanSnippet`.
- lib.rs: export becomes `pub use r#match::{match_snippet, TextSpan};`.

- [ ] **Step 4: Update the tests that used string bodies**

In `crates/snipx-core/tests/resolution.rs`:
- Every `match_snippet("<body>", &visible, profile)` call becomes `match_snippet(&body_parts("<body>"), &visible, profile)` (the Task 3 helper). The two Task 3 tests collapse: delete `structured_matcher_agrees_with_string_matcher` (its bodies are already covered by the existing direct-assertion tests once they route through `body_parts`) and keep `structured_matcher_preserves_invalid_snippet_errors` renamed to `malformed_snippets_report_invalid_snippet` — fold the old `malformed_captures_and_captures_in_ranges_are_invalid` and `captures_that_collapse_during_normalisation_are_invalid` cases into the same style.
- `expansion_preserves_snippet_quantifiers_for_resolution` (resolution.rs:306-314) becomes:

```rust
#[test]
fn expansion_preserves_snippet_quantifiers_for_resolution() {
    let expanded = expand_commentaria("[Alice]+ a Character.\n");

    let Value::Snippet(snippet) = &expanded.statements[0].subject else {
        panic!("expected snippet subject");
    };
    assert_eq!(snippet.source, "[Alice]+");
    assert_eq!(snippet.cardinality, Cardinality::OneOrMore);
    assert_eq!(snippet.parts, vec![SnippetPart::Text("Alice".into())]);
}
```

- [ ] **Step 5: Run the full workspace suite**

Run: `cargo test --workspace`
Expected: PASS. `json_snapshots` and `crates/snipx/tests/cli.rs` must pass **unchanged** — JSON sources and diagnostics are preserved. If a json snapshot changes, that is a regression in `source`/message threading; fix the code, don't regenerate.

- [ ] **Step 6: Gates and commit**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all`

```bash
git add -A crates
git commit -m "Thread structured snippet values from CST through expand into match/resolve"
```

---

### Task 5: Pin spec-aligned drift fixes, changelog, wrap up

**Files:**
- Test: `crates/snipx-core/tests/resolution.rs`
- Modify: `CHANGELOG.md`

The old string lexer deviated from the spec on inputs its tests never covered; the structured matcher fixes them. Pin the fixed behaviour:

- [ ] **Step 1: Write the tests**

```rust
#[test]
fn mid_body_quotes_are_literal_target_text() {
    // Spec: quotes delimit only when wrapping the entire body. The old
    // string matcher stripped any pattern that merely started and ended
    // with a quote character.
    let visible = extract_visible_text("\"a\" X \"b\"", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet(&body_parts("\"a\" {X} \"b\""), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 4, end: 5 }]
    );
}

#[test]
fn quoted_braces_in_range_endpoints_stay_literal() {
    // The old matcher re-lexed unquoted endpoint text and could mistake
    // previously-quoted braces for captures.
    let visible = extract_visible_text("{a} middle end", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet(&body_parts("\"{a}\"..end"), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 14 }]
    );
}

#[test]
fn quoted_empty_endpoint_is_open() {
    let visible = extract_visible_text("A to B", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet(&body_parts("\"\"..B"), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 6 }]
    );
}
```

- [ ] **Step 2: Run them**

Run: `cargo test -p snipx-core --test resolution`
Expected: PASS (they pin behaviour Task 4 already produces; investigate any failure as a Task 3/4 bug).

- [ ] **Step 3: Changelog**

Add under an `## [Unreleased]` heading in `CHANGELOG.md` (create the heading if absent, above `0.1.0`):

```markdown
### Changed

- Snippet bodies are now lexed once, by the parser: the matcher consumes
  the structured CST instead of re-lexing strings. Diagnostics and JSON
  output are unchanged for valid documents; a few pathological inputs
  (embedded quotes mid-body, quoted braces in range endpoints) now follow
  the spec's "quotes delimit only when they wrap an entire body or
  endpoint" rule instead of the old string re-lexer's approximations.
```

- [ ] **Step 4: Final gates and commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`

```bash
git add crates/snipx-core/tests/resolution.rs CHANGELOG.md
git commit -m "Pin spec-aligned snippet quoting behaviour; update changelog"
```

- [ ] **Step 5: Close out (session protocol)**

- `bd close snipx-2mv --reason="Matcher consumes structured CST snippet values; string re-lexers removed"` — note in the close reason that the parser-internal region scanners (`find_intralinea_close`, `intralinea_snippet_len`, `snippet_contains_range`) intentionally remain: they are the parser's own boundary pre-scan, now the only lexical grammar.
- Push branch and open a PR per the delivery-pr-default memory; run `bd dolt push` if beads changed without a git push.

---

## Self-review notes

- Spec coverage: quotes-delimit-only-when-wrapping (Tasks 3+5), capture rules (Task 3), range separator/openness (Tasks 1+3), cardinality (Tasks 2+4), diagnostics parity (Tasks 3+4 message tables).
- Type consistency: `SnippetPart`/`SnippetValue`/`Cardinality` defined in Task 2 and consumed by name in Tasks 3–5; `match_snippet_parts` exists only between Tasks 3 and 4 (renamed in Task 4 Step 3).
- Known judgement calls an implementer must not "fix" silently: `decoded` decodes only `\"`; endpoint/whole-body quote rule is single-Quoted-part; unterminated-snippet generic diagnostic fires only when the matcher finds no more specific error.
