#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    ParseError,
    UnterminatedSnippet,
    UnterminatedString,
    UnterminatedBlockComment,
    UnterminatedIntralineaBlock,
    InvalidDirectivePosition,
    InvalidLocalSubjectMarker,
    InvalidCliUsage,
    MissingAmbientSubject,
    InvalidDecorationTarget,
    InvalidStatementTerminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedSpan {
    pub message: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub related: Vec<RelatedSpan>,
}
