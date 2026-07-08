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
