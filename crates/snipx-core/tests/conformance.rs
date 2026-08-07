//! Conformance corpus runner.
//!
//! Executes every case under `conformance/cases/**` against `export_json`
//! and compares the result to `expected.json` under the contract declared
//! in `conformance/MANIFEST.json`: structural comparison, the
//! `implementation` block excluded, diagnostic codes normative but message
//! strings informative, `facts`/`resolutions` order-sensitive and
//! `diagnostics` an order-insensitive multiset.
//!
//! Set `SNIPX_CONFORMANCE_REGEN=1` to rewrite every `expected.json` from
//! the current implementation instead of comparing; diffs must be
//! human-reviewed before adoption.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use snipx_core::json::{export_json, ExportRequest, SPEC_VERSION};
use snipx_core::visible_text::Profile;
use snipx_core::InputForm;

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("cannot parse {}: {err}", path.display()))
}

/// Discover case directories (any directory containing request.json) in
/// stable path order.
fn discover_cases(cases_dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![cases_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .unwrap_or_else(|err| panic!("cannot list {}: {err}", dir.display()))
            .map(|entry| entry.expect("readable dir entry").path())
            .filter(|path| path.is_dir())
            .collect();
        entries.sort();
        for entry in entries {
            if entry.join("request.json").is_file() {
                found.push(entry);
            } else {
                stack.push(entry);
            }
        }
    }
    found.sort();
    found
}

fn string_field(object: &Map<String, Value>, key: &str, case: &Path) -> Option<String> {
    match object.get(key) {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.clone()),
        Some(other) => panic!(
            "{}: field {key} must be a string, got {other}",
            case.display()
        ),
    }
}

fn parse_ambient_subject(value: &Value, case: &Path) -> snipx_core::Value {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{}: ambientSubject must be an object", case.display()));
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{}: ambientSubject needs a string kind", case.display()));
    let field = |key: &str| {
        object
            .get(key)
            .unwrap_or_else(|| panic!("{}: ambientSubject missing {key}", case.display()))
    };
    match kind {
        "name" => snipx_core::Value::Name(field("value").as_str().expect("string").to_owned()),
        "string" => snipx_core::Value::String(field("value").as_str().expect("string").to_owned()),
        "uri" => snipx_core::Value::Uri(field("value").as_str().expect("string").to_owned()),
        "number" => snipx_core::Value::Number(field("value").as_f64().expect("number")),
        "boolean" => snipx_core::Value::Boolean(field("value").as_bool().expect("boolean")),
        other => panic!(
            "{}: unsupported ambientSubject kind {other:?}",
            case.display()
        ),
    }
}

fn parse_request(case_dir: &Path) -> ExportRequest {
    const KNOWN_FIELDS: &[&str] = &[
        "source",
        "inputForm",
        "targetText",
        "targetFile",
        "profile",
        "path",
        "targetUri",
        "ambientSubject",
    ];
    let path = case_dir.join("request.json");
    let value = read_json(&path);
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{}: request must be an object", path.display()));
    for key in object.keys() {
        assert!(
            KNOWN_FIELDS.contains(&key.as_str()),
            "{}: unknown request field {key:?}",
            path.display()
        );
    }

    let source = string_field(object, "source", &path)
        .unwrap_or_else(|| panic!("{}: request needs a source", path.display()));
    let input_form = match string_field(object, "inputForm", &path).as_deref() {
        Some("commentaria") => InputForm::Commentaria,
        Some("marginalia") => InputForm::Marginalia,
        Some("intralinea") => InputForm::Intralinea,
        other => panic!("{}: invalid inputForm {other:?}", path.display()),
    };
    let target_text = match (
        string_field(object, "targetText", &path),
        string_field(object, "targetFile", &path),
    ) {
        (Some(_), Some(_)) => panic!(
            "{}: targetText and targetFile are mutually exclusive",
            path.display()
        ),
        (Some(text), None) => Some(text),
        (None, Some(file)) => Some(
            fs::read_to_string(case_dir.join(&file))
                .unwrap_or_else(|err| panic!("{}: cannot read {file}: {err}", path.display())),
        ),
        (None, None) => None,
    };
    let profile = string_field(object, "profile", &path).map(|name| {
        Profile::from_name(&name)
            .unwrap_or_else(|| panic!("{}: unknown profile {name:?}", path.display()))
    });

    ExportRequest {
        source,
        input_form,
        target_text,
        profile,
        lint: false,
        path: string_field(object, "path", &path),
        target_uri: string_field(object, "targetUri", &path),
        ambient_subject: object
            .get("ambientSubject")
            .filter(|value| !value.is_null())
            .map(|value| parse_ambient_subject(value, &path)),
    }
}

