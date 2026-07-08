use snipx_core::{parse, InputForm, ParseOptions};

#[test]
fn parses_basic_commentaria_without_errors() {
    let parsed = parse(
        "[Alice] a Character.\n",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    assert!(parsed.diagnostics().is_empty());
    insta::assert_snapshot!(
        parsed.debug_tree(),
        @r###"
ROOT
  STATEMENT
    SNIPPET
      L_BRACK "["
      TEXT "Alice"
      R_BRACK "]"
    WHITESPACE " "
    IDENT "a"
    WHITESPACE " "
    IDENT "Character"
    DOT "."
  WHITESPACE "\n"
"###
    );
}
