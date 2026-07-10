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
fn directives_preserve_trailing_horizontal_whitespace_before_line_endings() {
    let src = "@target <doc.txt>  \n@profile loose\t\r\n[Alice] a Character.\n";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let directives: Vec<_> = parsed
        .syntax()
        .children()
        .filter(|node| {
            matches!(
                node.kind(),
                SyntaxKind::TargetDirective | SyntaxKind::ProfileDirective
            )
        })
        .map(|node| node.to_string())
        .collect();
    let whitespace: Vec<_> = parsed
        .syntax()
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Whitespace)
        .map(|token| token.text().to_string())
        .collect();

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(directives, ["@target <doc.txt>  ", "@profile loose\t"]);
    assert_eq!(whitespace, ["\n", "\r\n", "\n"]);
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
fn inline_comments_are_statement_trivia_and_preserve_semicolon_chains() {
    let src = "Alice /* binding */ friend Bob.\nAlice a Character; // carry-forward comment\n  friend Bob.\n";
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
        2
    );
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::Predicate)
            .count(),
        3
    );
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| {
                matches!(
                    node.kind(),
                    SyntaxKind::LineComment | SyntaxKind::BlockComment
                )
            })
            .count(),
        2
    );
}

#[test]
fn unterminated_inline_block_comments_preserve_source_and_diagnose() {
    let src = "Alice /* binding";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert_eq!(parsed.syntax().to_string(), src);
    assert!(parsed
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedBlockComment));
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

        assert!(
            !parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedIntralineaBlock),
            "{src:?}"
        );
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
fn unterminated_single_line_strings_stop_at_newlines_and_resume() {
    let src = "Alice note \"unterminated\nBob friend Dana.\n";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert_eq!(parsed.syntax().to_string(), src);
    assert!(parsed
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedString));
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::Statement)
            .count(),
        2
    );
}

#[test]
fn triple_strings_remain_multiline() {
    let src = "Alice note \"\"\"line one\nline two\"\"\".\n";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let triple_string = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::TripleString)
        .expect("triple string");

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(triple_string.to_string(), "\"\"\"line one\nline two\"\"\"");
}

#[test]
fn intralinea_closing_recovers_after_unterminated_single_line_strings() {
    for src in [
        "Before {{Alice note \"unterminated\nBob friend Dana.\n}} After",
        "Before {{Alice note \"unterminated\r\nBob friend Dana.\r\n}} After",
    ] {
        let parsed = parse(
            src,
            ParseOptions {
                input_form: InputForm::Intralinea,
            },
        );
        let trailing_text = parsed
            .syntax()
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.kind() == SyntaxKind::IntralineaText)
            .last()
            .expect("trailing host text");

        assert_eq!(parsed.syntax().to_string(), src);
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedString),
            "{src:?}"
        );
        assert!(
            !parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedIntralineaBlock),
            "{src:?}"
        );
        assert_eq!(
            parsed
                .syntax()
                .descendants()
                .filter(|node| node.kind() == SyntaxKind::Statement)
                .count(),
            2,
            "{src:?}"
        );
        assert_eq!(trailing_text.text(), " After", "{src:?}");
    }
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
fn malformed_snippet_capture_forms_recover_with_errors() {
    for src in ["[Alice {one} {two}] p O.", "[Alice {bad}..Bob] p O."] {
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

        assert_eq!(parsed.syntax().to_string(), src, "{src:?}");
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ParseError),
            "{src:?}"
        );
        assert!(
            snippet
                .descendants()
                .any(|node| node.kind() == SyntaxKind::Error),
            "{src:?}"
        );
    }

    let valid = "[looked at {Alice}]+ is Alice.";
    let parsed = parse(
        valid,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), valid);
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
fn misplaced_exact_directives_keep_specific_cst_kinds() {
    let src = concat!(
        "Alice friend Bob.\n",
        "@target <doc.txt>\n",
        "@profile loose\n",
        "@targeted value\n",
        "@profiled value\n",
    );
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let child_kinds: Vec<_> = parsed.syntax().children().map(|node| node.kind()).collect();

    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(
        child_kinds,
        [
            SyntaxKind::Statement,
            SyntaxKind::TargetDirective,
            SyntaxKind::ProfileDirective,
            SyntaxKind::Directive,
            SyntaxKind::Directive,
        ]
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::InvalidDirectivePosition)
            .count(),
        4
    );
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

