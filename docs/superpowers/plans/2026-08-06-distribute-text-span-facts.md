# Distribute Quantified Text-Span Facts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make text-span snippets (`~[...]`) emit one fact per matched span, per the language spec's "Denotation And Text Spans" section (issue snipx-2x0).

**Architecture:** Distribution happens in `resolve()`: after a text-span snippet resolves, the statement is replicated once per matched span (Cartesian product when both subject and object distribute). A new post-resolution `Value::ResolvedTextSpan { snippet, span }` variant carries the concrete span into JSON export, where `textSpanSnippet` values gain an optional `span` field in visible-text scalar offsets.

**Tech Stack:** Rust workspace; `snipx-core` library crate + `snipx` CLI crate. Tests: `cargo test`. Lint: `cargo clippy --all-targets`, `cargo fmt --check`.

**Spec:** `docs/superpowers/specs/2026-08-06-distribute-text-span-facts-design.md`

## Global Constraints

- Denotational snippets (`[...]` without `~`), the `resolutions` array, all diagnostics, and the no-target-text export path must be byte-identical to current behaviour.
- `span` on `textSpanSnippet` fact values uses visible-text Unicode scalar offsets (the same unit as `resolutions[].spans`), never source byte offsets.
- Text-span local subjects (`~<` etc.) are out of scope: their fact values stay span-less.
- Run all commands from the repo root (`cargo test -p snipx-core` etc. work from there).

---

### Task 1: Distribution in resolve

**Files:**
- Modify: `crates/snipx-core/src/expand.rs` (Value enum, ~line 8-22)
- Modify: `crates/snipx-core/src/resolve.rs` (resolve loop ~line 38-74, resolve_value ~line 76-153)
- Test: `crates/snipx-core/tests/resolution.rs`

**Interfaces:**
- Consumes: existing `resolve(&ExpandResult, &VisibleText, ResolveOptions) -> ResolveResult`, `Value::TextSpanSnippet(SnippetValue)`, `TextSpan { start: usize, end: usize }` (Copy) from `crate::r#match`.
- Produces: new enum variant `Value::ResolvedTextSpan { snippet: SnippetValue, span: TextSpan }` in `snipx_core::expand::Value` (re-exported from crate root). `resolve()` signature unchanged; its `statements` output may now contain more or fewer statements than the input.

- [ ] **Step 1: Write the failing tests**

Append to `crates/snipx-core/tests/resolution.rs`:

