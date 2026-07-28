use serde_json::json;
use snipx_core::{export_json, ExportRequest, InputForm, Profile, Value};

#[test]
fn unresolved_snippets_remain_in_partial_facts() {
    let document = export_json(ExportRequest {
        source: "[Alice] a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Bob waited.".to_owned()),
        profile: Profile::Plain,
        path: Some("notes.snipx".to_owned()),
        target_uri: Some("chapter.txt".to_owned()),
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(
        value,
        json!({
            "snipxVersion": "0.0",
            "implementation": {
                "name": "snipx",
                "version": "0.0.0"
            },
            "input": {
                "form": "commentaria",
                "path": "notes.snipx"
            },
            "target": {
                "uri": "chapter.txt",
                "profile": "plain"
            },
            "visibleText": {
                "normalisation": "NFC",
                "length": 11
            },
            "facts": [{
                "subject": {
                    "kind": "unresolvedSnippet",
                    "source": "[Alice]"
                },
                "predicate": {
                    "kind": "predicate",
                    "value": "a"
                },
                "object": {
                    "kind": "name",
                    "value": "Character"
                },
                "source": {
                    "statement": {"start": 0, "end": 20},
                    "subject": {"start": 0, "end": 7},
                    "predicate": {"start": 8, "end": 9},
                    "object": {"start": 10, "end": 19}
                }
            }],
            "resolutions": [],
            "diagnostics": [{
                "code": "SNIPPET_NOT_FOUND",
                "severity": "error",
                "message": "Snippet did not match: [Alice]",
                "span": {"start": 0, "end": 7}
            }]
        })
    );
}

#[test]
fn resolved_export_includes_visible_text_facts_and_resolutions() {
    let document = export_json(ExportRequest {
        source: "[Alice] friend Bob.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Alice waited.".to_owned()),
        profile: Profile::Plain,
        path: None,
        target_uri: Some("chapter.txt".to_owned()),
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(
        value["visibleText"],
        json!({"normalisation": "NFC", "length": 13})
    );
    assert_eq!(
        value["resolutions"][0],
        json!({
            "source": "[Alice]",
            "sourceSpan": {"start": 0, "end": 7},
            "spans": [{"start": 0, "end": 5}]
        })
    );
    assert_eq!(
        value["facts"][0]["source"]["subject"],
        json!({"start": 0, "end": 7})
    );
    assert_eq!(
        value["facts"][0]["source"]["statement"],
        json!({"start": 0, "end": 19})
    );
    assert_eq!(
        value["facts"][0]["source"]["predicate"],
        json!({"start": 8, "end": 14})
    );
    assert_eq!(
        value["resolutions"][0]["sourceSpan"],
        json!({"start": 0, "end": 7})
    );
    assert_eq!(value["diagnostics"], json!([]));
}

#[test]
fn export_uses_ambient_subject_and_preserves_parser_diagnostics() {
    let document = export_json(ExportRequest {
        source: "/// a Character.\n".to_owned(),
        input_form: InputForm::Marginalia,
        target_text: None,
        profile: Profile::Plain,
        path: None,
        target_uri: None,
        ambient_subject: Some(Value::WholeDocument),
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(
        value["facts"][0]["subject"],
        json!({"kind": "wholeDocument"})
    );
    assert_eq!(value["diagnostics"], json!([]));
}

#[test]
fn targetless_export_still_reports_the_effective_profile() {
    let document = export_json(ExportRequest {
        source: "Alice a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: None,
        profile: Profile::PlainLoose,
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["target"], json!({"profile": "plain-loose"}));
}

#[test]
fn programmatic_non_finite_numbers_are_diagnostic_partial_values() {
    let document = export_json(ExportRequest {
        source: "/// a Character.\n".to_owned(),
        input_form: InputForm::Marginalia,
        target_text: None,
        profile: Profile::Plain,
        path: None,
        target_uri: None,
        ambient_subject: Some(Value::Number(f64::NAN)),
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(
        value["facts"][0]["subject"],
        json!({"kind": "unresolvedNumber", "source": "NaN"})
    );
    assert_eq!(value["diagnostics"][0]["code"], "INVALID_NUMBER");
}

#[test]
fn intralinea_uses_host_text_as_its_implicit_target() {
    let document = export_json(ExportRequest {
        source: "Alice waited. {{[Alice] a Character.}}".to_owned(),
        input_form: InputForm::Intralinea,
        target_text: None,
        profile: Profile::Plain,
        path: Some("chapter.txt".to_owned()),
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(
        value["visibleText"],
        json!({"normalisation": "NFC", "length": 14})
    );
    assert_eq!(
        value["resolutions"][0],
        json!({
            "source": "[Alice]",
            "sourceSpan": {"start": 16, "end": 23},
            "spans": [{"start": 0, "end": 5}]
        })
    );
    assert_eq!(value["diagnostics"], json!([]));
}

#[test]
fn overflowing_source_number_preserves_its_lexeme_and_span() {
    let literal = "9".repeat(400);
    let document = export_json(ExportRequest {
        source: format!("Alice score {literal}.\n"),
        input_form: InputForm::Commentaria,
        target_text: None,
        profile: Profile::Plain,
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(
        value["facts"][0]["object"],
        json!({"kind": "unresolvedNumber", "source": literal})
    );
    assert_eq!(value["diagnostics"][0]["code"], "INVALID_NUMBER");
    assert_eq!(
        value["diagnostics"][0]["span"],
        json!({"start": 12, "end": 412})
    );
}

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
    assert_eq!(
        value["resolutions"][0]["spans"][0],
        json!({"start": 0, "end": 5})
    );
    assert!(value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| {
            diagnostic["code"] == "RAW_HTML_OMITTED"
                && diagnostic["severity"] == "warning"
                && diagnostic["span"].is_object()
        }));
}
