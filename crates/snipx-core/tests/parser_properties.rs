use proptest::prelude::*;
use snipx_core::{format, parse, FormatOptions, InputForm, ParseOptions};

proptest! {
    #[test]
    fn parsing_commentaria_never_panics(source in ".*") {
        let _ = parse(&source, ParseOptions { input_form: InputForm::Commentaria });
    }

    #[test]
    fn parsing_marginalia_never_panics(source in ".*") {
        let _ = parse(&source, ParseOptions { input_form: InputForm::Marginalia });
    }

    #[test]
    fn parsing_intralinea_never_panics(source in ".*") {
        let _ = parse(&source, ParseOptions { input_form: InputForm::Intralinea });
    }

    #[test]
    fn formatted_commentaria_is_parseable(source in ".*") {
        let formatted = format(&source, FormatOptions { input_form: InputForm::Commentaria });
        let _ = parse(&formatted.output, ParseOptions { input_form: InputForm::Commentaria });
    }
}
