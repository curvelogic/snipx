# Markdown Visible Text Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add canonical Markdown and Markdown-loose visible-text extraction, matching, diagnostics, JSON propagation, and CLI support.

**Architecture:** Parse Markdown with `pulldown-cmark` 0.13 and consume its offset event stream directly. Store an NFC canonical visible-text stream plus non-fatal extraction diagnostics; reuse the existing exact and loose matcher by mapping Markdown profiles to the corresponding matching policy.

**Tech Stack:** Rust 2021, `pulldown-cmark` 0.13, `unicode-normalization`, `serde`, `assert_cmd`, and the existing SnipX parser/expander/resolver pipeline.

## Global Constraints

- Headings, block quotes, list item text, code block text, inline code text, link text, and image alt text are visible.
- Link and image destinations, reference definitions, Markdown markers, and raw HTML tags are not visible.
- Inline HTML tags are omitted while surrounding Markdown text remains visible.
- Opaque raw HTML blocks are omitted and emit source-located warning diagnostics.
- Canonical visible text is NFC-normalised; loose transformations occur only while matching so spans remain Unicode-scalar offsets over the stored stream.
- `markdown` uses exact matching and `markdown-loose` uses the existing loose matching policy.
- Extraction warnings appear in canonical JSON and affect the CLI exit code only under `--strict`.
- Implement with tests first and preserve all existing plain-profile behavior.

---

### Task 1: Extract Canonical Markdown Text and Diagnostics

**Files:**
- Modify: `crates/snipx-core/Cargo.toml`
- Modify: `crates/snipx-core/src/diagnostic.rs`
- Modify: `crates/snipx-core/src/json.rs`
- Modify: `crates/snipx-core/src/visible_text.rs`
- Modify: `crates/snipx-core/tests/resolution.rs`

**Interfaces:**
- Consumes: `extract_visible_text(source: &str, profile: Profile) -> Result<VisibleText, Diagnostic>`
- Produces: `VisibleText { text, normalisation, profile, diagnostics }`
- Produces: `DiagnosticCode::RawHtmlOmitted`

- [ ] **Step 1: Add failing rendered-text and raw-HTML tests**

Append tests equivalent to:

```rust
#[test]
fn markdown_extracts_rendered_visible_text() {
    let source = concat!(
        "# Heading\n\n",
        "> Alice [opened](door.html) the door.\n\n",
        "- first item\n- `second` item\n\n",
        "```text\ncode block\n```\n\n",
        "![threshold](door.png)\n\n",
        "[reference]: hidden.html\n",
    );
    let visible = extract_visible_text(source, Profile::Markdown).unwrap();

    assert_eq!(
        visible.text,
        "Heading\nAlice opened the door.\nfirst item\nsecond item\ncode block\nthreshold\n"
    );
    assert!(visible.diagnostics.is_empty());
    assert!(!visible.text.contains("door.html"));
    assert!(!visible.text.contains("door.png"));
    assert!(!visible.text.contains("hidden.html"));
}

#[test]
fn markdown_omits_raw_html_with_source_located_warnings() {
    let source = "Before <span>Alice</span>.\n\n<div>\nHidden\n</div>\n";
    let visible = extract_visible_text(source, Profile::Markdown).unwrap();

    assert_eq!(visible.text, "Before Alice.\n");
    assert!(!visible.text.contains("<span>"));
    assert!(!visible.text.contains("Hidden"));
    assert!(visible.diagnostics.iter().all(|diagnostic| {
        diagnostic.code == DiagnosticCode::RawHtmlOmitted
            && diagnostic.severity == snipx_core::Severity::Warning
            && diagnostic.span.is_some()
    }));
}
```

- [ ] **Step 2: Run the focused tests and confirm the unsupported-profile failure**

Run:

```bash
cargo test -p snipx-core markdown_extracts_rendered_visible_text -- --exact
cargo test -p snipx-core markdown_omits_raw_html_with_source_located_warnings -- --exact
```

Expected: both tests fail because Markdown extraction returns `UnsupportedProfile`.

- [ ] **Step 3: Add the Markdown parser dependency and visible-text diagnostics**

Add:

```toml
pulldown-cmark = "0.13"
```

Add `RawHtmlOmitted` to `DiagnosticCode`, map it to the stable JSON code
`RAW_HTML_OMITTED` in `json.rs`, and add this field to `VisibleText`:

