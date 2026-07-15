use snipx_core::{format, FormatOptions, InputForm};

#[test]
fn formatter_snapshots_formats_commentaria_statements() {
    let result = format(
        "[Alice]   a   Character.\n",
        FormatOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert!(result.diagnostics.is_empty());
    assert_eq!(result.output, "[Alice] a Character.\n");
}

#[test]
fn formatter_snapshots_preserves_marginalia_prose() {
    let src = "Prose  stays.\n\n/// [Alice]   a   Character.\n";
    let result = format(
        src,
        FormatOptions {
            input_form: InputForm::Marginalia,
        },
    );

    assert!(result.diagnostics.is_empty());
    assert_eq!(result.output, "Prose  stays.\n\n/// [Alice] a Character.\n");
}

#[test]
fn formatter_snapshots_preserves_marginalia_prose_and_non_snipx_fences_byte_for_byte() {
    let src = concat!(
        "Lead  prose.\n",
        "\n",
        "```snipx\n",
        "[Alice]   a   Character.\n",
        "```\n",
        "\n",
        "```js\n",
        "const  value = 1;\n",
        "```\n",
        "\n",
        "Tail   prose.\n",
    );
    let result = format(
        src,
        FormatOptions {
            input_form: InputForm::Marginalia,
        },
    );

    assert!(result.diagnostics.is_empty());
    assert_eq!(
        result.output,
        concat!(
            "Lead  prose.\n",
            "\n",
            "```snipx\n",
            "[Alice] a Character.\n",
            "```\n",
            "\n",
            "```js\n",
            "const  value = 1;\n",
            "```\n",
            "\n",
            "Tail   prose.\n",
        )
    );
}

#[test]
fn formatter_snapshots_preserves_intralinea_host_text() {
    let src = "Alice  promised. {{<   a   Promise}}";
    let result = format(
        src,
        FormatOptions {
            input_form: InputForm::Intralinea,
        },
    );

    assert!(result.diagnostics.is_empty());
    assert_eq!(result.output, "Alice  promised. {{< a Promise}}");
}

#[test]
fn formatter_snapshots_preserves_multiple_intralinea_host_spans_byte_for_byte() {
    let src = "Before  {{Alice   friend   Bob.}} middle\t{{<   a   Promise}}  after";
    let result = format(
        src,
        FormatOptions {
            input_form: InputForm::Intralinea,
        },
    );

    assert!(result.diagnostics.is_empty());
    assert_eq!(
        result.output,
        "Before  {{Alice friend Bob.}} middle\t{{< a Promise}}  after"
    );
}

#[test]
fn formatter_snapshots_includes_parser_diagnostics() {
    let result = format(
        "[Alice] a Character",
        FormatOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert_eq!(result.output, "[Alice] a Character");
    assert!(!result.diagnostics.is_empty());
}
