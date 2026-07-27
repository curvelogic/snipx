use snipx_core::{
    expand, extract_visible_text, match_snippet, parse, resolve, DiagnosticCode, ExpandOptions,
    InputForm, ParseOptions, Profile, ResolveOptions, TextSpan, Value,
};

fn expand_commentaria(source: &str) -> snipx_core::ExpandResult {
    let parsed = parse(
        source,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    expand(&parsed, ExpandOptions::default())
}

#[test]
fn plain_visible_text_is_nfc_normalised() {
    let visible = extract_visible_text("Cafe\u{301}", Profile::Plain).unwrap();

    assert_eq!(visible.text, "Café");
    assert_eq!(visible.normalisation, "NFC");
}

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
    assert!(!visible.diagnostics.is_empty());
    assert!(visible.diagnostics.iter().all(|diagnostic| {
        diagnostic.code == DiagnosticCode::RawHtmlOmitted
            && diagnostic.severity == snipx_core::Severity::Warning
            && diagnostic.span.is_some()
    }));
}

#[test]
fn exact_matching_returns_unicode_scalar_offsets() {
    let visible = extract_visible_text("é Alice Alice", Profile::Plain).unwrap();
    let spans = match_snippet("Alice", &visible, Profile::Plain).unwrap();

    assert_eq!(
        spans,
        vec![
            TextSpan { start: 2, end: 7 },
            TextSpan { start: 8, end: 13 }
        ]
    );
}

#[test]
fn exact_matching_preserves_leading_and_trailing_whitespace() {
    let visible = extract_visible_text("x Alice yAlice", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet(" Alice ", &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 1, end: 8 }]
    );
}

#[test]
fn loose_matching_collapses_whitespace_and_typography() {
    let visible = extract_visible_text(
        "Alice\u{2014}opened\n\nthe \u{fb01}le.",
        Profile::PlainLoose,
    )
    .unwrap();
    let spans = match_snippet("Alice-opened the file", &visible, Profile::PlainLoose).unwrap();

    assert_eq!(spans, vec![TextSpan { start: 0, end: 21 }]);
}

#[test]
fn matches_closed_and_open_ranges() {
    let visible = extract_visible_text("Start middle End tail", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet("Start..End", &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 16 }]
    );
    assert_eq!(
        match_snippet("..End", &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 16 }]
    );
    assert_eq!(
        match_snippet("middle..", &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 6, end: 21 }]
    );
    assert_eq!(
        match_snippet("", &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 21 }]
    );
}

#[test]
fn range_matching_is_leftmost_first_and_non_overlapping() {
    let visible = extract_visible_text("A A B A B", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet("A..B", &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 5 }, TextSpan { start: 6, end: 9 }]
    );
}

#[test]
fn open_range_matching_is_non_overlapping() {
    let visible = extract_visible_text("A End End", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet("..End", &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 5 }]
    );
    assert_eq!(
        match_snippet("A..", &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 9 }]
    );
}

#[test]
fn quoted_snippet_text_treats_range_syntax_as_literal() {
    let visible = extract_visible_text("A..B then A to B", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet("\"A..B\"", &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 4 }]
    );
}

#[test]
fn malformed_captures_and_captures_in_ranges_are_invalid() {
    let visible = extract_visible_text("A to B", Profile::Plain).unwrap();

    for body in ["{A}..B", "A {to B", "A {} B", "\"unterminated"] {
        let diagnostic = match_snippet(body, &visible, Profile::Plain).unwrap_err();
        assert_eq!(diagnostic.code, DiagnosticCode::InvalidSnippet, "{body:?}");
    }
}

#[test]
fn capture_context_matches_whole_expression_but_returns_capture() {
    let visible = extract_visible_text(
        "Alice waited. Bob looked at Alice and smiled.",
        Profile::Plain,
    )
    .unwrap();
    let spans = match_snippet("looked at {Alice} and smiled", &visible, Profile::Plain).unwrap();

    assert_eq!(spans, vec![TextSpan { start: 28, end: 33 }]);
}

