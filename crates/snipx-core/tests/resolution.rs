use snipx_core::{
    expand, extract_visible_text, match_snippet, parse, resolve, Cardinality, DiagnosticCode,
    ExpandOptions, InputForm, ParseOptions, Profile, ResolveOptions, SnippetPart, SnippetValue,
    SyntaxKind, TextSpan, Value,
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
fn markdown_separates_tight_nested_list_items() {
    let visible = extract_visible_text("- one\n  - two\n- three\n", Profile::Markdown).unwrap();

    assert_eq!(visible.text, "one\ntwo\nthree\n");
}

#[test]
fn markdown_separates_block_quote_nested_in_tight_list_item() {
    let visible = extract_visible_text("- one\n  > two\n- three\n", Profile::Markdown).unwrap();

    assert_eq!(visible.text, "one\ntwo\nthree\n");
}

#[test]
fn markdown_separates_heading_nested_in_tight_list_item() {
    let visible = extract_visible_text("- one\n  # two\n- three\n", Profile::Markdown).unwrap();

    assert_eq!(visible.text, "one\ntwo\nthree\n");
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
fn markdown_inlines_footnote_text_at_reference_point() {
    let source = "Alice[^1] went home.\n\n[^1]: The heroine.\n";
    let visible = extract_visible_text(source, Profile::Markdown).unwrap();

    // The definition text appears at the reference point, delimited
    // like any other block, and is not repeated at its definition site.
    assert_eq!(visible.text, "Alice\nThe heroine.\n went home.\n");
    assert!(visible.diagnostics.is_empty());

    // Spans returned by matching are Unicode-scalar offsets into the
    // inlined visible text.
    assert_eq!(
        match_snippet(&body_parts("The heroine"), &visible, Profile::Markdown).unwrap(),
        vec![TextSpan { start: 6, end: 17 }]
    );
    assert_eq!(
        match_snippet(&body_parts("went home"), &visible, Profile::Markdown).unwrap(),
        vec![TextSpan { start: 20, end: 29 }]
    );
}

#[test]
fn markdown_inlines_footnote_text_at_every_reference() {
    let source = "A[^n] and B[^n].\n\n[^n]: note\n";
    let visible = extract_visible_text(source, Profile::Markdown).unwrap();

    assert_eq!(visible.text, "A\nnote\n and B\nnote\n.\n");
    assert_eq!(
        match_snippet(&body_parts("note"), &visible, Profile::Markdown).unwrap(),
        vec![
            TextSpan { start: 2, end: 6 },
            TextSpan { start: 14, end: 18 }
        ]
    );
}

#[test]
fn markdown_keeps_undefined_footnote_reference_as_literal_text() {
    let visible = extract_visible_text("See[^missing] here.\n", Profile::Markdown).unwrap();

    // GitHub-style footnotes: a reference with no definition is not
    // footnote syntax at all, so its characters stay visible verbatim.
    assert_eq!(visible.text, "See[^missing] here.\n");
    assert!(visible.diagnostics.is_empty());
    assert_eq!(
        match_snippet(&body_parts("here"), &visible, Profile::Markdown).unwrap(),
        vec![TextSpan { start: 14, end: 18 }]
    );
}

#[test]
fn markdown_footnote_self_reference_is_inlined_once() {
    let source = "X[^a].\n\n[^a]: sees [^a] itself\n";
    let visible = extract_visible_text(source, Profile::Markdown).unwrap();

    // A cyclic reference cannot re-inline the footnote currently being
    // inserted; the inner reference contributes nothing.
    assert_eq!(visible.text, "X\nsees  itself\n.\n");
}

#[test]
fn markdown_first_footnote_definition_wins() {
    let source = "R[^d].\n\n[^d]: first\n\n[^d]: second\n";
    let visible = extract_visible_text(source, Profile::Markdown).unwrap();

    assert_eq!(visible.text, "R\nfirst\n.\n");
}

#[test]
fn markdown_inlines_multi_block_footnote_definitions() {
    let source = "P[^m].\n\n[^m]: one\n\n    two\n";
    let visible = extract_visible_text(source, Profile::Markdown).unwrap();

    assert_eq!(visible.text, "P\none\ntwo\n.\n");
}

#[test]
fn markdown_renders_table_rows_and_cells() {
    let source = "| Name | Age |\n| --- | --- |\n| Alice | 30 |\n| Bob | 41 |\n";
    let visible = extract_visible_text(source, Profile::Markdown).unwrap();

    // Cells are space-separated within a row; rows are newline-separated
    // like every other block boundary.
    assert_eq!(visible.text, "Name Age\nAlice 30\nBob 41\n");
    assert!(visible.diagnostics.is_empty());
    assert_eq!(
        match_snippet(&body_parts("Alice 30"), &visible, Profile::Markdown).unwrap(),
        vec![TextSpan { start: 9, end: 17 }]
    );
}

#[test]
fn markdown_table_empty_cells_do_not_double_separators() {
    let source = "| a |  | b |\n| - | - | - |\n| 1 |  | 2 |\n";
    let visible = extract_visible_text(source, Profile::Markdown).unwrap();

    assert_eq!(visible.text, "a b\n1 2\n");
}

#[test]
fn footnote_and_table_extraction_is_consistent_across_profiles() {
    let source = "T[^1] | left | right |\n\n[^1]: note\n\n| a | b |\n| - | - |\n| c | d |\n";

    // Both Markdown profiles extract identical visible text.
    let exact = extract_visible_text(source, Profile::Markdown).unwrap();
    let loose = extract_visible_text(source, Profile::MarkdownLoose).unwrap();
    assert_eq!(exact.text, loose.text);
    assert_eq!(exact.text, "T\nnote\n | left | right |\na b\nc d\n");

    // Plain profiles keep Markdown syntax literal (NFC only).
    let plain = extract_visible_text(source, Profile::Plain).unwrap();
    let plain_loose = extract_visible_text(source, Profile::PlainLoose).unwrap();
    assert_eq!(plain.text, source);
    assert_eq!(plain_loose.text, source);
}

#[test]
fn exact_matching_returns_unicode_scalar_offsets() {
    let visible = extract_visible_text("é Alice Alice", Profile::Plain).unwrap();
    let spans = match_snippet(&body_parts("Alice"), &visible, Profile::Plain).unwrap();

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
        match_snippet(&body_parts(" Alice "), &visible, Profile::Plain).unwrap(),
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
    let spans = match_snippet(
        &body_parts("Alice-opened the file"),
        &visible,
        Profile::PlainLoose,
    )
    .unwrap();

    assert_eq!(spans, vec![TextSpan { start: 0, end: 21 }]);
}

#[test]
fn matches_closed_and_open_ranges() {
    let visible = extract_visible_text("Start middle End tail", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet(&body_parts("Start..End"), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 16 }]
    );
    assert_eq!(
        match_snippet(&body_parts("..End"), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 16 }]
    );
    assert_eq!(
        match_snippet(&body_parts("middle.."), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 6, end: 21 }]
    );
    assert_eq!(
        match_snippet(&body_parts(""), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 21 }]
    );
}

