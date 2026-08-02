use snipx_core::{
    parse, Cardinality, InputForm, ParseOptions, SnippetPart, SnippetValue, SyntaxKind,
};

fn snippet_value(snippet: &str) -> SnippetValue {
    let parsed = parse(
        &format!("{snippet} a Character.\n"),
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
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
            SnippetPart::Capture {
                text: "Alice".into(),
                terminated: true
            },
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
    assert!(value.parts.iter().any(|part| matches!(
        part,
        SnippetPart::Capture {
            terminated: false,
            ..
        }
    )));
}

#[test]
fn empty_snippet_has_no_parts() {
    let value = snippet_value("[]");
    assert!(value.parts.is_empty());
    assert!(value.terminated);
}
