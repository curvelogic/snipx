use snipx_core::{export_json, ExportDocument, ExportRequest, InputForm, Profile};

fn run(source: &str, target: &str, profile: Option<Profile>, lint: bool) -> ExportDocument {
    export_json(ExportRequest {
        source: source.to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some(target.to_owned()),
        profile,
        path: None,
        target_uri: None,
        ambient_subject: None,
        lint,
    })
}

fn lint_codes(source: &str, target: &str, profile: Option<Profile>) -> Vec<String> {
    run(source, target, profile, true)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

// FRAGILE_SHORT_ANCHOR

#[test]
fn short_anchor_warns_on_a_short_snippet_body() {
    let document = run("[Bob] a Character.\n", "Bob waited.", None, true);

    let fragile: Vec<_> = document
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "FRAGILE_SHORT_ANCHOR")
        .collect();
    assert_eq!(fragile.len(), 1);
    assert_eq!(fragile[0].severity, "warning");
    assert!(fragile[0].span.is_some());
}

#[test]
fn short_anchor_warns_on_a_short_range_endpoint() {
    let codes = lint_codes(
        "[Bob..Alice waited] a Scene.\n",
        "Bob then Alice waited.",
        None,
    );

    assert!(codes.contains(&"FRAGILE_SHORT_ANCHOR".to_owned()));
}

#[test]
fn short_anchor_does_not_warn_on_a_long_snippet_body() {
    let codes = lint_codes("[Alice waited] a Scene.\n", "Alice waited.", None);

    assert!(codes.is_empty());
}

#[test]
fn short_anchor_boundary_is_five_scalars() {
    // Exactly five scalars: no warning.
    let codes = lint_codes("[Alice] a Character.\n", "Alice waited.", None);
    assert!(codes.is_empty());

    // Four scalars: warning.
    let codes = lint_codes("[Abby] a Character.\n", "Abby waited.", None);
    assert_eq!(codes, vec!["FRAGILE_SHORT_ANCHOR".to_owned()]);
}

#[test]
fn short_anchor_ignores_whole_document_snippets() {
    let codes = lint_codes("[..] a Document.\n", "Alice waited.", None);

    assert!(codes.is_empty());
}

// FRAGILE_NEAR_DUPLICATE

#[test]
fn near_duplicate_warns_when_loose_matching_gains_spans() {
    let document = run(
        "[Alice waited] a Scene.\n",
        "Alice waited.\nAlice  waited some more.\n",
        None,
        true,
    );

    let fragile: Vec<_> = document
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "FRAGILE_NEAR_DUPLICATE")
        .collect();
    assert_eq!(fragile.len(), 1);
    assert_eq!(fragile[0].severity, "warning");
}

#[test]
fn near_duplicate_does_not_warn_when_counts_agree() {
    let codes = lint_codes(
        "[Alice waited] a Scene.\n",
        "Alice waited.\nBob left.\n",
        None,
    );

    assert!(codes.is_empty());
}

#[test]
fn near_duplicate_never_warns_under_a_loose_profile() {
    // Under plain-loose both occurrences match, so the snippet needs a
    // quantifier; the loose-vs-loose comparison is then a no-op.
    let codes = lint_codes(
        "[Alice waited]+ a Scene.\n",
        "Alice waited.\nAlice  waited some more.\n",
        Some(Profile::PlainLoose),
    );

    assert!(codes.is_empty());
}

// FRAGILE_CAPTURE_CONTEXT

#[test]
fn capture_context_warns_when_context_occurs_elsewhere() {
    let document = run(
        "[speaker: {Alice}] a Character.\n",
        "speaker: Alice\nspeaker: Bob\n",
        None,
        true,
    );

    let fragile: Vec<_> = document
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "FRAGILE_CAPTURE_CONTEXT")
        .collect();
    assert_eq!(fragile.len(), 1);
    assert_eq!(fragile[0].severity, "warning");
}

#[test]
fn capture_context_does_not_warn_when_context_is_unique() {
    let codes = lint_codes(
        "[speaker: {Alice}] a Character.\n",
        "speaker: Alice\nnarrator: Bob\n",
        None,
    );

    assert!(codes.is_empty());
}

#[test]
fn capture_context_boundary_counts_resolved_spans() {
    // The context occurs exactly as often as the snippet matches, so
    // every occurrence is accounted for: no warning.
    let codes = lint_codes(
        "[speaker: {Alice}]+ a Character.\n",
        "speaker: Alice\nspeaker: Alice\n",
        None,
    );

    assert!(codes.is_empty());
}

// Gating and purity

#[test]
fn fragility_warnings_are_lint_only() {
    let document = run("[Bob] a Character.\n", "Bob waited.", None, false);

    assert!(document
        .diagnostics
        .iter()
        .all(|diagnostic| !diagnostic.code.starts_with("FRAGILE_")));
}

#[test]
fn lint_does_not_change_resolutions_or_facts() {
    let source = "[Bob] a Character.\n";
    let target = "Bob waited.";
    let plain = run(source, target, None, false);
    let linted = run(source, target, None, true);

    assert_eq!(
        serde_json::to_value(&plain.resolutions).unwrap(),
        serde_json::to_value(&linted.resolutions).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&plain.facts).unwrap(),
        serde_json::to_value(&linted.facts).unwrap()
    );
}

#[test]
fn unresolved_snippets_are_not_analysed() {
    // [Bo] is short but never resolves; the resolution error stands
    // alone without fragility noise.
    let codes = lint_codes("[Bo] a Character.\n", "Alice waited.", None);

    assert_eq!(codes, vec!["SNIPPET_NOT_FOUND".to_owned()]);
}

#[test]
fn repeated_statements_warn_once_per_snippet_occurrence() {
    // `;` carry-forward expands one subject into two statements; the
    // shared subject snippet is analysed once.
    let codes = lint_codes("[Bob] a Character; note \"short\".\n", "Bob waited.", None);

    assert_eq!(codes, vec!["FRAGILE_SHORT_ANCHOR".to_owned()]);
}
