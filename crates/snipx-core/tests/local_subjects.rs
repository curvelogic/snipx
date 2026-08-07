use serde_json::json;
use snipx_core::{export_json, ExportRequest, InputForm};

fn export(source: &str) -> serde_json::Value {
    let document = export_json(ExportRequest {
        lint: false,
        source: source.to_owned(),
        input_form: InputForm::Intralinea,
        target_text: None,
        profile: None,
        path: None,
        target_uri: None,
        ambient_subject: None,
    });
    serde_json::to_value(document).unwrap()
}

/// Slice the resolved span (Unicode scalar offsets) out of the visible text.
fn resolved_text(value: &serde_json::Value, visible: &str) -> String {
    let span = &value["resolutions"][0]["spans"][0];
    let start = span["start"].as_u64().unwrap() as usize;
    let end = span["end"].as_u64().unwrap() as usize;
    visible.chars().skip(start).take(end - start).collect()
}

#[test]
fn sentence_before_marker_selects_the_preceding_sentence() {
    let value = export("Alice promised to return before dawn. {{< a Promise}} Bob waited.");
    let visible = "Alice promised to return before dawn.  Bob waited.";

    assert_eq!(value["diagnostics"], json!([]));
    assert_eq!(
        value["facts"][0]["subject"],
        json!({
            "kind": "localSubject",
            "marker": "<",
            "scope": "sentence",
            "region": "before"
        })
    );
    assert_eq!(
        resolved_text(&value, visible),
        "Alice promised to return before dawn."
    );
}

#[test]
fn sentence_after_marker_selects_the_following_sentence_text() {
    let value = export("One. {{theme Endings >}} Two two. Three.");
    let visible = "One.  Two two. Three.";

    assert_eq!(value["diagnostics"], json!([]));
    assert_eq!(value["facts"][0]["subject"]["region"], "after");
    assert_eq!(resolved_text(&value, visible), "Two two.");
}

#[test]
fn whole_sentence_marker_mid_sentence_selects_the_current_sentence() {
    let value = export("Alice opened {{<> theme Doors}} the door. Bob waited.");
    let visible = "Alice opened  the door. Bob waited.";

    assert_eq!(value["diagnostics"], json!([]));
    assert_eq!(resolved_text(&value, visible), "Alice opened  the door.");
}

#[test]
fn whole_paragraph_marker_selects_the_current_paragraph() {
    let value = export(
        "Para one line.\n\nSecond para {{<<>> theme Entrapment}} continues here.\n\nThird para.",
    );
    let visible = "Para one line.\n\nSecond para  continues here.\n\nThird para.";

    assert_eq!(value["diagnostics"], json!([]));
    assert_eq!(value["facts"][0]["subject"]["scope"], "paragraph");
    assert_eq!(
        resolved_text(&value, visible),
        "Second para  continues here."
    );
}

#[test]
fn paragraph_before_and_after_markers_split_the_paragraph() {
    let before = export("First bit. {{<< theme Start}} Second bit.");
    let before_visible = "First bit.  Second bit.";
    assert_eq!(before["diagnostics"], json!([]));
    assert_eq!(resolved_text(&before, before_visible), "First bit.");

    let after = export("First bit. {{theme End >>}} Second bit.");
    let after_visible = "First bit.  Second bit.";
    assert_eq!(after["diagnostics"], json!([]));
    assert_eq!(resolved_text(&after, after_visible), "Second bit.");
}

#[test]
fn tilde_marker_is_a_text_span_subject() {
    let value = export("Alice waited. {{~<> highlight true. }}");

    assert_eq!(value["diagnostics"], json!([]));
    assert_eq!(
        value["facts"][0]["subject"],
        json!({
            "kind": "textSpanLocalSubject",
            "marker": "~<>",
            "scope": "sentence",
            "region": "whole"
        })
    );
    assert_eq!(value["resolutions"][0]["source"], "~<>");
}

#[test]
fn local_subject_with_no_text_is_diagnosed() {
    let value = export("{{< a Promise}} Alice waited.");

    assert_eq!(value["diagnostics"][0]["code"], "EMPTY_LOCAL_SUBJECT");
    assert_eq!(
        value["facts"][0]["subject"]["kind"],
        "unresolvedLocalSubject"
    );
}

#[test]
fn local_subject_takes_precedence_over_ambient_subject() {
    let document = export_json(ExportRequest {
        lint: false,
        source: "Alice waited. {{< a Promise}}".to_owned(),
        input_form: InputForm::Intralinea,
        target_text: None,
        profile: None,
        path: None,
        target_uri: None,
        ambient_subject: Some(snipx_core::Value::WholeDocument),
    });
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(value["facts"][0]["subject"]["kind"], "localSubject");
}

#[test]
fn multiple_blocks_resolve_against_their_own_anchors() {
    let value = export("One one. {{< a First}} Two two. {{< a Second}}");
    let visible = "One one.  Two two. ";

    assert_eq!(value["diagnostics"], json!([]));
    assert_eq!(resolved_text(&value, visible), "One one.");
    let span = &value["resolutions"][1]["spans"][0];
    let start = span["start"].as_u64().unwrap() as usize;
    let end = span["end"].as_u64().unwrap() as usize;
    let second: String = visible.chars().skip(start).take(end - start).collect();
    assert_eq!(second, "Two two.");
}
