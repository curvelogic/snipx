use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
use crate::expand::{
    ExpandResult, ExpandedStatement, LocalRegion, LocalScope, LocalSubject, Value,
};
use crate::r#match::{match_snippet, TextSpan};
use crate::snippet::Cardinality;
use crate::visible_text::{Profile, VisibleText};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolveOptions {
    pub profile: Option<Profile>,
    /// Visible-text anchor for each intralinea block, used to resolve
    /// local subject markers. `visible_offset` counts Unicode scalar
    /// values into the NFC visible text at the block's position.
    pub intralinea_anchors: Vec<IntralineaAnchor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntralineaAnchor {
    pub block_span: SourceSpan,
    pub visible_offset: usize,
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
        statements: Vec::new(),
        resolutions: Vec::new(),
        diagnostics: expanded.diagnostics.clone(),
    };

    for statement in &expanded.statements {
        let mut statement = statement.clone();
        let subject_span = statement.subject_span.clone();
        let subject_spans = resolve_value(
            &mut statement.subject,
            subject_span,
            visible_text,
            profile,
            &options.intralinea_anchors,
            &mut result.resolutions,
            &mut result.diagnostics,
        );
        let object_span = statement.object_span.clone();
        let object_spans = resolve_value(
            &mut statement.object,
            object_span,
            visible_text,
            profile,
            &options.intralinea_anchors,
            &mut result.resolutions,
            &mut result.diagnostics,
        );
        distribute(statement, subject_spans, object_spans, &mut result.statements);
    }

    result
}

/// Spec (Denotation And Text Spans): text-span snippets distribute one
/// fact per matched span; both sides distributing yields the Cartesian
/// product. Denotational values pass through as a single alternative.
fn distribute(
    statement: ExpandedStatement,
    subject_spans: Option<Vec<TextSpan>>,
    object_spans: Option<Vec<TextSpan>>,
    statements: &mut Vec<ExpandedStatement>,
) {
    let subjects = value_alternatives(statement.subject.clone(), subject_spans);
    let objects = value_alternatives(statement.object.clone(), object_spans);
    for subject in &subjects {
        for object in &objects {
            let mut replica = statement.clone();
            replica.subject = subject.clone();
            replica.object = object.clone();
            statements.push(replica);
        }
    }
}

fn value_alternatives(value: Value, spans: Option<Vec<TextSpan>>) -> Vec<Value> {
    match (value, spans) {
        (Value::TextSpanSnippet(snippet), Some(spans)) => spans
            .into_iter()
            .map(|span| Value::ResolvedTextSpan {
                snippet: snippet.clone(),
                span,
            })
            .collect(),
        (value, _) => vec![value],
    }
}

fn resolve_value(
    value: &mut Value,
    source_span: Option<crate::SourceSpan>,
    visible_text: &VisibleText,
    profile: Profile,
    anchors: &[IntralineaAnchor],
    resolutions: &mut Vec<SnippetResolution>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<TextSpan>> {
    if let Value::LocalSubject(local) = value {
        let local = local.clone();
        resolve_local_subject(
            value,
            &local,
            source_span,
            visible_text,
            anchors,
            resolutions,
            diagnostics,
        );
        return None;
    }
    let text_span = matches!(value, Value::TextSpanSnippet(_));
    let snippet = match value {
        Value::Snippet(snippet) | Value::TextSpanSnippet(snippet) => snippet.clone(),
        _ => return None,
    };

    let spans = match match_snippet(&snippet.parts, visible_text, profile) {
        // An unterminated snippet with no more specific lexical defect
        // keeps the historical generic diagnostic.
        Ok(_) if !snippet.terminated => {
            diagnostics.push(diagnostic(
                DiagnosticCode::InvalidSnippet,
                format!("Invalid snippet syntax: {}", snippet.source),
                source_span,
            ));
            *value = Value::Unresolved(snippet.source);
            return None;
        }
        Ok(spans) => spans,
        Err(mut error) => {
            if error.span.is_none() {
                error.span = source_span;
            }
            diagnostics.push(error);
            *value = Value::Unresolved(snippet.source);
            return None;
        }
    };

    let error_code = match snippet.cardinality {
        Cardinality::ExactlyOne if spans.is_empty() => Some(DiagnosticCode::SnippetNotFound),
        Cardinality::ExactlyOne if spans.len() > 1 => Some(DiagnosticCode::SnippetAmbiguous),
        Cardinality::OneOrMore if spans.is_empty() => Some(DiagnosticCode::SnippetNotFound),
        Cardinality::ZeroOrOne if spans.len() > 1 => Some(DiagnosticCode::SnippetAmbiguous),
        _ => None,
    };
    if let Some(code) = error_code {
        let message = match code {
            DiagnosticCode::SnippetNotFound => {
                format!("Snippet did not match: {}", snippet.source)
            }
            DiagnosticCode::SnippetAmbiguous => {
                format!("Snippet matched more than allowed: {}", snippet.source)
            }
            _ => unreachable!(),
        };
        diagnostics.push(diagnostic(code, message, source_span));
        *value = Value::Unresolved(snippet.source);
        return None;
    }

    resolutions.push(SnippetResolution {
        source: snippet.source,
        source_span,
        spans: spans.clone(),
    });
    text_span.then_some(spans)
}

fn resolve_local_subject(
    value: &mut Value,
    local: &LocalSubject,
    source_span: Option<SourceSpan>,
    visible_text: &VisibleText,
    anchors: &[IntralineaAnchor],
    resolutions: &mut Vec<SnippetResolution>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let anchor = anchors
        .iter()
        .find(|anchor| anchor.block_span == local.block_span);
    let Some(anchor) = anchor else {
        diagnostics.push(diagnostic(
            DiagnosticCode::InvalidLocalSubjectMarker,
            format!(
                "Local subject {} cannot be anchored in the visible text",
                local.marker
            ),
            source_span,
        ));
        *value = Value::UnresolvedLocalSubject(local.marker.clone());
        return;
    };

    let chars: Vec<char> = visible_text.text.chars().collect();
    let span = match local.scope {
        LocalScope::Sentence => sentence_span(&chars, anchor.visible_offset, local.region),
        LocalScope::Paragraph => paragraph_span(&chars, anchor.visible_offset, local.region),
    };

    if span.start >= span.end {
        diagnostics.push(diagnostic(
            DiagnosticCode::EmptyLocalSubject,
            format!("Local subject {} selects no text", local.marker),
            source_span,
        ));
        *value = Value::UnresolvedLocalSubject(local.marker.clone());
        return;
    }

    resolutions.push(SnippetResolution {
        source: local.marker.clone(),
        source_span,
        spans: vec![span],
    });
}

fn is_terminator(ch: char) -> bool {
    matches!(ch, '.' | '?' | '!')
}

/// A sentence boundary is `.`, `?`, or `!` followed by whitespace or
/// end of text (the spec's simple v0 rule).
fn is_sentence_boundary(chars: &[char], index: usize) -> bool {
    is_terminator(chars[index]) && chars.get(index + 1).is_none_or(|next| next.is_whitespace())
}

fn skip_whitespace_back(chars: &[char], mut pos: usize) -> usize {
    while pos > 0 && chars[pos - 1].is_whitespace() {
        pos -= 1;
    }
    pos
}

fn skip_whitespace_forward(chars: &[char], mut pos: usize) -> usize {
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }
    pos
}