```rust
#[test]
fn quantified_text_span_snippet_distributes_one_statement_per_span() {
    let visible = extract_visible_text("Alice met Alice.", Profile::Plain).unwrap();
    let resolved = resolve(
        &expand_commentaria("~[Alice]+ highlight true.\n"),
        &visible,
        ResolveOptions::default(),
    );

    assert!(resolved.diagnostics.is_empty());
    assert_eq!(resolved.statements.len(), 2);
    let spans: Vec<TextSpan> = resolved
        .statements
        .iter()
        .map(|statement| {
            let Value::ResolvedTextSpan { snippet, span } = &statement.subject else {
                panic!("expected resolved text-span subject");
            };
            assert_eq!(snippet.source, "[Alice]+");
            *span
        })
        .collect();
    assert_eq!(
        spans,
        vec![
            TextSpan { start: 0, end: 5 },
            TextSpan { start: 10, end: 15 }
        ]
    );
    // The resolutions array is unchanged: one entry listing all spans.
    assert_eq!(resolved.resolutions.len(), 1);
    assert_eq!(resolved.resolutions[0].spans.len(), 2);
}

#[test]
fn both_sides_text_span_distribution_is_cartesian() {
    let visible = extract_visible_text("A A B B", Profile::Plain).unwrap();
    let resolved = resolve(
        &expand_commentaria("~[A]+ before ~[B]+.\n"),
        &visible,
        ResolveOptions::default(),
    );

    assert!(resolved.diagnostics.is_empty());
    let pairs: Vec<(TextSpan, TextSpan)> = resolved
        .statements
        .iter()
        .map(|statement| {
            let Value::ResolvedTextSpan { span: subject, .. } = &statement.subject else {
                panic!("expected resolved text-span subject");
            };
            let Value::ResolvedTextSpan { span: object, .. } = &statement.object else {
                panic!("expected resolved text-span object");
            };
            (*subject, *object)
        })
        .collect();
    let span = |start, end| TextSpan { start, end };
    assert_eq!(
        pairs,
        vec![
            (span(0, 1), span(4, 5)),
            (span(0, 1), span(6, 7)),
            (span(2, 3), span(4, 5)),
            (span(2, 3), span(6, 7)),
        ]
    );
}

#[test]
fn zero_match_star_text_span_produces_no_statements() {
    let visible = extract_visible_text("Alice waited.", Profile::Plain).unwrap();
    let resolved = resolve(
        &expand_commentaria("~[Bob]* highlight true.\n"),
        &visible,
        ResolveOptions::default(),
    );

    assert!(resolved.diagnostics.is_empty());
    assert!(resolved.statements.is_empty());
    assert_eq!(resolved.resolutions[0].spans, Vec::<TextSpan>::new());
}

#[test]
fn unquantified_text_span_snippet_carries_its_span() {
    let visible = extract_visible_text("Alice waited.", Profile::Plain).unwrap();
    let resolved = resolve(
        &expand_commentaria("~[Alice] italic true.\n"),
        &visible,
        ResolveOptions::default(),
    );

    assert_eq!(resolved.statements.len(), 1);
    let Value::ResolvedTextSpan { span, .. } = &resolved.statements[0].subject else {
        panic!("expected resolved text-span subject");
    };
    assert_eq!(*span, TextSpan { start: 0, end: 5 });
}

#[test]
fn quantified_denotational_snippet_still_collapses_to_one_statement() {
    let visible = extract_visible_text("Alice met Alice.", Profile::Plain).unwrap();
    let resolved = resolve(
        &expand_commentaria("[Alice]+ a Character.\n"),
        &visible,
        ResolveOptions::default(),
    );

    assert_eq!(resolved.statements.len(), 1);
    assert!(matches!(resolved.statements[0].subject, Value::Snippet(_)));
}

#[test]
fn decorations_on_text_span_subjects_distribute() {
    let visible = extract_visible_text("Alice met Alice.", Profile::Plain).unwrap();
    let resolved = resolve(
        &expand_commentaria("~[Alice]+ ::\"note\".\n"),
        &visible,
        ResolveOptions::default(),
    );

    assert!(resolved.diagnostics.is_empty());
    // The decoration expands to one note statement, distributed over both spans.
    assert_eq!(resolved.statements.len(), 2);
    for statement in &resolved.statements {
        assert!(matches!(
            statement.subject,
            Value::ResolvedTextSpan { .. }
        ));
        assert_eq!(statement.predicate, Value::Predicate("note".into()));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p snipx-core --test resolution`
Expected: compile error — `Value` has no variant `ResolvedTextSpan`.

- [ ] **Step 3: Add the `ResolvedTextSpan` variant**

In `crates/snipx-core/src/expand.rs`, add the import and variant:

```rust
use crate::r#match::TextSpan;
```

and in `pub enum Value`, immediately after `TextSpanSnippet(SnippetValue)`:

```rust
    /// A text-span snippet after resolution, pinned to one concrete
    /// matched span. Produced only by resolve, never by expand.
    ResolvedTextSpan {
        snippet: SnippetValue,
        span: TextSpan,
    },
```

Expect non-exhaustive-match compile errors in `json.rs` only; add a temporary arm there in this task so the crate compiles (it becomes the real arm in Task 2):

```rust
        Value::ResolvedTextSpan { snippet, .. } => JsonValue::TextSpanSnippet {
            source: snippet.source,
        },
```

- [ ] **Step 4: Rework the resolve loop to distribute**

