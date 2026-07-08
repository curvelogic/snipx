use snipx_core::{parse, DiagnosticCode, InputForm, ParseOptions, SyntaxKind};

#[test]
fn parses_basic_commentaria_without_errors() {
    let parsed = parse(
        "[Alice] a Character.\n",
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
      SNIPPET
        L_BRACK "["
        TEXT "Alice"
        R_BRACK "]"
    WHITESPACE " "
    PREDICATE
      IDENT
        TEXT "a"
    WHITESPACE " "
    OBJECT_LIST
      OBJECT
        IDENT
          TEXT "Character"
    DOT "."
  WHITESPACE "\n"
"###
    );
}

#[test]
fn parses_commentaria_language_surface() {
    let src = r#"@profile plain-loose
@target <novel.txt>

// binding
[looked at {Alice}]+ is Alice.
Alice `was born in` Oxford;
  friend Bob ::"childhood friend", Clara.
~["[Alice]"] italic true.
"#;

    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    insta::assert_snapshot!(parsed.debug_tree());
}

#[test]
fn parses_marginalia_embedded_regions() {
    let src = r#"Alice feels evasive.

```
[Alice] mood "guarded".
```

/// [door] motif Threshold.

```js
console.log("not snipx");
```
"#;

    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Marginalia,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    insta::assert_snapshot!(parsed.debug_tree());
}

#[test]
fn parses_intralinea_blocks_and_local_subjects() {
    let src = "Alice promised to return. {{< a Promise}} Bob waited. {{~<> highlight true. }}";

    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Intralinea,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    insta::assert_snapshot!(parsed.debug_tree());
}

#[test]
fn parses_ranges_and_all_snippet_quantifiers() {
    let src = "[The quick..jumped]* before [Alice]? and [Bob]+.\n";

    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == snipx_core::SyntaxKind::Quantifier)
            .count(),
        3
    );
    assert!(parsed
        .syntax()
        .descendants()
        .any(|node| node.kind() == snipx_core::SyntaxKind::RangeSnippet));
}

#[test]
fn marginalia_slash_marker_is_lossless_without_space() {
    let src = "///[Alice] a Character.\n";

    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Marginalia,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
}

#[test]
fn malformed_snippet_recovers() {
    let parsed = parse(
        "[Alice a Character.",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert_eq!(
        parsed.diagnostics()[0].code,
        DiagnosticCode::UnterminatedSnippet
    );
}

#[test]
fn malformed_intralinea_block_recovers() {
    let parsed = parse(
        "Before {{< a Promise.",
        ParseOptions {
            input_form: InputForm::Intralinea,
        },
    );

    assert_eq!(
        parsed.diagnostics()[0].code,
        DiagnosticCode::UnterminatedIntralineaBlock
    );
}

#[test]
fn embedded_statements_preserve_sibling_and_nesting_structure() {
    for (src, input_form, container_kind) in [
        (
            "```\nAlice friend Bob.\n```\n",
            InputForm::Marginalia,
            SyntaxKind::Fence,
        ),
        (
            "{{Alice friend Bob.}}",
            InputForm::Intralinea,
            SyntaxKind::IntralineaBlock,
        ),
    ] {
        let parsed = parse(src, ParseOptions { input_form });
        let container = parsed
            .syntax()
            .descendants()
            .find(|node| node.kind() == container_kind)
            .expect("embedded container");
        let statement = container
            .children()
            .find(|node| node.kind() == SyntaxKind::Statement)
            .expect("statement is a direct child of its container");
        let child_kinds: Vec<_> = statement.children().map(|node| node.kind()).collect();

        assert_eq!(
            child_kinds,
            [
                SyntaxKind::Subject,
                SyntaxKind::Predicate,
                SyntaxKind::ObjectList
            ]
        );
        assert_eq!(parsed.syntax().to_string(), src);
    }
}

#[test]
fn semicolon_carry_forward_across_newline_is_one_statement_chain() {
    let src = "Alice a Character;\n  friend Bob.\n";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::Statement)
            .count(),
        1
    );
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::Subject)
            .count(),
        1
    );
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::Predicate)
            .count(),
        2
    );
}

#[test]
fn intralinea_closing_ignores_strings_and_capture_closes() {
    for src in [r#"{{ note "a }} b". }}"#, "{{{Alice}}}"] {
        let parsed = parse(
            src,
            ParseOptions {
                input_form: InputForm::Intralinea,
            },
        );

        assert!(parsed.diagnostics().is_empty(), "{src:?}");
        assert_eq!(parsed.syntax().to_string(), src);
        assert_eq!(
            parsed
                .syntax()
                .descendants()
                .filter(|node| node.kind() == SyntaxKind::IntralineaBlock)
                .count(),
            1
        );
    }
}

#[test]
fn recognises_all_local_subject_markers() {
    let cases = [
        "{{< p O. }}",
        "{{p O >}}",
        "{{<> p O. }}",
        "{{<< p O. }}",
        "{{p O >>}}",
        "{{<<>> p O. }}",
        "{{~< p O. }}",
        "{{p O ~>}}",
        "{{~<> p O. }}",
        "{{~<< p O. }}",
        "{{p O ~>>}}",
        "{{~<<>> p O. }}",
    ];

    for src in cases {
        let parsed = parse(
            src,
            ParseOptions {
                input_form: InputForm::Intralinea,
            },
        );

        assert!(parsed.diagnostics().is_empty(), "{src:?}");
        assert_eq!(parsed.syntax().to_string(), src);
        assert_eq!(
            parsed
                .syntax()
                .descendants()
                .filter(|node| node.kind() == SyntaxKind::LocalSubjectMarker)
                .count(),
            1,
            "{src:?}"
        );
    }
}