#[test]
fn range_matching_is_leftmost_first_and_non_overlapping() {
    let visible = extract_visible_text("A A B A B", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet(&body_parts("A..B"), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 5 }, TextSpan { start: 6, end: 9 }]
    );
}

#[test]
fn open_ranges_return_every_candidate_for_cardinality_checks() {
    let visible = extract_visible_text("A End End", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet(&body_parts("..End"), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 5 }, TextSpan { start: 0, end: 9 }]
    );
    assert_eq!(
        match_snippet(&body_parts("A.."), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 9 }]
    );
}

#[test]
fn ambiguous_open_range_is_a_resolution_error() {
    let visible = extract_visible_text("A End End", Profile::Plain).unwrap();
    let parsed = parse(
        "[..End] a Region.\n",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let expanded = expand(&parsed, ExpandOptions::default());
    let resolved = resolve(&expanded, &visible, ResolveOptions::default());

    assert!(resolved
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::SnippetAmbiguous));
}

#[test]
fn range_endpoints_may_not_overlap() {
    let visible = extract_visible_text("abcd", Profile::Plain).unwrap();

    // The end match must begin at or after the end of the start match.
    assert_eq!(
        match_snippet(&body_parts("abc..bcd"), &visible, Profile::Plain).unwrap(),
        Vec::<TextSpan>::new()
    );
    // Exact adjacency is allowed.
    assert_eq!(
        match_snippet(&body_parts("ab..cd"), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 4 }]
    );
}

#[test]
fn quotes_inside_a_snippet_body_match_literally() {
    let visible = extract_visible_text("said \"sic\" loudly", Profile::Plain).unwrap();

    // Mid-body quotes are literal target text, not delimiters.
    assert_eq!(
        match_snippet(&body_parts("said \"sic\" loudly"), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 17 }]
    );
    // Without quotes in the target, the quoted-looking body does not match.
    let unquoted = extract_visible_text("said sic loudly", Profile::Plain).unwrap();
    assert_eq!(
        match_snippet(
            &body_parts("said \"sic\" loudly"),
            &unquoted,
            Profile::Plain
        )
        .unwrap(),
        Vec::<TextSpan>::new()
    );
    // Whole-body quotes still delimit.
    let bracketed = extract_visible_text("well [sic] indeed", Profile::Plain).unwrap();
    assert_eq!(
        match_snippet(&body_parts("\"[sic]\""), &bracketed, Profile::Plain).unwrap(),
        vec![TextSpan { start: 5, end: 10 }]
    );
}

