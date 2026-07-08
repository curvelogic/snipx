use snipx_core::{parse, DiagnosticCode, InputForm, ParseOptions};

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