```rust
pub diagnostics: Vec<Diagnostic>,
```

Populate `diagnostics: Vec::new()` for both plain profiles.

- [ ] **Step 4: Implement offset-aware event extraction**

Implement `extract_markdown` using:

```rust
use pulldown_cmark::{Event, Parser, TagEnd};

fn extract_markdown(source: &str, profile: Profile) -> VisibleText {
    let mut text = String::new();
    let mut diagnostics = Vec::new();

    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Text(value) | Event::Code(value) => text.push_str(&value),
            Event::SoftBreak | Event::HardBreak => push_newline(&mut text),
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::BlockQuote(_)
                | TagEnd::CodeBlock
                | TagEnd::Item
                | TagEnd::TableRow,
            ) => push_newline(&mut text),
            Event::InlineHtml(_) | Event::Html(_) => diagnostics.push(Diagnostic {
                code: DiagnosticCode::RawHtmlOmitted,
                severity: Severity::Warning,
                message: "Raw HTML is omitted from Markdown visible text".to_owned(),
                span: Some(SourceSpan {
                    start: range.start,
                    end: range.end,
                }),
                related: Vec::new(),
            }),
            Event::Start(_)
            | Event::End(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::FootnoteReference(_)
            | Event::Rule
            | Event::TaskListMarker(_) => {}
        }
    }

    VisibleText {
        text: text.nfc().collect(),
        normalisation: "NFC",
        profile,
        diagnostics,
    }
}

fn push_newline(text: &mut String) {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
}
```

Route `Profile::Markdown | Profile::MarkdownLoose` to `extract_markdown`.
If the exact event output requires a boundary adjustment, preserve the
contract asserted by the tests rather than Markdown source whitespace.

- [ ] **Step 5: Run Task 1 tests and the existing resolution suite**

Run:

```bash
cargo test -p snipx-core --test resolution
```

Expected: all visible-text tests pass and plain-profile regressions remain green.

---

### Task 2: Reuse Exact and Loose Matching for Markdown

**Files:**
- Modify: `crates/snipx-core/src/match.rs`
- Modify: `crates/snipx-core/tests/resolution.rs`

**Interfaces:**
- Consumes: `VisibleText` produced by Task 1.
- Produces: `match_snippet` and `resolve` support for both Markdown profiles.

- [ ] **Step 1: Add failing exact, loose, and Unicode-span tests**

Append:

```rust
#[test]
fn markdown_profiles_resolve_against_rendered_text() {
    let exact = extract_visible_text(
        "# Café\n\nAlice **opened** the file.\n",
        Profile::Markdown,
    )
    .unwrap();
    assert_eq!(
        match_snippet("Alice opened", &exact, Profile::Markdown).unwrap(),
        vec![TextSpan { start: 5, end: 17 }]
    );

    let loose = extract_visible_text(
        "Alice\u{2014}opened\n\nthe \u{fb01}le.",
        Profile::MarkdownLoose,
    )
    .unwrap();
    assert_eq!(
        match_snippet(
            "Alice-opened the file",
            &loose,
            Profile::MarkdownLoose,
        )
        .unwrap(),
        vec![TextSpan { start: 0, end: 20 }]
    );
}
```

- [ ] **Step 2: Run the focused test and confirm matching is unsupported**

Run:

```bash
cargo test -p snipx-core markdown_profiles_resolve_against_rendered_text -- --exact
```

Expected: FAIL with `UnsupportedProfile` from `match_snippet`.

- [ ] **Step 3: Map Markdown profiles to existing matcher policies**

Remove the unsupported-profile early return and define looseness as:

```rust
let loose = matches!(profile, Profile::PlainLoose | Profile::MarkdownLoose);
```

Use that policy in `match_capture`; all range and capture matching continues
to flow through the same function.

- [ ] **Step 4: Run resolver and JSON suites**

Run:

```bash
cargo test -p snipx-core --test resolution
cargo test -p snipx-core --test json_snapshots
```

Expected: all tests pass.

---

### Task 3: Propagate Extraction Warnings Through Canonical JSON

**Files:**
- Modify: `crates/snipx-core/src/json.rs`
- Modify: `crates/snipx-core/tests/json_snapshots.rs`

