use unicode_normalization::UnicodeNormalization;

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use crate::snippet::SnippetPart;
use crate::visible_text::{Profile, VisibleText};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub(crate) struct NormalizedText {
    pub(crate) text: String,
    starts: Vec<usize>,
    ends: Vec<usize>,
}

fn match_pattern(
    pattern: &str,
    capture: Option<std::ops::Range<usize>>,
    visible_text: &VisibleText,
    profile: Profile,
) -> Result<Vec<TextSpan>, Diagnostic> {
    let loose = matches!(profile, Profile::PlainLoose | Profile::MarkdownLoose);
    let haystack = normalize(&visible_text.text, loose);
    let needle = normalize(pattern, loose);

    if needle.text.is_empty() {
        return Ok(vec![TextSpan {
            start: 0,
            end: visible_text.text.chars().count(),
        }]);
    }

    let capture_normalized = capture.map(|capture| {
        let prefix: String = pattern.chars().take(capture.start).collect();
        let through_capture: String = pattern.chars().take(capture.end).collect();
        let start = normalize(&prefix, loose).text.chars().count();
        let end = normalize(&through_capture, loose).text.chars().count();
        start..end
    });
    if capture_normalized
        .as_ref()
        .is_some_and(std::ops::Range::is_empty)
    {
        return Err(invalid(
            DiagnosticCode::InvalidSnippet,
            "Capture boundaries collapse during text normalisation",
        ));
    }

    let mut spans = Vec::new();
    let mut last_end = 0;
    for matched in find_matches(&haystack, &needle.text) {
        let selected = capture_normalized
            .clone()
            .map(|capture| (matched.start + capture.start)..(matched.start + capture.end))
            .unwrap_or(matched);
        let span = TextSpan {
            start: haystack.starts[selected.start],
            end: haystack.ends[selected.end - 1],
        };
        if span.start < last_end {
            continue;
        }
        last_end = span.end;
        spans.push(span);
    }
    Ok(spans)
}

fn match_range(
    start: &str,
    end: &str,
    visible_text: &VisibleText,
    profile: Profile,
) -> Result<Vec<TextSpan>, Diagnostic> {
    let document_end = visible_text.text.chars().count();

    match (start.is_empty(), end.is_empty()) {
        (true, true) => Ok(vec![TextSpan {
            start: 0,
            end: document_end,
        }]),
        // Open ranges resolve like any other snippet: every candidate
        // match of the open endpoint is a candidate span, and the
        // caller's cardinality rules decide whether several candidates
        // are ambiguous.
        (true, false) => Ok(match_pattern(end, None, visible_text, profile)?
            .into_iter()
            .map(|end| TextSpan {
                start: 0,
                end: end.end,
            })
            .collect()),
        (false, true) => Ok(match_pattern(start, None, visible_text, profile)?
            .into_iter()
            .map(|start| TextSpan {
                start: start.start,
                end: document_end,
            })
            .collect()),
        (false, false) => {
            let starts = match_pattern(start, None, visible_text, profile)?;
            let ends = match_pattern(end, None, visible_text, profile)?;
            let mut last_end = 0;
            let mut ranges = Vec::new();
            for start in starts {
                if start.start < last_end {
                    continue;
                }
                if let Some(end) = ends.iter().find(|end| end.start >= start.end) {
                    ranges.push(TextSpan {
                        start: start.start,
                        end: end.end,
                    });
                    last_end = end.end;
                }
            }
            Ok(ranges)
        }
    }
}

pub fn match_snippet(
    parts: &[SnippetPart],
    visible_text: &VisibleText,
    profile: Profile,
) -> Result<Vec<TextSpan>, Diagnostic> {
    let separators = parts
        .iter()
        .filter(|part| matches!(part, SnippetPart::RangeSeparator))
        .count();
    if separators > 1 {
        return Err(invalid(
            DiagnosticCode::InvalidSnippet,
            "A range snippet may contain only one range separator",
        ));
    }
    if separators == 1 {
        if parts
            .iter()
            .any(|part| matches!(part, SnippetPart::Capture { .. }))
        {
            return Err(invalid(
                DiagnosticCode::InvalidSnippet,
                "Captures are not allowed inside range snippets",
            ));
        }
        let split = parts
            .iter()
            .position(|part| matches!(part, SnippetPart::RangeSeparator))
            .expect("separator counted above");
        let start = endpoint_needle(&parts[..split])?;
        let end = endpoint_needle(&parts[split + 1..])?;
        return match_range(&start, &end, visible_text, profile);
    }

    let (pattern, capture) = assemble_pattern(parts)?;
    match_pattern(&pattern, capture, visible_text, profile)
}

