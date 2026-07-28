#![no_main]

use libfuzzer_sys::fuzz_target;
use snipx_core::{format, parse, FormatOptions, InputForm, ParseOptions};

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        for input_form in [
            InputForm::Commentaria,
            InputForm::Marginalia,
            InputForm::Intralinea,
        ] {
            let _ = parse(source, ParseOptions { input_form });
            let formatted = format(source, FormatOptions { input_form });
            let _ = parse(&formatted.output, ParseOptions { input_form });
        }
    }
});
