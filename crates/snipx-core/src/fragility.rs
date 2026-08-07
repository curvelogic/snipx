//! Lint-only fragility analysis for resolved snippets.
//!
//! Snippet anchors are exact text matches, so edits to the target can
//! silently break or re-bind them. This module inspects *already
//! resolved* snippets and warns about anchors likely to fail under
//! routine editing. It is a pure function over the resolution result:
//! it never changes statements, resolutions, or exit behaviour, and
//! every diagnostic it emits is a warning.
//!
//! The diagnostic set and its codes are provisional pending
//! ratification of ADR 0004 (docs/adr/0004-fragility-diagnostics.md).

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
use crate::expand::Value;
use crate::r#match::{assemble_pattern, find_matches, match_snippet, normalize};
use crate::resolve::ResolveResult;
use crate::snippet::{SnippetPart, SnippetValue};
use crate::visible_text::{Profile, VisibleText};

/// Anchors shorter than this many Unicode scalar values (measured on
/// the normalised needle) warn as `FRAGILE_SHORT_ANCHOR`. Chosen so a
/// typical word ("Alice") passes while sub-word fragments ("Bob")
/// warn; a tuning constant, not a spec value.
pub const SHORT_ANCHOR_THRESHOLD: usize = 5;

/// Analyse the resolved snippets in `resolved` for fragility under
/// target edits. Pure: reads the resolution result, returns warnings.
pub fn analyse_fragility(
    resolved: &ResolveResult,
    visible_text: &VisibleText,
    profile: Profile,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: Vec<(&str, &Option<SourceSpan>)> = Vec::new();

    for statement in &resolved.statements {
        let values = [
            (&statement.subject, &statement.subject_span),
            (&statement.object, &statement.object_span),
        ];
        for (value, span) in values {
            let (Value::Snippet(snippet) | Value::TextSpanSnippet(snippet)) = value else {
                continue;
            };
            if seen.contains(&(snippet.source.as_str(), span)) {
                continue;
            }
            seen.push((snippet.source.as_str(), span));
            // Failed snippets become `Value::Unresolved`, so a
            // resolution exists for every snippet still present; skip
            // defensively if the join fails.
            let Some(resolution) = resolved.resolutions.iter().find(|resolution| {
                resolution.source == snippet.source && resolution.source_span == *span
            }) else {
                continue;
            };
            analyse_snippet(
                snippet,
                span.clone(),
                resolution.spans.len(),
                visible_text,
                profile,
                &mut diagnostics,
            );
        }
    }

    diagnostics
}

fn analyse_snippet(
    snippet: &SnippetValue,
    span: Option<SourceSpan>,
    resolved_spans: usize,
    visible_text: &VisibleText,
    profile: Profile,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let loose = is_loose(profile);

    if let Some(length) = shortest_anchor(&snippet.parts, loose) {
        if length < SHORT_ANCHOR_THRESHOLD {
            diagnostics.push(warning(
                DiagnosticCode::FragileShortAnchor,
                format!(
                    "Snippet anchor is shorter than {SHORT_ANCHOR_THRESHOLD} scalars \
                     ({length}) and may break or re-bind under edits: {}",
                    snippet.source
                ),
                span.clone(),
            ));
        }
    }

    if let Some(loose_profile) = loose_variant(profile) {
        if let Ok(loose_spans) = match_snippet(&snippet.parts, visible_text, loose_profile) {
            if loose_spans.len() > resolved_spans {
                diagnostics.push(warning(
                    DiagnosticCode::FragileNearDuplicate,
                    format!(
                        "Snippet matches {} near-duplicate span(s) under loose \
                         normalisation but {} exactly; small typographic edits \
                         could re-bind it: {}",
                        loose_spans.len(),
                        resolved_spans,
                        snippet.source
                    ),
                    span.clone(),
                ));
            }
        }
    }

    if capture_context_is_ambiguous(&snippet.parts, resolved_spans, visible_text, loose) {
        diagnostics.push(warning(
            DiagnosticCode::FragileCaptureContext,
            format!(
                "Capture context also occurs elsewhere in the target; edits \
                 could re-bind the capture: {}",
                snippet.source
            ),
            span,
        ));
    }
}

/// Normalised scalar length of the weakest anchor: the whole pattern
/// for a plain snippet, the shorter non-empty endpoint for a range.
/// `None` when every anchor is empty (whole-document forms) or the
/// snippet is structurally invalid.
fn shortest_anchor(parts: &[SnippetPart], loose: bool) -> Option<usize> {
    let is_separator = |part: &SnippetPart| matches!(part, SnippetPart::RangeSeparator);
    parts
        .split(is_separator)
        .filter_map(|anchor| {
            let (pattern, _) = assemble_pattern(anchor).ok()?;
            let length = normalize(&pattern, loose).text.chars().count();
            (length > 0).then_some(length)
        })
        .min()
}

/// True when the context on either side of a capture occurs at more
/// positions in the target than the snippet resolved to.
fn capture_context_is_ambiguous(
    parts: &[SnippetPart],
    resolved_spans: usize,
    visible_text: &VisibleText,
    loose: bool,
) -> bool {
    if parts
        .iter()
        .any(|part| matches!(part, SnippetPart::RangeSeparator))
    {
        return false;
    }
    let Ok((pattern, Some(capture))) = assemble_pattern(parts) else {
        return false;
    };

    let haystack = normalize(&visible_text.text, loose);
    let prefix: String = pattern.chars().take(capture.start).collect();
    let suffix: String = pattern.chars().skip(capture.end).collect();
    [prefix, suffix].into_iter().any(|context| {
        let needle = normalize(&context, loose).text;
        !needle.is_empty() && find_matches(&haystack, &needle).len() > resolved_spans
    })
}

fn is_loose(profile: Profile) -> bool {
    matches!(profile, Profile::PlainLoose | Profile::MarkdownLoose)
}

/// The loose sibling of a strict profile; `None` when already loose.
fn loose_variant(profile: Profile) -> Option<Profile> {
    match profile {
        Profile::Plain => Some(Profile::PlainLoose),
        Profile::Markdown => Some(Profile::MarkdownLoose),
        Profile::PlainLoose | Profile::MarkdownLoose => None,
    }
}

fn warning(code: DiagnosticCode, message: String, span: Option<SourceSpan>) -> Diagnostic {
    Diagnostic {
        code,
        severity: Severity::Warning,
        message,
        span,
        related: Vec::new(),
    }
}
