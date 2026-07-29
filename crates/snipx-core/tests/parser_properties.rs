use proptest::prelude::*;
use snipx_core::{format, parse, FormatOptions, InputForm, ParseOptions};

proptest! {
    #[test]
    fn parsing_commentaria_never_panics(source in "(?s:.*)") {
        let _ = parse(&source, ParseOptions { input_form: InputForm::Commentaria });
    }

    #[test]
    fn parsing_marginalia_never_panics(source in "(?s:.*)") {
        let _ = parse(&source, ParseOptions { input_form: InputForm::Marginalia });
    }

    #[test]
    fn parsing_intralinea_never_panics(source in "(?s:.*)") {
        let _ = parse(&source, ParseOptions { input_form: InputForm::Intralinea });
    }

    #[test]
    fn formatted_commentaria_preserves_diagnostics(source in "(?s:.*)") {
        let formatted = format(&source, FormatOptions { input_form: InputForm::Commentaria });
        let reparsed = parse(
            &formatted.output,
            ParseOptions { input_form: InputForm::Commentaria },
        );
        prop_assert_eq!(reparsed.diagnostics(), formatted.diagnostics.as_slice());
    }
}