#[test]
fn quoted_snippet_text_treats_range_syntax_as_literal() {
    let visible = extract_visible_text("A..B then A to B", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet(&body_parts("\"A..B\""), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 0, end: 4 }]
    );
}

#[test]
fn capture_context_matches_whole_expression_but_returns_capture() {
    let visible = extract_visible_text(
        "Alice waited. Bob looked at Alice and smiled.",
        Profile::Plain,
    )
    .unwrap();
    let spans = match_snippet(
        &body_parts("looked at {Alice} and smiled"),
        &visible,
        Profile::Plain,
    )
    .unwrap();

    assert_eq!(spans, vec![TextSpan { start: 28, end: 33 }]);
}

#[test]
fn capture_boundaries_follow_nfc_normalisation() {
    let visible = extract_visible_text("Café Alice", Profile::Plain).unwrap();
    let spans =
        match_snippet(&body_parts("Cafe\u{301} {Alice}"), &visible, Profile::Plain).unwrap();

    assert_eq!(spans, vec![TextSpan { start: 5, end: 10 }]);
}

#[test]
fn loose_expansions_do_not_create_duplicate_source_spans() {
    let visible = extract_visible_text("\u{fb00}", Profile::PlainLoose).unwrap();

    assert_eq!(
        match_snippet(&body_parts("f"), &visible, Profile::PlainLoose).unwrap(),
        vec![TextSpan { start: 0, end: 1 }]
    );
}

#[test]
fn markdown_profiles_resolve_against_rendered_text() {
    let exact =
        extract_visible_text("# Café\n\nAlice **opened** the file.\n", Profile::Markdown).unwrap();
    assert_eq!(
        match_snippet(&body_parts("Alice opened"), &exact, Profile::Markdown).unwrap(),
        vec![TextSpan { start: 5, end: 17 }]
    );

    let loose = extract_visible_text(
        "Alice\u{2014}opened\n\nthe \u{fb01}le.",
        Profile::MarkdownLoose,
    )
    .unwrap();
    assert_eq!(
        match_snippet(
            &body_parts("Alice-opened the file"),
            &loose,
            Profile::MarkdownLoose,
        )
        .unwrap(),
        vec![TextSpan { start: 0, end: 20 }]
    );
}

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

fn body_parts(body: &str) -> Vec<SnippetPart> {
    let parsed = parse(
        &format!("[{body}] a Character.\n"),
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
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
fn malformed_snippets_report_invalid_snippet() {
    let visible = extract_visible_text("A to B", Profile::Plain).unwrap();
    let exact = extract_visible_text("é", Profile::Plain).unwrap();
    let loose = extract_visible_text("A B", Profile::PlainLoose).unwrap();

    for (body, target, profile, message) in [
        (
            "{A}..B",
            &visible,
            Profile::Plain,
            "Captures are not allowed inside range snippets",
        ),
        (
            "A..B..C",
            &visible,
            Profile::Plain,
            "A range snippet may contain only one range separator",
        ),
        (
            "A {} B",
            &visible,
            Profile::Plain,
            "Capture may not be empty",
        ),
        (
            "A {b} {c}",
            &visible,
            Profile::Plain,
            "A snippet may contain at most one capture",
        ),
        (
            "A {to B",
            &visible,
            Profile::Plain,
            "Capture is not terminated",
        ),
        (
            "\"unterminated",
            &visible,
            Profile::Plain,
            "Quoted snippet text is not terminated",
        ),
        (
            "e{\u{301}}",
            &exact,
            Profile::Plain,
            "Capture boundaries collapse during text normalisation",
        ),
        (
            "A { } B",
            &loose,
            Profile::PlainLoose,
            "Capture boundaries collapse during text normalisation",
        ),
    ] {
        let diagnostic = match_snippet(&body_parts(body), target, profile).unwrap_err();
        assert_eq!(diagnostic.code, DiagnosticCode::InvalidSnippet, "{body:?}");
        assert_eq!(diagnostic.message, message, "{body:?}");
    }
}

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
fn triple_quoted_whole_body_decodes_delimiters() {
    // The whole-body quote rule applies to triple-quoted parts too: the
    // delimiters decode away and the inner text is the literal needle.
    let visible = extract_visible_text("before [sic] after", Profile::Plain).unwrap();

    assert_eq!(
        match_snippet(&body_parts("\"\"\"[sic]\"\"\""), &visible, Profile::Plain).unwrap(),
        vec![TextSpan { start: 7, end: 12 }]
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
        assert!(matches!(statement.subject, Value::ResolvedTextSpan { .. }));
        assert_eq!(statement.predicate, Value::Predicate("note".into()));
    }
}
