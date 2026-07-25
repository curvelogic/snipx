use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
use crate::expand::{ExpandResult, ExpandedStatement, Value};
use crate::r#match::{match_snippet, TextSpan};
use crate::visible_text::{Profile, VisibleText};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResolveOptions {
    pub profile: Option<Profile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetResolution {
    pub source: String,
    pub source_span: Option<SourceSpan>,
    pub spans: Vec<TextSpan>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolveResult {
    pub statements: Vec<ExpandedStatement>,
    pub resolutions: Vec<SnippetResolution>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn resolve(
    expanded: &ExpandResult,
    visible_text: &VisibleText,
    options: ResolveOptions,
) -> ResolveResult {
    let profile = options.profile.unwrap_or(visible_text.profile);
    let mut result = ResolveResult {
        statements: expanded.statements.clone(),
        resolutions: Vec::new(),
        diagnostics: expanded.diagnostics.clone(),
    };

    for statement in &mut result.statements {
        let subject_span = statement.subject_span.clone();
        resolve_value(
            &mut statement.subject,
            subject_span,
            visible_text,
            profile,
            &mut result.resolutions,
            &mut result.diagnostics,
        );
        let object_span = statement.object_span.clone();
        resolve_value(
            &mut statement.object,
            object_span,
            visible_text,
            profile,
            &mut result.resolutions,
            &mut result.diagnostics,
        );
    }

    result
}

fn resolve_value(
    value: &mut Value,
    source_span: Option<crate::SourceSpan>,
    visible_text: &VisibleText,
    profile: Profile,
    resolutions: &mut Vec<SnippetResolution>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let source = match value {
        Value::Snippet(source) | Value::TextSpanSnippet(source) => source.clone(),
        _ => return,
    };
    let Some((body, cardinality)) = snippet_parts(&source) else {
        diagnostics.push(diagnostic(
            DiagnosticCode::InvalidSnippet,
            format!("Invalid snippet syntax: {source}"),
            source_span,
        ));
        *value = Value::Unresolved(source);
        return;
    };

    let spans = match match_snippet(body, visible_text, profile) {
        Ok(spans) => spans,
        Err(mut error) => {
            if error.span.is_none() {
                error.span = source_span;
            }
            diagnostics.push(error);
            *value = Value::Unresolved(source);
            return;
        }
    };

    let error_code = match cardinality {
        Cardinality::ExactlyOne if spans.is_empty() => Some(DiagnosticCode::SnippetNotFound),
        Cardinality::ExactlyOne if spans.len() > 1 => Some(DiagnosticCode::SnippetAmbiguous),
        Cardinality::OneOrMore if spans.is_empty() => Some(DiagnosticCode::SnippetNotFound),
        Cardinality::ZeroOrOne if spans.len() > 1 => Some(DiagnosticCode::SnippetAmbiguous),
        _ => None,
    };

    if let Some(code) = error_code {
        let message = match code {
            DiagnosticCode::SnippetNotFound => format!("Snippet did not match: {source}"),
            DiagnosticCode::SnippetAmbiguous => {
                format!("Snippet matched more than allowed: {source}")
            }
            _ => unreachable!(),
        };
        diagnostics.push(diagnostic(code, message, source_span));
        *value = Value::Unresolved(source);
        return;
    }

    resolutions.push(SnippetResolution {
        source,
        source_span,
        spans,
    });
}

#[derive(Debug, Clone, Copy)]
enum Cardinality {
    ExactlyOne,
    OneOrMore,
    ZeroOrMore,
    ZeroOrOne,
}

fn snippet_parts(source: &str) -> Option<(&str, Cardinality)> {
    let closing = source.rfind(']')?;
    let body = source.strip_prefix('[')?.get(..closing - 1)?;
    let cardinality = match &source[closing + 1..] {
        "" => Cardinality::ExactlyOne,
        "+" => Cardinality::OneOrMore,
        "*" => Cardinality::ZeroOrMore,
        "?" => Cardinality::ZeroOrOne,
        _ => return None,
    };
    Some((body, cardinality))
}

fn diagnostic(
    code: DiagnosticCode,
    message: String,
    span: Option<crate::SourceSpan>,
) -> Diagnostic {
    Diagnostic {
        code,
        severity: Severity::Error,
        message,
        span,
        related: Vec::new(),
    }
}