/// Spec ("Quoted Snippet Text"): quotes delimit only when they wrap an
/// entire snippet body or an entire range endpoint; anywhere else they
/// are literal target text.
pub(crate) fn assemble_pattern(
    parts: &[SnippetPart],
) -> Result<(String, Option<std::ops::Range<usize>>), Diagnostic> {
    if let [SnippetPart::Quoted {
        decoded,
        terminated,
        ..
    }] = parts
    {
        if !terminated {
            return Err(invalid(
                DiagnosticCode::InvalidSnippet,
                "Quoted snippet text is not terminated",
            ));
        }
        return Ok((decoded.clone(), None));
    }

    let mut pattern = String::new();
    let mut capture = None;
    for part in parts {
        match part {
            SnippetPart::Text(text) => pattern.push_str(text),
            SnippetPart::Quoted {
                raw, terminated, ..
            } => {
                if !terminated {
                    return Err(invalid(
                        DiagnosticCode::InvalidSnippet,
                        "Quoted snippet text is not terminated",
                    ));
                }
                pattern.push_str(raw);
            }
            SnippetPart::Capture { text, terminated } => {
                if capture.is_some() {
                    return Err(invalid(
                        DiagnosticCode::InvalidSnippet,
                        "A snippet may contain at most one capture",
                    ));
                }
                if !terminated {
                    return Err(invalid(
                        DiagnosticCode::InvalidSnippet,
                        "Capture is not terminated",
                    ));
                }
                if text.is_empty() {
                    return Err(invalid(
                        DiagnosticCode::InvalidSnippet,
                        "Capture may not be empty",
                    ));
                }
                let start = pattern.chars().count();
                pattern.push_str(text);
                capture = Some(start..pattern.chars().count());
            }
            SnippetPart::RangeSeparator => unreachable!("handled by caller"),
        }
    }
    Ok((pattern, capture))
}

fn endpoint_needle(parts: &[SnippetPart]) -> Result<String, Diagnostic> {
    let (needle, capture) = assemble_pattern(parts)?;
    debug_assert!(
        capture.is_none(),
        "captures are rejected before endpoints are assembled"
    );
    Ok(needle)
}

pub(crate) fn find_matches(haystack: &NormalizedText, needle: &str) -> Vec<std::ops::Range<usize>> {
    let mut matches = Vec::new();
    let mut byte_cursor = 0;

    while let Some(relative) = haystack.text[byte_cursor..].find(needle) {
        let byte_start = byte_cursor + relative;
        let byte_end = byte_start + needle.len();
        let start = haystack.text[..byte_start].chars().count();
        let end = haystack.text[..byte_end].chars().count();
        matches.push(start..end);
        byte_cursor = byte_end;
    }

    matches
}

pub(crate) fn normalize(input: &str, loose: bool) -> NormalizedText {
    let nfc: String = input.nfc().collect();
    let mut text = String::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let mut whitespace_start = None;

    for (index, character) in nfc.chars().enumerate() {
        if loose && character.is_whitespace() {
            whitespace_start.get_or_insert(index);
            continue;
        }

        if let Some(start) = whitespace_start.take() {
            text.push(' ');
            starts.push(start);
            ends.push(index);
        }

        let replacement = if loose {
            loose_replacement(character).unwrap_or_else(|| character.to_string())
        } else {
            character.to_string()
        };
        for replacement_character in replacement.chars() {
            text.push(replacement_character);
            starts.push(index);
            ends.push(index + 1);
        }
    }

    if let Some(start) = whitespace_start {
        text.push(' ');
        starts.push(start);
        ends.push(nfc.chars().count());
    }

    NormalizedText { text, starts, ends }
}

fn loose_replacement(character: char) -> Option<String> {
    match character {
        '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2212}' => {
            Some("-".to_owned())
        }
        '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}' => Some("'".to_owned()),
        '\u{201c}' | '\u{201d}' | '\u{201e}' | '\u{201f}' => Some("\"".to_owned()),
        '\u{fb00}' => Some("ff".to_owned()),
        '\u{fb01}' => Some("fi".to_owned()),
        '\u{fb02}' => Some("fl".to_owned()),
        '\u{fb03}' => Some("ffi".to_owned()),
        '\u{fb04}' => Some("ffl".to_owned()),
        '\u{fb05}' | '\u{fb06}' => Some("st".to_owned()),
        _ => None,
    }
}

fn invalid(code: DiagnosticCode, message: &str) -> Diagnostic {
    Diagnostic {
        code,
        severity: Severity::Error,
        message: message.to_owned(),
        span: None,
        related: Vec::new(),
    }
}