#[test]
fn text_span_sigils_and_decorations_have_distinct_cst_roles() {
    let text_span = parse(
        "~[Alice] p O.",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let statement = text_span
        .syntax()
        .children()
        .find(|node| node.kind() == SyntaxKind::Statement)
        .expect("statement");
    let subject = statement
        .children()
        .find(|node| node.kind() == SyntaxKind::Subject)
        .expect("subject");

    assert!(text_span.diagnostics().is_empty());
    assert_eq!(text_span.syntax().to_string(), "~[Alice] p O.");
    assert!(subject
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::Tilde));
    assert!(subject
        .descendants()
        .any(|node| node.kind() == SyntaxKind::Snippet));
    assert!(!text_span
        .syntax()
        .descendants()
        .any(|node| node.kind() == SyntaxKind::Decoration));

    for (src, expected_children) in [
        (
            r#"[Alice] ::"note"."#,
            vec![SyntaxKind::Subject, SyntaxKind::Decoration],
        ),
        (r#"{{::"note".}}"#, vec![SyntaxKind::Decoration]),
        ("/// ::\"note\".\n", vec![SyntaxKind::Decoration]),
    ] {
        let input_form = if src.starts_with("{{") {
            InputForm::Intralinea
        } else if src.starts_with("///") {
            InputForm::Marginalia
        } else {
            InputForm::Commentaria
        };
        let parsed = parse(src, ParseOptions { input_form });
        let statement = parsed
            .syntax()
            .descendants()
            .find(|node| node.kind() == SyntaxKind::Statement)
            .expect("statement");
        let decoration = parsed
            .syntax()
            .descendants()
            .find(|node| node.kind() == SyntaxKind::Decoration)
            .expect("decoration");

        assert!(parsed.diagnostics().is_empty(), "{src:?}");
        assert_eq!(parsed.syntax().to_string(), src);
        assert!(decoration
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .any(|token| token.kind() == SyntaxKind::ColonColon));
        assert!(decoration
            .descendants()
            .any(|node| node.kind() == SyntaxKind::String));
        assert_eq!(
            statement
                .children()
                .map(|node| node.kind())
                .collect::<Vec<_>>(),
            expected_children,
            "{src:?}"
        );
    }
}

#[test]
fn malformed_decorations_recover_losslessly() {
    let src = "{{::note.}}";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Intralinea,
        },
    );

    assert_eq!(parsed.syntax().to_string(), src);
    assert!(parsed
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::ParseError));
}

#[test]
fn incomplete_statements_emit_parse_errors_and_error_nodes() {
    for (src, input_form) in [
        ("Alice.", InputForm::Commentaria),
        (".", InputForm::Commentaria),
        ("{{hair.}}", InputForm::Intralinea),
    ] {
        let parsed = parse(src, ParseOptions { input_form });

        assert_eq!(parsed.syntax().to_string(), src);
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ParseError),
            "{src:?}"
        );
        assert!(
            parsed
                .syntax()
                .descendants()
                .any(|node| node.kind() == SyntaxKind::Error),
            "{src:?}"
        );
    }
}

#[test]
fn object_decorations_attach_after_their_objects() {
    let src = r#"Alice friend Bob ::"childhood", Clara ::"rival"."#;
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let object_list = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::ObjectList)
        .expect("object list");
    let child_kinds: Vec<_> = object_list.children().map(|node| node.kind()).collect();

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(
        child_kinds,
        [
            SyntaxKind::Object,
            SyntaxKind::Decoration,
            SyntaxKind::Object,
            SyntaxKind::Decoration,
        ]
    );
}

#[test]
fn only_the_final_embedded_statement_may_omit_its_terminator() {
    for (invalid, valid, input_form) in [
        (
            "```\nAlice friend Bob\nCarol friend Dana\n```\n",
            "```\nAlice friend Bob.\nCarol friend Dana\n```\n",
            InputForm::Marginalia,
        ),
        (
            "{{Alice friend Bob\nCarol friend Dana}}",
            "{{Alice friend Bob.\nCarol friend Dana}}",
            InputForm::Intralinea,
        ),
    ] {
        let invalid_parse = parse(invalid, ParseOptions { input_form });
        let valid_parse = parse(valid, ParseOptions { input_form });

        assert_eq!(invalid_parse.syntax().to_string(), invalid);
        assert_eq!(
            invalid_parse
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagnosticCode::ParseError)
                .count(),
            1,
            "{invalid:?}"
        );
        assert!(valid_parse.diagnostics().is_empty(), "{valid:?}");
        assert_eq!(valid_parse.syntax().to_string(), valid);
    }
}

