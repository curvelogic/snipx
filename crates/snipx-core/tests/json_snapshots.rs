use serde_json::json;
use snipx_core::{export_json, ExportRequest, InputForm, Profile, Value};

#[test]
fn unresolved_snippets_remain_in_partial_facts() {
    let document = export_json(ExportRequest {
        lint: false,
        source: "[Alice] a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Bob waited.".to_owned()),
        profile: Some(Profile::Plain),
        path: Some("notes.snipx".to_owned()),
        target_uri: Some("chapter.txt".to_owned()),
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(
        value,
        json!({
            "snipxVersion": "0.1",
            "implementation": {
                "name": "snipx",
                "version": "0.1.1"
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
        lint: false,
        source: "[Alice] friend Bob.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Alice waited.".to_owned()),
        profile: Some(Profile::Plain),
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
        lint: false,
        source: "/// a Character.\n".to_owned(),
        input_form: InputForm::Marginalia,
        target_text: None,
        profile: Some(Profile::Plain),
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
        lint: false,
        source: "Alice a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: None,
        profile: Some(Profile::PlainLoose),
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
        lint: false,
        source: "/// a Character.\n".to_owned(),
        input_form: InputForm::Marginalia,
        target_text: None,
        profile: Some(Profile::Plain),
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
        lint: false,
        source: "Alice waited. {{[Alice] a Character.}}".to_owned(),
        input_form: InputForm::Intralinea,
        target_text: None,
        profile: Some(Profile::Plain),
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
        lint: false,
        source: format!("Alice score {literal}.\n"),
        input_form: InputForm::Commentaria,
        target_text: None,
        profile: Some(Profile::Plain),
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
fn export_honours_profile_directive_when_no_profile_is_requested() {
    let document = export_json(ExportRequest {
        lint: false,
        source: "@profile plain-loose\n\n[Alice b] a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Alice   b waited.".to_owned()),
        profile: None,
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["target"]["profile"], "plain-loose");
    assert_eq!(value["diagnostics"], json!([]));
    assert_eq!(
        value["resolutions"][0]["spans"],
        json!([{"start": 0, "end": 9}])
    );
}

#[test]
fn requested_profile_overrides_profile_directive() {
    let document = export_json(ExportRequest {
        lint: false,
        source: "@profile plain-loose\n\n[Alice b] a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Alice   b waited.".to_owned()),
        profile: Some(Profile::Plain),
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["target"]["profile"], "plain");
    assert_eq!(value["diagnostics"][0]["code"], "SNIPPET_NOT_FOUND");
}

#[test]
fn unsupported_profile_directive_is_diagnosed_and_falls_back_to_plain() {
    let document = export_json(ExportRequest {
        lint: false,
        source: "@profile rtf-loose\n\nAlice a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: None,
        profile: None,
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["target"]["profile"], "plain");
    assert_eq!(value["diagnostics"][0]["code"], "UNSUPPORTED_PROFILE");
    assert_eq!(value["diagnostics"][0]["severity"], "error");
    assert!(value["diagnostics"][0]["span"].is_object());
}

#[test]
fn target_directive_supplies_the_effective_target_uri() {
    let document = export_json(ExportRequest {
        lint: false,
        source: "@target <chapter.txt>\n\nAlice a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: None,
        profile: None,
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["target"]["uri"], "chapter.txt");
}

#[test]
fn duplicate_directives_warn_and_first_occurrence_wins() {
    let document = export_json(ExportRequest {
        lint: false,
        source: "@profile plain\n@profile plain-loose\n\nAlice a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: None,
        profile: None,
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["target"]["profile"], "plain");
    assert_eq!(value["diagnostics"][0]["code"], "DUPLICATE_DIRECTIVE");
    assert_eq!(value["diagnostics"][0]["severity"], "warning");
}

#[test]
fn directives_outside_commentaria_do_not_select_the_profile() {
    let document = export_json(ExportRequest {
        lint: false,
        source: "```\n@profile plain-loose\nAlice a Character.\n```\n".to_owned(),
        input_form: InputForm::Marginalia,
        target_text: None,
        profile: None,
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["target"]["profile"], "plain");
}

#[test]
fn markdown_export_includes_non_fatal_extraction_warnings() {
    let document = export_json(ExportRequest {
        lint: false,
        source: "[Alice] a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Alice <span>waited</span>.\n".to_owned()),
        profile: Some(Profile::Markdown),
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

#[test]
fn quantified_text_span_facts_distribute_per_span() {
    let document = export_json(ExportRequest {
        source: "~[Alice]+ highlight true.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Alice met Alice.".to_owned()),
        profile: Some(Profile::Plain),
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(
        value,
        json!({
            "snipxVersion": "0.1",
            "implementation": {
                "name": "snipx",
                "version": "0.1.1"
            },
            "input": {
                "form": "commentaria"
            },
            "target": {
                "profile": "plain"
            },
            "visibleText": {
                "normalisation": "NFC",
                "length": 16
            },
            "facts": [
                {
                    "subject": {
                        "kind": "textSpanSnippet",
                        "source": "[Alice]+",
                        "span": {"start": 0, "end": 5}
                    },
                    "predicate": {
                        "kind": "predicate",
                        "value": "highlight"
                    },
                    "object": {
                        "kind": "boolean",
                        "value": true
                    },
                    "source": {
                        "statement": {"start": 0, "end": 25},
                        "subject": {"start": 0, "end": 9},
                        "predicate": {"start": 10, "end": 19},
                        "object": {"start": 20, "end": 24}
                    }
                },
                {
                    "subject": {
                        "kind": "textSpanSnippet",
                        "source": "[Alice]+",
                        "span": {"start": 10, "end": 15}
                    },
                    "predicate": {
                        "kind": "predicate",
                        "value": "highlight"
                    },
                    "object": {
                        "kind": "boolean",
                        "value": true
                    },
                    "source": {
                        "statement": {"start": 0, "end": 25},
                        "subject": {"start": 0, "end": 9},
                        "predicate": {"start": 10, "end": 19},
                        "object": {"start": 20, "end": 24}
                    }
                }
            ],
            "resolutions": [{
                "source": "[Alice]+",
                "sourceSpan": {"start": 0, "end": 9},
                "spans": [
                    {"start": 0, "end": 5},
                    {"start": 10, "end": 15}
                ]
            }],
            "diagnostics": []
        })
    );
}
