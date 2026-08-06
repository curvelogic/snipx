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
    DuplicateDirective,
    InvalidLocalSubjectMarker,
    EmptyLocalSubject,
    InvalidCliUsage,
    MissingAmbientSubject,
    InvalidDecorationTarget,
    InvalidStatementTerminator,
    UnsupportedProfile,
    RawHtmlOmitted,
    InvalidSnippet,
    InvalidNumber,
    SnippetNotFound,
    SnippetAmbiguous,
    // Fragility lint warnings. Provisional pending ratification of
    // ADR 0004 (docs/adr/0004-fragility-diagnostics.md).
    FragileShortAnchor,
    FragileNearDuplicate,
    FragileCaptureContext,
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