fn forward_sentence_end(chars: &[char], from: usize) -> usize {
    let mut index = from;
    while index < chars.len() {
        if is_sentence_boundary(chars, index) {
            return index + 1;
        }
        index += 1;
    }
    chars.len()
}

fn sentence_span(chars: &[char], anchor: usize, region: LocalRegion) -> TextSpan {
    let anchor = anchor.min(chars.len());
    if region == LocalRegion::After {
        let start = skip_whitespace_forward(chars, anchor);
        return TextSpan {
            start,
            end: forward_sentence_end(chars, start),
        };
    }

    // `<` and `<>` attach backwards: a marker placed just after a
    // completed sentence refers to that sentence.
    let attach = skip_whitespace_back(chars, anchor);
    let ends_at_attach = attach > 0 && is_terminator(chars[attach - 1]);
    let scan_from = if ends_at_attach { attach - 1 } else { attach };
    let mut start = 0;
    let mut index = scan_from;
    while index > 0 {
        if is_sentence_boundary(chars, index - 1) {
            start = index;
            break;
        }
        index -= 1;
    }
    start = skip_whitespace_forward(chars, start).min(attach);

    let end = match region {
        LocalRegion::Before => attach,
        LocalRegion::Whole if ends_at_attach => attach,
        LocalRegion::Whole => forward_sentence_end(chars, attach),
        LocalRegion::After => unreachable!("handled above"),
    };
    TextSpan { start, end }
}

fn paragraph_span(chars: &[char], anchor: usize, region: LocalRegion) -> TextSpan {
    let anchor = anchor.min(chars.len());
    if region == LocalRegion::After {
        let start = skip_whitespace_forward(chars, anchor);
        let (_, end) = paragraph_bounds(chars, start);
        return TextSpan {
            start,
            end: end.max(start),
        };
    }

    let attach = skip_whitespace_back(chars, anchor);
    let (start, end) = paragraph_bounds(chars, attach.saturating_sub(1));
    let start = start.min(attach);
    let end = match region {
        LocalRegion::Before => attach,
        LocalRegion::Whole => end.max(attach),
        LocalRegion::After => unreachable!("handled above"),
    };
    TextSpan { start, end }
}

/// The paragraph containing `pos`: the surrounding maximal run of
/// non-blank lines, with the end trimmed of trailing whitespace.
fn paragraph_bounds(chars: &[char], pos: usize) -> (usize, usize) {
    let lines = line_ranges(chars);
    if lines.is_empty() {
        return (0, 0);
    }
    let pos = pos.min(chars.len());
    let mut line = lines
        .iter()
        .position(|range| pos >= range.0 && pos <= range.1)
        .unwrap_or(lines.len() - 1);

    let blank =
        |range: &(usize, usize)| chars[range.0..range.1].iter().all(|ch| ch.is_whitespace());
    if blank(&lines[line]) {
        return (lines[line].0, lines[line].0);
    }
    let mut first = line;
    while first > 0 && !blank(&lines[first - 1]) {
        first -= 1;
    }
    while line + 1 < lines.len() && !blank(&lines[line + 1]) {
        line += 1;
    }
    let start = lines[first].0;
    let end = skip_whitespace_back(chars, lines[line].1);
    (start, end)
}

/// Line ranges as `[start, end)` char offsets, excluding the newline.
fn line_ranges(chars: &[char]) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, ch) in chars.iter().enumerate() {
        if *ch == '\n' {
            lines.push((start, index));
            start = index + 1;
        }
    }
    lines.push((start, chars.len()));
    lines
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