#[test]
fn diagnoses_malformed_local_subject_markers() {
    for src in ["{{<<< p O. }}", "{{<>> p O. }}", "{{~>>> p O. }}"] {
        let parsed = parse(
            src,
            ParseOptions {
                input_form: InputForm::Intralinea,
            },
        );

        assert_eq!(parsed.syntax().to_string(), src);
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::InvalidLocalSubjectMarker),
            "{src:?}"
        );
    }
}

#[test]
fn marginalia_fences_preserve_spacing_suffixes_and_crlf() {
    let src = concat!(
        "``` snipx \t\r\n",
        "Alice friend Bob.  \r\n",
        "``` trailing \t\r\n",
        "``` js \r\n",
        "const value = 1;\r\n",
        "``` ignored\r\n",
    );
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Marginalia,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::Statement)
            .count(),
        1
    );
}

#[test]
fn indented_marginalia_slash_marker_is_lossless() {
    let src = " \t///   Alice friend Bob.\r\n";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Marginalia,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert!(parsed
        .syntax()
        .descendants()
        .any(|node| node.kind() == SyntaxKind::Statement));
}

#[test]
fn ordinary_strings_honour_escaped_quotes_and_backslashes() {
    let src = r#"Alice note "a \"quote\" and \\ path"."#;
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let string = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::String)
        .expect("string");

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(string.to_string(), r#""a \"quote\" and \\ path""#);
}

#[test]
fn numbers_do_not_consume_statement_terminators() {
    let src = "Answer value 42.\nRatio value 1.5.\n";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let numbers: Vec<_> = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::Number)
        .map(|node| node.to_string())
        .collect();

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(numbers, ["42", "1.5"]);
    assert_eq!(
        parsed
            .syntax()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.kind() == SyntaxKind::Dot)
            .count(),
        2
    );

    let malformed = parse(
        "Value is 1.2.3.",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    assert_eq!(malformed.syntax().to_string(), "Value is 1.2.3.");
    assert_eq!(
        malformed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::Number)
            .map(|node| node.to_string())
            .collect::<Vec<_>>(),
        ["1.2", "3"]
    );
}

#[test]
fn snippet_ranges_honour_escaped_quotes_and_backslashes() {
    for (src, expected_kind) in [
        (r#"["quoted \"..\" text"] p O."#, SyntaxKind::Snippet),
        (r#"["path \\"..outside] p O."#, SyntaxKind::RangeSnippet),
    ] {
        let parsed = parse(
            src,
            ParseOptions {
                input_form: InputForm::Commentaria,
            },
        );
        let snippet = parsed
            .syntax()
            .descendants()
            .find(|node| matches!(node.kind(), SyntaxKind::Snippet | SyntaxKind::RangeSnippet))
            .expect("snippet");

        assert!(parsed.diagnostics().is_empty(), "{src:?}");
        assert_eq!(parsed.syntax().to_string(), src);
        assert_eq!(snippet.kind(), expected_kind, "{src:?}");
    }
}

#[test]
fn statement_terminator_policy_depends_on_input_form() {
    let commentaria = parse(
        "Alice friend Bob",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    assert_eq!(commentaria.syntax().to_string(), "Alice friend Bob");
    assert!(commentaria.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::ParseError
            && diagnostic.message.contains("statement terminator")
    }));

    for (src, input_form) in [
        ("{{Alice friend Bob}}", InputForm::Intralinea),
        ("/// Alice friend Bob\n", InputForm::Marginalia),
    ] {
        let parsed = parse(src, ParseOptions { input_form });

        assert!(parsed.diagnostics().is_empty(), "{src:?}");
        assert_eq!(parsed.syntax().to_string(), src);
    }
}

#[test]
fn directives_after_statements_are_diagnosed_in_each_region() {
    for (src, input_form) in [
        (
            "Alice friend Bob.\n@profile loose\n",
            InputForm::Commentaria,
        ),
        (
            "```\nAlice friend Bob.\n@profile loose\n```\n",
            InputForm::Marginalia,
        ),
        (
            "{{Alice friend Bob.\n@profile loose\n}}",
            InputForm::Intralinea,
        ),
    ] {
        let parsed = parse(src, ParseOptions { input_form });

        assert_eq!(parsed.syntax().to_string(), src);
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::InvalidDirectivePosition),
            "{src:?}"
        );
    }
}

#[test]
fn local_subject_markers_are_rejected_outside_intralinea() {
    for (src, input_form) in [
        ("< p O.", InputForm::Commentaria),
        ("/// ~<> p O.\n", InputForm::Marginalia),
    ] {
        let parsed = parse(src, ParseOptions { input_form });

        assert_eq!(parsed.syntax().to_string(), src);
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::InvalidLocalSubjectMarker),
            "{src:?}"
        );
        assert!(
            !parsed
                .syntax()
                .descendants()
                .any(|node| node.kind() == SyntaxKind::LocalSubjectMarker),
            "{src:?}"
        );
    }
}
