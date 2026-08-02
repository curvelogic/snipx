use snipx_core::{expand, parse, DiagnosticCode, ExpandOptions, InputForm, ParseOptions, Value};

fn parse_commentaria(source: &str) -> snipx_core::Parse {
    parse(
        source,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    )
}

#[test]
fn expands_semicolon_and_comma_carry_forward() {
    let parsed = parse_commentaria("Alice a Character; hair \"red\", \"brown\".\n");
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(expanded.diagnostics.is_empty());
    assert_eq!(expanded.statements.len(), 3);
    assert_eq!(expanded.statements[0].subject, Value::Name("Alice".into()));
    assert_eq!(
        expanded.statements[0].predicate,
        Value::Predicate("a".into())
    );
    assert_eq!(
        expanded.statements[0].object,
        Value::Name("Character".into())
    );
    assert_eq!(
        expanded.statements[1].predicate,
        Value::Predicate("hair".into())
    );
    assert_eq!(expanded.statements[1].object, Value::String("red".into()));
    assert_eq!(expanded.statements[2].object, Value::String("brown".into()));
}

#[test]
fn fills_ambient_subject_for_subjectless_statement_chain() {
    let parsed = parse(
        "/// a Character; hair \"red\".\n",
        ParseOptions {
            input_form: InputForm::Marginalia,
        },
    );
    let expanded = expand(
        &parsed,
        ExpandOptions {
            ambient_subject: Some(Value::WholeDocument),
        },
    );

    assert!(expanded.diagnostics.is_empty());
    assert_eq!(expanded.statements.len(), 2);
    assert!(expanded
        .statements
        .iter()
        .all(|statement| statement.subject == Value::WholeDocument));
}

#[test]
fn diagnoses_subjectless_statement_without_ambient_subject() {
    let parsed = parse(
        "/// hair \"red\".\n",
        ParseOptions {
            input_form: InputForm::Marginalia,
        },
    );
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(expanded.statements.is_empty());
    assert!(expanded
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::MissingAmbientSubject));
}

#[test]
fn decodes_standard_escapes_in_ordinary_strings() {
    let parsed =
        parse_commentaria("Alice note \"Line one\\nTab\\there \\\"quoted\\\" back\\\\slash\".\n");
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(expanded.diagnostics.is_empty());
    assert_eq!(
        expanded.statements[0].object,
        Value::String("Line one\nTab\there \"quoted\" back\\slash".into())
    );
}

#[test]
fn preserves_unknown_escape_sequences_verbatim() {
    let parsed = parse_commentaria("Alice note \"odd \\q escape\".\n");
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(expanded.diagnostics.is_empty());
    assert_eq!(
        expanded.statements[0].object,
        Value::String("odd \\q escape".into())
    );
}

#[test]
fn dedents_common_indentation_in_triple_strings() {
    let parsed = parse_commentaria(
        "[Alice] note \"\"\"\n  This is a longer note.\n\n  It can contain paragraphs.\n\"\"\".\n",
    );
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(expanded.diagnostics.is_empty());
    assert_eq!(
        expanded.statements[0].object,
        Value::String("This is a longer note.\n\nIt can contain paragraphs.\n".into())
    );
}

#[test]
fn triple_string_dedent_uses_minimum_indent_and_ignores_blank_lines() {
    let parsed =
        parse_commentaria("[Alice] note \"\"\"\n    deep\n  shallow\n\n      deeper\n  \"\"\".\n");
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(expanded.diagnostics.is_empty());
    assert_eq!(
        expanded.statements[0].object,
        Value::String("  deep\nshallow\n\n    deeper\n".into())
    );
}

#[test]
fn single_line_triple_string_is_not_dedented() {
    let parsed = parse_commentaria("[Alice] note \"\"\"inline text\"\"\".\n");
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(expanded.diagnostics.is_empty());
    assert_eq!(
        expanded.statements[0].object,
        Value::String("inline text".into())
    );
}

#[test]
fn triple_strings_do_not_decode_escapes() {
    let parsed = parse_commentaria("[Alice] note \"\"\"raw \\n stays\"\"\".\n");
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(expanded.diagnostics.is_empty());
    assert_eq!(
        expanded.statements[0].object,
        Value::String("raw \\n stays".into())
    );
}

#[test]
fn negative_number_literals_are_supported() {
    let parsed = parse_commentaria("Alice score -5; delta -0.25.\n");
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(expanded.diagnostics.is_empty());
    assert_eq!(expanded.statements[0].object, Value::Number(-5.0));
    assert_eq!(expanded.statements[1].object, Value::Number(-0.25));
}

#[test]
fn bare_minus_without_digits_is_still_an_error() {
    let parsed = parse_commentaria("Alice score -x.\n");
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(!expanded.diagnostics.is_empty());
}

#[test]
fn line_comment_directly_after_identifier_does_not_join_it() {
    let parsed = parse_commentaria("Alice friend Bob// trailing comment\n");
    let expanded = expand(&parsed, ExpandOptions::default());

    assert_eq!(expanded.statements.len(), 1);
    assert_eq!(expanded.statements[0].object, Value::Name("Bob".into()));
    // The only diagnostic is the missing statement terminator.
    assert_eq!(expanded.diagnostics.len(), 1);
    assert_eq!(expanded.diagnostics[0].code, DiagnosticCode::ParseError);
}

#[test]
fn expands_subject_and_object_decorations_to_note_statements() {
    let parsed = parse_commentaria(
        "[Alice] ::\"protagonist\".\nAlice friend Bob ::\"childhood friend\", Clara ::\"rival\".\n",
    );
    let expanded = expand(&parsed, ExpandOptions::default());

    assert!(expanded.diagnostics.is_empty());
    assert_eq!(expanded.statements.len(), 5);
    assert_eq!(
        expanded.statements[0].predicate,
        Value::Predicate("note".into())
    );
    assert_eq!(
        expanded.statements[0].object,
        Value::String("protagonist".into())
    );
    assert_eq!(expanded.statements[2].subject, Value::Name("Bob".into()));
    assert_eq!(
        expanded.statements[2].predicate,
        Value::Predicate("note".into())
    );
    assert_eq!(
        expanded.statements[2].object,
        Value::String("childhood friend".into())
    );
    assert_eq!(expanded.statements[4].subject, Value::Name("Clara".into()));
    assert_eq!(
        expanded.statements[4].predicate,
        Value::Predicate("note".into())
    );
    assert_eq!(expanded.statements[4].object, Value::String("rival".into()));
}