In `crates/snipx-core/src/resolve.rs`, replace the body of `resolve()` (keeping its signature) with:

```rust
pub fn resolve(
    expanded: &ExpandResult,
    visible_text: &VisibleText,
    options: ResolveOptions,
) -> ResolveResult {
    let profile = options.profile.unwrap_or(visible_text.profile);
    let mut result = ResolveResult {
        statements: Vec::new(),
        resolutions: Vec::new(),
        diagnostics: expanded.diagnostics.clone(),
    };

    for statement in &expanded.statements {
        let mut statement = statement.clone();
        let subject_span = statement.subject_span.clone();
        let subject_spans = resolve_value(
            &mut statement.subject,
            subject_span,
            visible_text,
            profile,
            &options.intralinea_anchors,
            &mut result.resolutions,
            &mut result.diagnostics,
        );
        let object_span = statement.object_span.clone();
        let object_spans = resolve_value(
            &mut statement.object,
            object_span,
            visible_text,
            profile,
            &options.intralinea_anchors,
            &mut result.resolutions,
            &mut result.diagnostics,
        );
        distribute(statement, subject_spans, object_spans, &mut result.statements);
    }

    result
}
```

Change `resolve_value` to return `Option<Vec<TextSpan>>` — `Some(spans)` only when the value is a `Value::TextSpanSnippet` that resolved successfully, `None` in every other case (non-snippets, local subjects, denotational snippets, and all error paths). The only edits to its body: capture whether the value is a text-span snippet, clone the spans into the resolution, and return them. The full revised function:

```rust
fn resolve_value(
    value: &mut Value,
    source_span: Option<crate::SourceSpan>,
    visible_text: &VisibleText,
    profile: Profile,
    anchors: &[IntralineaAnchor],
    resolutions: &mut Vec<SnippetResolution>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<TextSpan>> {
    if let Value::LocalSubject(local) = value {
        let local = local.clone();
        resolve_local_subject(
            value,
            &local,
            source_span,
            visible_text,
            anchors,
            resolutions,
            diagnostics,
        );
        return None;
    }
    let text_span = matches!(value, Value::TextSpanSnippet(_));
    let snippet = match value {
        Value::Snippet(snippet) | Value::TextSpanSnippet(snippet) => snippet.clone(),
        _ => return None,
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
            return None;
        }
        Ok(spans) => spans,
        Err(mut error) => {
            if error.span.is_none() {
                error.span = source_span;
            }
            diagnostics.push(error);
            *value = Value::Unresolved(snippet.source);
            return None;
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
        return None;
    }

    resolutions.push(SnippetResolution {
        source: snippet.source,
        source_span,
        spans: spans.clone(),
    });
    text_span.then_some(spans)
}
```

Then add the distribution helpers at module level:

```rust
/// Spec (Denotation And Text Spans): text-span snippets distribute one
/// fact per matched span; both sides distributing yields the Cartesian
/// product. Denotational values pass through as a single alternative.
fn distribute(
    statement: ExpandedStatement,
    subject_spans: Option<Vec<TextSpan>>,
    object_spans: Option<Vec<TextSpan>>,
    statements: &mut Vec<ExpandedStatement>,
) {
    let subjects = value_alternatives(statement.subject.clone(), subject_spans);
    let objects = value_alternatives(statement.object.clone(), object_spans);
    for subject in &subjects {
        for object in &objects {
            let mut replica = statement.clone();
            replica.subject = subject.clone();
            replica.object = object.clone();
            statements.push(replica);
        }
    }
}

fn value_alternatives(value: Value, spans: Option<Vec<TextSpan>>) -> Vec<Value> {
    match (value, spans) {
        (Value::TextSpanSnippet(snippet), Some(spans)) => spans
            .into_iter()
            .map(|span| Value::ResolvedTextSpan {
                snippet: snippet.clone(),
                span,
            })
            .collect(),
        (value, _) => vec![value],
    }
}
```