/// Rebuild a value with object keys in sorted order, recursively. This
/// keeps stored files and comparison sort keys stable regardless of
/// whether serde_json was built with `preserve_order` (a workspace
/// feature-unification detail the corpus must not depend on).
fn sort_keys(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            let mut sorted = Map::new();
            for key in keys {
                sorted.insert(key.clone(), sort_keys(&object[key]));
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_keys).collect()),
        other => other.clone(),
    }
}

/// Reduce a document to its comparable form per the MANIFEST contract:
/// drop the implementation block, drop informative message fields, and
/// sort diagnostics into a canonical order.
fn comparable(document: &Value) -> Value {
    let mut document = sort_keys(document);
    let object = document
        .as_object_mut()
        .expect("export document must be an object");
    object.remove("implementation");
    if let Some(Value::Array(diagnostics)) = object.get_mut("diagnostics") {
        for diagnostic in diagnostics.iter_mut() {
            let diagnostic = diagnostic
                .as_object_mut()
                .expect("diagnostic must be an object");
            diagnostic.remove("message");
            if let Some(Value::Array(related)) = diagnostic.get_mut("related") {
                for entry in related.iter_mut() {
                    entry
                        .as_object_mut()
                        .expect("related span must be an object")
                        .remove("message");
                }
            }
        }
        diagnostics.sort_by_key(|diagnostic| diagnostic.to_string());
    }
    document
}

/// Serialise the actual export document, stripping the implementation
/// block (excluded from the contract and stored form) but keeping
/// informative message fields so regenerated files stay reviewable.
fn stored_form(request: ExportRequest) -> Value {
    let mut value = serde_json::to_value(export_json(request)).expect("document serialises");
    value
        .as_object_mut()
        .expect("export document must be an object")
        .remove("implementation");
    sort_keys(&value)
}

#[test]
fn conformance_corpus() {
    let root = corpus_root();
    let manifest = read_json(&root.join("MANIFEST.json"));
    assert_eq!(
        manifest.get("specVersion").and_then(Value::as_str),
        Some(SPEC_VERSION),
        "MANIFEST specVersion must match the implementation's SPEC_VERSION"
    );
    let expected_count = manifest
        .get("caseCount")
        .and_then(Value::as_u64)
        .expect("MANIFEST caseCount") as usize;

    let cases = discover_cases(&root.join("cases"));
    assert_eq!(
        cases.len(),
        expected_count,
        "MANIFEST caseCount ({expected_count}) does not match discovered cases ({}); \
         update conformance/MANIFEST.json",
        cases.len()
    );

    let regen = std::env::var_os("SNIPX_CONFORMANCE_REGEN").is_some();
    let mut failures = Vec::new();
    for case_dir in &cases {
        let name = case_dir
            .strip_prefix(root.join("cases"))
            .expect("case under cases/")
            .display()
            .to_string();
        let actual = stored_form(parse_request(case_dir));
        let expected_path = case_dir.join("expected.json");
        if regen {
            let mut text = serde_json::to_string_pretty(&actual).expect("serialises");
            text.push('\n');
            fs::write(&expected_path, text)
                .unwrap_or_else(|err| panic!("cannot write {}: {err}", expected_path.display()));
            continue;
        }
        let expected = read_json(&expected_path);
        if comparable(&expected) != comparable(&actual) {
            failures.push(format!(
                "{name}:\n  expected: {}\n  actual:   {}",
                comparable(&expected),
                comparable(&actual)
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} conformance case(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