#[test]
fn intralinea_closing_ignores_line_and_block_comments() {
    for src in [
        "Before {{Alice friend Bob. // }} ignored\nCarol friend Dana.}} After",
        "Before {{Alice friend Bob. /* }} ignored */ Carol friend Dana.}} After",
    ] {
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
                .filter(|node| node.kind() == SyntaxKind::Statement)
                .count(),
            2,
            "{src:?}"
        );
    }
}

#[test]
fn intralinea_closing_ignores_uri_and_snippet_text_comment_markers() {
    for src in [
        "Before {{Alice source <https://example.test>.}} After",
        "Before {{[https://example.test] source Example.}} After",
    ] {
        let parsed = parse(
            src,
            ParseOptions {
                input_form: InputForm::Intralinea,
            },
        );
        let trailing_text = parsed
            .syntax()
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.kind() == SyntaxKind::IntralineaText)
            .last()
            .expect("trailing host text");

        assert!(parsed.diagnostics().is_empty(), "{src:?}");
        assert_eq!(parsed.syntax().to_string(), src);
        assert_eq!(trailing_text.text(), " After", "{src:?}");
    }
}

#[test]
fn unterminated_uri_and_snippet_still_allow_intralinea_close() {
    for (src, diagnostic_code, opener) in [
        (
            "Before {{<https://example.test}} After",
            DiagnosticCode::ParseError,
            "<",
        ),
        (
            "Before {{[https://example.test}} After",
            DiagnosticCode::UnterminatedSnippet,
            "[",
        ),
    ] {
        let parsed = parse(
            src,
            ParseOptions {
                input_form: InputForm::Intralinea,
            },
        );
        let diagnostic = parsed
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.code == diagnostic_code)
            .expect("expected unterminated diagnostic");
        let trailing_text = parsed
            .syntax()
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.kind() == SyntaxKind::IntralineaText)
            .last()
            .expect("trailing host text");
        let start = src.find(opener).expect("unterminated opener");
        let end = src.find("}}").expect("intralinea close");

        assert_eq!(parsed.syntax().to_string(), src);
        assert!(
            !parsed
                .diagnostics()
                .iter()
                .any(|item| item.code == DiagnosticCode::UnterminatedIntralineaBlock),
            "{src:?}"
        );
        assert_eq!(trailing_text.text(), " After", "{src:?}");
        assert_eq!(diagnostic.span.as_ref().map(|span| span.start), Some(start));
        assert_eq!(diagnostic.span.as_ref().map(|span| span.end), Some(end));
    }
}

#[test]
fn unterminated_quoted_snippet_still_allows_intralinea_close() {
    let src = "Before {{[\"unterminated\n}} After";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Intralinea,
        },
    );
    let trailing_text = parsed
        .syntax()
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::IntralineaText)
        .last()
        .expect("trailing host text");

    assert_eq!(parsed.syntax().to_string(), src);
    assert!(parsed
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedString));
    assert!(parsed
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedSnippet));
    assert!(!parsed
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedIntralineaBlock));
    assert_eq!(trailing_text.text(), " After");
}

#[test]
fn malformed_capture_still_allows_intralinea_close() {
    let src = "Before {{Alice rel {Bob }} After";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Intralinea,
        },
    );
    let trailing_text = parsed
        .syntax()
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::IntralineaText)
        .last()
        .expect("trailing host text");
    let capture_diagnostic = parsed
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message == "Unterminated capture")
        .expect("unterminated capture diagnostic");
    let close_start = src.find("}}").expect("intralinea close");

    assert_eq!(parsed.syntax().to_string(), src);
    assert!(!parsed
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedIntralineaBlock));
    assert_eq!(trailing_text.text(), " After");
    assert_eq!(
        capture_diagnostic.span.as_ref().map(|span| span.end),
        Some(close_start)
    );
}