- [ ] **Step 5: Run the resolution tests**

Run: `cargo test -p snipx-core --test resolution`
Expected: all PASS (new and pre-existing — pre-existing tests pin that denotational behaviour, diagnostics, and resolutions are unchanged).

- [ ] **Step 6: Run the full test suite**

Run: `cargo test`
Expected: everything passes except possibly snapshot/CLI expectations that pin the OLD one-fact behaviour for text-span snippets. If any fail, inspect: failures are acceptable ONLY where the old behaviour was the spec non-conformance being fixed; update those expectations to the distributed form. Anything else failing means the change broke an invariant — stop and fix.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Distribute resolved text-span snippet statements per matched span"
```

---

### Task 2: JSON representation and canonical-json docs

**Files:**
- Modify: `crates/snipx-core/src/json.rs` (`JsonValue::TextSpanSnippet` ~line 131, `json_value` ~line 377-382)
- Modify: `docs/canonical-json.md`
- Test: `crates/snipx-core/tests/json_snapshots.rs`

**Interfaces:**
- Consumes: `Value::ResolvedTextSpan { snippet: SnippetValue, span: TextSpan }` from Task 1; existing `json_text_span(TextSpan) -> JsonSpan`.
- Produces: `JsonValue::TextSpanSnippet { source: String, span: Option<JsonSpan> }` — `span` omitted from serialisation when `None`.

- [ ] **Step 1: Write the failing test**

Append to `crates/snipx-core/tests/json_snapshots.rs`:

```rust
#[test]
fn quantified_text_span_facts_distribute_per_span() {
    let document = export_json(ExportRequest {
        source: "~[Alice]+ highlight true.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Alice met Alice.".to_owned()),
        profile: Some(Profile::Plain),
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(
        value,
        json!({
            "snipxVersion": "0.0",
            "implementation": {
                "name": "snipx",
                "version": "0.1.1"
            },
            "input": {
                "form": "commentaria"
            },
            "target": {
                "profile": "plain"
            },
            "visibleText": {
                "normalisation": "NFC",
                "length": 16
            },
            "facts": [
                {
                    "subject": {
                        "kind": "textSpanSnippet",
                        "source": "[Alice]+",
                        "span": {"start": 0, "end": 5}
                    },
                    "predicate": {
                        "kind": "predicate",
                        "value": "highlight"
                    },
                    "object": {
                        "kind": "boolean",
                        "value": true
                    },
                    "source": {
                        "statement": {"start": 0, "end": 25},
                        "subject": {"start": 0, "end": 9},
                        "predicate": {"start": 10, "end": 19},
                        "object": {"start": 20, "end": 24}
                    }
                },
                {
                    "subject": {
                        "kind": "textSpanSnippet",
                        "source": "[Alice]+",
                        "span": {"start": 10, "end": 15}
                    },
                    "predicate": {
                        "kind": "predicate",
                        "value": "highlight"
                    },
                    "object": {
                        "kind": "boolean",
                        "value": true
                    },
                    "source": {
                        "statement": {"start": 0, "end": 25},
                        "subject": {"start": 0, "end": 9},
                        "predicate": {"start": 10, "end": 19},
                        "object": {"start": 20, "end": 24}
                    }
                }
            ],
            "resolutions": [{
                "source": "[Alice]+",
                "sourceSpan": {"start": 0, "end": 9},
                "spans": [
                    {"start": 0, "end": 5},
                    {"start": 10, "end": 15}
                ]
            }],
            "diagnostics": []
        })
    );
}
```

If the assertion fails only on `source` span offsets (statement/subject/predicate/object), trust the actual values printed by the test — they are byte offsets produced by the parser — and update the expected JSON accordingly. Every other field must match as written.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p snipx-core --test json_snapshots quantified_text_span_facts_distribute_per_span`
Expected: FAIL — two facts are present (Task 1) but neither carries a `span` field.

- [ ] **Step 3: Add the optional span to the JSON value**

In `crates/snipx-core/src/json.rs`, change the `TextSpanSnippet` variant of `JsonValue`:

```rust
    TextSpanSnippet {
        source: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        span: Option<JsonSpan>,
    },
```

In `json_value`, update the two arms (replacing the temporary arm from Task 1):

```rust
        Value::TextSpanSnippet(snippet) => JsonValue::TextSpanSnippet {
            source: snippet.source,
            span: None,
        },
        Value::ResolvedTextSpan { snippet, span } => JsonValue::TextSpanSnippet {
            source: snippet.source,
            span: Some(json_text_span(span)),
        },
```

- [ ] **Step 4: Run the test suite**

Run: `cargo test`
Expected: all PASS. The CLI test `ambient_values_use_the_core_expression_grammar` (crates/snipx/tests/cli.rs:526) pins the span-less no-target output `{"kind":"textSpanSnippet","source":"[Alice]"}` and must still pass unchanged.

- [ ] **Step 5: Document the schema change**

In `docs/canonical-json.md`, "Value kinds" section, append after the existing paragraph (ending "…the other kinds carry a decoded `value`."):

```markdown
A resolved `textSpanSnippet` value also carries the concrete matched
`span` it distributes over. Text-span snippets distribute: a statement
whose subject or object is a text-span snippet emits one fact per
matched span (the Cartesian product when both sides are text-span
snippets), each fact's value carrying its own `span`. Denotational
`snippet` values collapse to a single fact and never carry `span`.
`span` is omitted when resolution did not run (no target text).
```

And in "Span offset conventions", extend the resolution-spans paragraph's first sentence:

```markdown
**Resolution spans are Unicode scalar offsets.** `resolutions[].spans`
and the `span` field on `textSpanSnippet` fact values address the
*canonical visible text* of the target document — after extraction and
NFC normalisation — counted in Unicode scalar values (Rust `char`s),
not bytes.
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Carry matched span on distributed textSpanSnippet fact values"
```

---

### Task 3: End-to-end CLI test and changelog

**Files:**
- Test: `crates/snipx/tests/cli.rs`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: the complete pipeline from Tasks 1-2 via the `snipx export` binary; existing test helpers `temp_file(name, contents)` and `Command::cargo_bin("snipx")`.
- Produces: nothing consumed later; final verification gate.

- [ ] **Step 1: Write the CLI test**

Append to `crates/snipx/tests/cli.rs`:

```rust
#[test]
fn quantified_text_span_snippets_export_one_fact_per_span() {
    let target = temp_file("distribute-target", "Alice met Alice.");
    let mut command = Command::cargo_bin("snipx").expect("snipx binary should build");

    command
        .arg("export")
        .arg("--target")
        .arg(&target)
        .write_stdin("~[Alice]+ highlight true.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"span\":{\"start\":0,\"end\":5}",
        ))
        .stdout(predicate::str::contains(
            "\"span\":{\"start\":10,\"end\":15}",
        ));
}
```

- [ ] **Step 2: Run it (expected to pass — it exercises Tasks 1-2 end to end)**

Run: `cargo test -p snipx --test cli quantified_text_span_snippets_export_one_fact_per_span`
Expected: PASS. If it fails on the serialised span format, print the actual stdout (`--nocapture`), confirm the two spans appear once each, and align the contains-strings with the actual serialisation — but the two facts themselves must be present.

- [ ] **Step 3: Add the changelog entry**

In `CHANGELOG.md`, under `## [Unreleased]`, add:

```markdown
### Fixed

- Text-span snippets (`~[...]`) now distribute one fact per matched
  span, as the spec's "Denotation And Text Spans" section requires
  (Cartesian product when both subject and object are text-span
  snippets). Each distributed `textSpanSnippet` fact value carries its
  concrete `span` in visible-text scalar offsets; quantified
  denotational snippets still collapse to a single fact.
```

- [ ] **Step 4: Run quality gates**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: all clean.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "Pin text-span fact distribution end to end; changelog"
```
