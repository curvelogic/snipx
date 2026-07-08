pub mod diagnostic;
pub mod input;

pub use diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
pub use input::{InputForm, ParseOptions, ParseResult};

pub fn parse(source: &str, options: ParseOptions) -> ParseResult {
    ParseResult {
        input_form: options.input_form,
        diagnostics: Vec::new(),
        debug_tree: source.to_owned(),
    }
}
