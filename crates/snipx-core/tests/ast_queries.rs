use snipx_core::ast::{AstNode, Directive, Root};
use snipx_core::{parse, InputForm, ParseOptions};

#[test]
fn root_exposes_commentaria_region_and_statement_parts() {
    let parsed = parse(
        "@target <novel.txt>\n[Alice]   a   Character.\n",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let root = Root::cast(parsed.syntax().clone()).expect("root wrapper");
    let regions = root.regions();
    let statement = root.statements().next().expect("statement");
    let object_list = statement.object_list().expect("object list");

    assert_eq!(regions.len(), 1);
    assert_eq!(statement.subject().expect("subject").text(), "[Alice]");
    assert_eq!(statement.predicate().expect("predicate").text(), "a");
    assert_eq!(
        object_list
            .objects()
            .map(|object| object.text())
            .collect::<Vec<_>>(),
        ["Character"]
    );
    assert_eq!(
        root.directives()
            .map(|directive| directive.name().expect("directive name"))
            .collect::<Vec<_>>(),
        ["target"]
    );
}

#[test]
fn root_exposes_embedded_regions_in_source_order() {
    let parsed = parse(
        "Prose.\n/// [Alice] a Character.\n\n```snipx\n[Bob] a Character.\n```\n",
        ParseOptions {
            input_form: InputForm::Marginalia,
        },
    );
    let root = Root::cast(parsed.syntax().clone()).expect("root wrapper");
    let regions = root.regions();

    assert_eq!(regions.len(), 2);
    assert_eq!(
        regions
            .iter()
            .map(|region| { region.statements().next().expect("region statement").text() })
            .collect::<Vec<_>>(),
        ["[Alice] a Character.", "[Bob] a Character."]
    );
}

#[test]
fn directive_cast_accepts_all_directive_kinds() {
    let parsed = parse(
        "@target <novel.txt>\n@profile plain-loose\n@custom true\n",
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );

    let names = parsed
        .syntax()
        .children()
        .filter_map(Directive::cast)
        .map(|directive| directive.name().expect("directive name"))
        .collect::<Vec<_>>();

    assert_eq!(names, ["target", "profile", "custom"]);
}
