use unicode_normalization::UnicodeNormalization;

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Plain,
    PlainLoose,
    Markdown,
    MarkdownLoose,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleText {
    pub text: String,
    pub normalisation: &'static str,
    pub profile: Profile,
}

pub fn extract_visible_text(source: &str, profile: Profile) -> Result<VisibleText, Diagnostic> {
    match profile {
        Profile::Plain | Profile::PlainLoose => Ok(VisibleText {
            text: source.nfc().collect(),
            normalisation: "NFC",
            profile,
        }),
        Profile::Markdown | Profile::MarkdownLoose => Err(Diagnostic {
            code: DiagnosticCode::UnsupportedProfile,
            severity: Severity::Error,
            message: "Markdown visible-text extraction is not implemented".to_owned(),
            span: None,
            related: Vec::new(),
        }),
    }
}