**Interfaces:**
- Consumes: `VisibleText::diagnostics`.
- Produces: `RAW_HTML_OMITTED` JSON diagnostics alongside facts and resolutions.

- [ ] **Step 1: Add a failing canonical JSON warning test**

Append:

```rust
#[test]
fn markdown_export_includes_non_fatal_extraction_warnings() {
    let document = export_json(ExportRequest {
        source: "[Alice] a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Alice <span>waited</span>.\n".to_owned()),
        profile: Profile::Markdown,
        path: None,
        target_uri: Some("chapter.md".to_owned()),
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["target"]["profile"], "markdown");
    assert_eq!(value["resolutions"][0]["spans"][0], json!({"start": 0, "end": 5}));
    assert!(value["diagnostics"].as_array().unwrap().iter().any(|diagnostic| {
        diagnostic["code"] == "RAW_HTML_OMITTED"
            && diagnostic["severity"] == "warning"
            && diagnostic["span"].is_object()
    }));
}
```

- [ ] **Step 2: Run the focused test and confirm diagnostics are absent**

Run:

```bash
cargo test -p snipx-core markdown_export_includes_non_fatal_extraction_warnings -- --exact
```

Expected: FAIL because visible-text diagnostics are not merged into export diagnostics.

- [ ] **Step 3: Merge extraction diagnostics into resolution output**

When extraction succeeds in `export_json`, clone `visible.diagnostics` before
passing `visible` to `resolve`, then extend `resolved.diagnostics` with those
warnings. The stable JSON code was added in Task 1 so every intermediate
commit remains compilable.

- [ ] **Step 4: Run JSON and resolution suites**

Run:

```bash
cargo test -p snipx-core --test json_snapshots
cargo test -p snipx-core --test resolution
```

Expected: all tests pass.

---

### Task 4: Enable Markdown Profiles in the CLI

**Files:**
- Modify: `crates/snipx/src/main.rs`
- Modify: `crates/snipx/tests/cli.rs`

**Interfaces:**
- Consumes: Markdown-capable `export_json`.
- Produces: `--profile markdown` and `--profile markdown-loose` for `check`, `resolve`, and `export`.

- [ ] **Step 1: Replace the unsupported-profile test with success and strict-warning tests**

Add integration coverage using temporary source and target files:

```rust
#[test]
fn cli_resolves_markdown_and_strict_mode_rejects_html_warnings() {
    let source = temp_file("markdown-source", "[Alice] a Character.\n");
    let clean_target = temp_file("markdown-target", "# Alice\n\nShe waited.\n");
    let html_target = temp_file(
        "markdown-html-target",
        "Alice <span>waited</span>.\n",
    );

    let mut clean = Command::cargo_bin("snipx").unwrap();
    clean
        .args([
            "resolve",
            "-c",
            "--profile",
            "markdown",
            "--target",
            clean_target.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"profile\":\"markdown\""))
        .stdout(predicate::str::contains("\"start\":0,\"end\":5"));

    let mut warning = Command::cargo_bin("snipx").unwrap();
    warning
        .args([
            "export",
            "-c",
            "--profile",
            "markdown",
            "--strict",
            "--target",
            html_target.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("RAW_HTML_OMITTED"));

    for path in [source, clean_target, html_target] {
        std::fs::remove_file(path).unwrap();
    }
}
```

- [ ] **Step 2: Run the focused CLI test and confirm exit code 4**

Run:

```bash
cargo test -p snipx --test cli cli_resolves_markdown_and_strict_mode_rejects_html_warnings -- --exact
```

Expected: FAIL because `run_document` rejects both Markdown profiles before export.

- [ ] **Step 3: Remove the Markdown unsupported-profile guard**

Delete the `matches!(profile, Profile::Markdown | Profile::MarkdownLoose)`
early return from `run_document`. Keep exit code 4 support for future stable
unsupported diagnostics.

- [ ] **Step 4: Run all quality gates**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
git diff --check
```

Expected: formatting, clippy, every workspace test, and whitespace validation pass.

- [ ] **Step 5: Review and update Beads**

Request code review against this plan. Fix every Critical and Important
finding, rerun the full quality gates, then close `snipx-pmh.1` with a reason
that records Markdown extraction, raw-HTML diagnostics, matching, CLI support,
and the final verification result.
