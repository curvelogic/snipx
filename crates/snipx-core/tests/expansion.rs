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