#[test]
fn capture_boundaries_follow_nfc_normalisation() {
    let visible = extract_visible_text("Café Alice", Profile::Plain).unwrap();
    let spans = match_snippet("Cafe\u{301} {Alice}", &visible, Profile::Plain).unwrap();

    assert_eq!(spans, vec![TextSpan { start: 5, end: 10 }]);
}

#[test]
fn captures_that_collapse_during_normalisation_are_invalid() {
    let exact = extract_visible_text("é", Profile::Plain).unwrap();
    let loose = extract_visible_text("A B", Profile::PlainLoose).unwrap();

    for (body, visible, profile) in [
        ("e{\u{301}}", &exact, Profile::Plain),
        ("A { } B", &loose, Profile::PlainLoose),
    ] {
        let diagnostic = match_snippet(body, visible, profile).unwrap_err();
        assert_eq!(diagnostic.code, DiagnosticCode::InvalidSnippet);
    }
}

#[test]
fn loose_expansions_do_not_create_duplicate_source_spans() {
    let visible = extract_visible_text("\u{fb00}", Profile::PlainLoose).unwrap();

    assert_eq!(
        match_snippet("f", &visible, Profile::PlainLoose).unwrap(),
        vec![TextSpan { start: 0, end: 1 }]
    );
}

#[test]
fn markdown_profiles_resolve_against_rendered_text() {
    let exact =
        extract_visible_text("# Café\n\nAlice **opened** the file.\n", Profile::Markdown).unwrap();
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
        match_snippet("Alice-opened the file", &loose, Profile::MarkdownLoose,).unwrap(),
        vec![TextSpan { start: 0, end: 20 }]
    );
}

#[test]
fn expansion_preserves_snippet_quantifiers_for_resolution() {
    let expanded = expand_commentaria("[Alice]+ a Character.\n");

    assert_eq!(
        expanded.statements[0].subject,
        Value::Snippet("[Alice]+".into())
    );
}

#[test]
fn resolver_enforces_default_and_quantified_cardinality() {
    let visible = extract_visible_text("Alice met Alice.", Profile::Plain).unwrap();

    let ambiguous = resolve(
        &expand_commentaria("[Alice] a Character.\n"),
        &visible,
        ResolveOptions::default(),
    );
    assert_eq!(ambiguous.resolutions.len(), 0);
    assert!(ambiguous
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::SnippetAmbiguous));

    let repeated = resolve(
        &expand_commentaria("[Alice]+ a Character.\n"),
        &visible,
        ResolveOptions::default(),
    );
    assert_eq!(
        repeated.resolutions[0].spans,
        vec![
            TextSpan { start: 0, end: 5 },
            TextSpan { start: 10, end: 15 }
        ]
    );
}

#[test]
fn unresolved_snippet_remains_in_partial_result() {
    let visible = extract_visible_text("Bob waited.", Profile::Plain).unwrap();
    let resolved = resolve(
        &expand_commentaria("[Alice] a Character.\n"),
        &visible,
        ResolveOptions::default(),
    );

    assert_eq!(
        resolved.diagnostics[0].code,
        DiagnosticCode::SnippetNotFound
    );
    assert_eq!(
        resolved.statements[0].subject,
        Value::Unresolved("[Alice]".into())
    );
    assert_eq!(
        resolved.diagnostics[0].span,
        Some(snipx_core::SourceSpan { start: 0, end: 7 })
    );
}

#[test]
fn star_allows_zero_matches_and_question_rejects_multiple() {
    let visible = extract_visible_text("Alice met Alice.", Profile::Plain).unwrap();

    let zero = resolve(
        &expand_commentaria("[Bob]* a Character.\n"),
        &visible,
        ResolveOptions::default(),
    );
    assert!(zero.diagnostics.is_empty());
    assert_eq!(zero.resolutions[0].spans, Vec::<TextSpan>::new());

    let too_many = resolve(
        &expand_commentaria("[Alice]? a Character.\n"),
        &visible,
        ResolveOptions::default(),
    );
    assert!(too_many
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::SnippetAmbiguous));
}
