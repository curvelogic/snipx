use crate::diagnostic::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputForm {
    Commentaria,
    Marginalia,
    Intralinea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    pub input_form: InputForm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    pub input_form: InputForm,
    pub diagnostics: Vec<Diagnostic>,
    pub debug_tree: String,
}
