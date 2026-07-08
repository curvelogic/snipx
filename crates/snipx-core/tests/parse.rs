use snipx_core::{parse, InputForm, ParseOptions};

#[test]
fn parse_returns_the_requested_input_form_and_debug_tree() {
    let result = parse(
        "subject: predicate object",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert_eq!(result.input_form, InputForm::Commentaria);
    assert!(result.diagnostics.is_empty());
    assert_eq!(result.debug_tree, "subject: predicate object");
}