#[test]
fn unterminated_snippet_stops_at_newline_and_recovers_statement() {
    let src = "[Alice a Character.\nBob friend Carol.\n";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let statements: Vec<_> = parsed
        .syntax()
        .children()
        .filter(|node| node.kind() == SyntaxKind::Statement)
        .collect();
    let snippet_diagnostic = parsed
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedSnippet)
        .expect("unterminated snippet diagnostic");
    let second_statement = statements.get(1).expect("second statement");
    let child_kinds: Vec<_> = second_statement
        .children()
        .map(|node| node.kind())
        .collect();

    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(statements.len(), 2);
    assert_eq!(second_statement.to_string(), "Bob friend Carol.");
    assert_eq!(
        child_kinds,
        vec![
            SyntaxKind::Subject,
            SyntaxKind::Predicate,
            SyntaxKind::ObjectList
        ]
    );
    assert_eq!(
        snippet_diagnostic.span.as_ref().map(|span| span.end),
        src.find('\n')
    );
}

#[test]
fn unterminated_capture_stops_at_newline_and_recovers_statement() {
    let src = "Alice friend {Bob\nCarol friend Dana.\n";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let statements: Vec<_> = parsed
        .syntax()
        .children()
        .filter(|node| node.kind() == SyntaxKind::Statement)
        .collect();
    let capture_diagnostic = parsed
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message == "Unterminated capture")
        .expect("unterminated capture diagnostic");
    let second_statement = statements.get(1).expect("second statement");
    let child_kinds: Vec<_> = second_statement
        .children()
        .map(|node| node.kind())
        .collect();

    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(statements.len(), 2);
    assert_eq!(second_statement.to_string(), "Carol friend Dana.");
    assert_eq!(
        child_kinds,
        vec![
            SyntaxKind::Subject,
            SyntaxKind::Predicate,
            SyntaxKind::ObjectList
        ]
    );
    assert_eq!(
        capture_diagnostic.span.as_ref().map(|span| span.end),
        src.find('\n')
    );
}

#[test]
fn ambient_statements_have_predicate_and_object_boundaries_without_subjects() {
    for (src, input_form) in [
        ("/// hair \"red\".\n", InputForm::Marginalia),
        ("```\n`is afraid of` TheDark.\n```\n", InputForm::Marginalia),
        ("{{hair \"red\".}}", InputForm::Intralinea),
        ("{{`is afraid of` TheDark.}}", InputForm::Intralinea),
    ] {
        let parsed = parse(src, ParseOptions { input_form });
        let statement = parsed
            .syntax()
            .descendants()
            .find(|node| node.kind() == SyntaxKind::Statement)
            .expect("statement");
        let child_kinds: Vec<_> = statement.children().map(|node| node.kind()).collect();

        assert!(parsed.diagnostics().is_empty(), "{src:?}");
        assert_eq!(parsed.syntax().to_string(), src);
        assert_eq!(
            child_kinds,
            [SyntaxKind::Predicate, SyntaxKind::ObjectList],
            "{src:?}"
        );
    }

    let commentaria = parse(
        "hair \"red\".",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let statement = commentaria
        .syntax()
        .children()
        .find(|node| node.kind() == SyntaxKind::Statement)
        .expect("statement");

    assert!(statement
        .children()
        .any(|node| node.kind() == SyntaxKind::Subject));
}

#[test]
fn directive_names_are_classified_by_exact_identifier() {
    let src = "@targeted value\n@profiled value\nAlice friend Bob.\n";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let child_kinds: Vec<_> = parsed.syntax().children().map(|node| node.kind()).collect();

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert_eq!(
        child_kinds,
        [
            SyntaxKind::Directive,
            SyntaxKind::Directive,
            SyntaxKind::Statement,
        ]
    );
}

#[test]
fn standalone_quantifiers_and_punctuation_recover_as_errors() {
    for src in ["+ + +.", ") ) )."] {
        let parsed = parse(
            src,
            ParseOptions {
                input_form: InputForm::Commentaria,
            },
        );

        assert_eq!(parsed.syntax().to_string(), src);
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ParseError),
            "{src:?}"
        );
        assert!(
            parsed
                .syntax()
                .descendants()
                .any(|node| node.kind() == SyntaxKind::Error),
            "{src:?}"
        );
    }
}

#[test]
fn predicate_synonym_with_punctuation_remains_valid() {
    let src = "Alice = <https://example.org/alice>.\n";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.syntax().to_string(), src);
    assert!(parsed
        .syntax()
        .descendants()
        .any(|node| node.kind() == SyntaxKind::Predicate));
    assert!(!parsed
        .syntax()
        .descendants()
        .any(|node| node.kind() == SyntaxKind::Error));
}

#[test]
fn directive_inline_values_do_not_duplicate_following_source() {
    for src in [
        "@custom [doc\nAlice friend Bob.\n",
        "@custom {doc\nAlice friend Bob.\n",
        "@custom `doc\nAlice friend Bob.\n",
        "@custom \"\"\"doc\nAlice friend Bob.\n",
    ] {
        let parsed = parse(
            src,
            ParseOptions {
                input_form: InputForm::Commentaria,
            },
        );
        let rendered = parsed.syntax().to_string();

        assert_eq!(rendered, src, "{src:?}");
        assert_eq!(
            rendered.match_indices("Alice friend Bob.\n").count(),
            1,
            "{src:?}"
        );
        assert!(!parsed.diagnostics().is_empty(), "{src:?}");
    }
}

#[test]
fn non_identifier_predicates_recover_as_errors() {
    for src in [
        r#"Alice "not a predicate" Bob."#,
        "Alice [Bob] Carol.",
        "Alice Bob Carol.",
    ] {
        let parsed = parse(
            src,
            ParseOptions {
                input_form: InputForm::Commentaria,
            },
        );
        let statement = parsed
            .syntax()
            .children()
            .find(|node| node.kind() == SyntaxKind::Statement)
            .expect("statement");
        let predicate = statement
            .children()
            .find(|node| node.kind() == SyntaxKind::Predicate)
            .expect("predicate");
        let object_list = statement
            .children()
            .find(|node| node.kind() == SyntaxKind::ObjectList)
            .expect("object list");

        assert_eq!(parsed.syntax().to_string(), src, "{src:?}");
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ParseError),
            "{src:?}"
        );
        assert!(
            predicate
                .descendants()
                .any(|node| node.kind() == SyntaxKind::Error),
            "{src:?}"
        );
        assert!(
            object_list
                .descendants()
                .any(|node| node.kind() == SyntaxKind::Identifier),
            "{src:?}"
        );
    }
}

#[test]
fn boolean_literals_do_not_parse_as_predicates() {
    let invalid = parse(
        "Alice true Bob.",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let statement = invalid
        .syntax()
        .children()
        .find(|node| node.kind() == SyntaxKind::Statement)
        .expect("statement");
    let predicate = statement
        .children()
        .find(|node| node.kind() == SyntaxKind::Predicate)
        .expect("predicate");
    let object_list = statement
        .children()
        .find(|node| node.kind() == SyntaxKind::ObjectList)
        .expect("object list");

    assert_eq!(invalid.syntax().to_string(), "Alice true Bob.");
    assert!(invalid
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::ParseError));
    assert!(predicate
        .descendants()
        .any(|node| node.kind() == SyntaxKind::Error));
    assert!(object_list
        .descendants()
        .any(|node| node.kind() == SyntaxKind::Identifier));

    let valid = parse(
        "Alice note true.",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let boolean = valid
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::Boolean)
        .expect("boolean object");

    assert!(valid.diagnostics().is_empty());
    assert_eq!(valid.syntax().to_string(), "Alice note true.");
    assert_eq!(boolean.to_string(), "true");
}

#[test]
fn unterminated_backtick_predicate_still_allows_intralinea_close() {
    let src = "Before {{Alice `unterminated\nBob friend Dana.}} After";
    let parsed = parse(
        src,
        ParseOptions {
            input_form: InputForm::Intralinea,
        },
    );
    let trailing_text = parsed
        .syntax()
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::IntralineaText)
        .last()
        .expect("trailing host text");

    assert_eq!(parsed.syntax().to_string(), src);
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::ParseError && diagnostic.message.contains("backtick")
    }));
    assert!(!parsed
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::UnterminatedIntralineaBlock));
    assert_eq!(trailing_text.text(), " After");
}
