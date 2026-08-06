use std::collections::HashMap;
use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use unicode_normalization::UnicodeNormalization;

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Plain,
    PlainLoose,
    Markdown,
    MarkdownLoose,
}

impl Profile {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "plain" => Some(Self::Plain),
            "plain-loose" => Some(Self::PlainLoose),
            "markdown" => Some(Self::Markdown),
            "markdown-loose" => Some(Self::MarkdownLoose),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleText {
    pub text: String,
    pub normalisation: &'static str,
    pub profile: Profile,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn extract_visible_text(source: &str, profile: Profile) -> Result<VisibleText, Diagnostic> {
    match profile {
        Profile::Plain | Profile::PlainLoose => Ok(VisibleText {
            text: source.nfc().collect(),
            normalisation: "NFC",
            profile,
            diagnostics: Vec::new(),
        }),
        Profile::Markdown | Profile::MarkdownLoose => Ok(extract_markdown(source, profile)),
    }
}

/// See ADR 0003 (docs/adr/0003-markdown-tables-footnotes-visible-text.md)
/// for the footnote-inlining and table policies implemented here.
fn extract_markdown(source: &str, profile: Profile) -> VisibleText {
    let options = Options::ENABLE_FOOTNOTES | Options::ENABLE_TABLES;
    let events: Vec<(Event, Range<usize>)> = Parser::new_ext(source, options)
        .into_offset_iter()
        .collect();
    let definitions = collect_footnote_definitions(&events);

    let mut text = String::new();
    let mut diagnostics = Vec::new();
    let mut inlining = Vec::new();
    render_events(
        &events,
        0..events.len(),
        &definitions,
        &mut inlining,
        &mut text,
        &mut diagnostics,
    );

    VisibleText {
        text: text.nfc().collect(),
        normalisation: "NFC",
        profile,
        diagnostics,
    }
}

/// Maps each footnote label to the event range of its definition body
/// (exclusive of the surrounding Start/End events). The first
/// definition of a label wins; later duplicates are ignored.
fn collect_footnote_definitions(events: &[(Event, Range<usize>)]) -> HashMap<String, Range<usize>> {
    let mut definitions = HashMap::new();
    let mut index = 0;
    while index < events.len() {
        if let (Event::Start(Tag::FootnoteDefinition(label)), _) = &events[index] {
            let end = footnote_definition_end(events, index);
            definitions
                .entry(label.to_string())
                .or_insert((index + 1)..end);
            index = end + 1;
        } else {
            index += 1;
        }
    }
    definitions
}

/// Returns the index of the End event matching the FootnoteDefinition
/// Start at `start`.
fn footnote_definition_end(events: &[(Event, Range<usize>)], start: usize) -> usize {
    let mut depth = 0usize;
    for (offset, (event, _)) in events[start..].iter().enumerate() {
        match event {
            Event::Start(Tag::FootnoteDefinition(_)) => depth += 1,
            Event::End(TagEnd::FootnoteDefinition) => {
                depth -= 1;
                if depth == 0 {
                    return start + offset;
                }
            }
            _ => {}
        }
    }
    events.len()
}

fn render_events(
    events: &[(Event, Range<usize>)],
    range: Range<usize>,
    definitions: &HashMap<String, Range<usize>>,
    inlining: &mut Vec<String>,
    text: &mut String,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut index = range.start;
    while index < range.end {
        let (event, source_range) = &events[index];
        match event {
            // Definition bodies render at their reference points, never
            // at the definition site.
            Event::Start(Tag::FootnoteDefinition(_)) => {
                index = footnote_definition_end(events, index) + 1;
                continue;
            }
            // The footnote's text is inserted at the reference point,
            // delimited like a nested block. pulldown-cmark only emits
            // references for defined footnotes, so the lookup cannot
            // miss; a reference to the footnote currently being inlined
            // would recurse forever, so it contributes nothing.
            Event::FootnoteReference(label) => {
                if let Some(body) = definitions.get(label.as_ref()) {
                    if !inlining.iter().any(|active| active == label.as_ref()) {
                        inlining.push(label.to_string());
                        push_newline(text);
                        render_events(
                            events,
                            body.clone(),
                            definitions,
                            inlining,
                            text,
                            diagnostics,
                        );
                        push_newline(text);
                        inlining.pop();
                    }
                }
            }
            Event::Text(value) | Event::Code(value) => text.push_str(value),
            Event::SoftBreak | Event::HardBreak => push_newline(text),
            Event::Start(
                Tag::Paragraph
                | Tag::Heading { .. }
                | Tag::BlockQuote(_)
                | Tag::CodeBlock(_)
                | Tag::Item
                | Tag::Table(_)
                | Tag::TableHead
                | Tag::TableRow,
            ) => push_newline(text),
            Event::Start(Tag::TableCell) => push_cell_separator(text),
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::BlockQuote(_)
                | TagEnd::CodeBlock
                | TagEnd::Item
                | TagEnd::Table
                | TagEnd::TableHead
                | TagEnd::TableRow,
            ) => push_newline(text),
            Event::InlineHtml(_) | Event::Html(_) => diagnostics.push(Diagnostic {
                code: DiagnosticCode::RawHtmlOmitted,
                severity: Severity::Warning,
                message: "Raw HTML is omitted from Markdown visible text".to_owned(),
                span: Some(SourceSpan {
                    start: source_range.start,
                    end: source_range.end,
                }),
                related: Vec::new(),
            }),
            Event::Start(_)
            | Event::End(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::Rule
            | Event::TaskListMarker(_) => {}
        }
        index += 1;
    }
}

fn push_newline(text: &mut String) {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
}

/// Cells within a table row are separated by a single space; row
/// boundaries are newlines like every other block boundary.
fn push_cell_separator(text: &mut String) {
    if !text.is_empty() && !text.ends_with('\n') && !text.ends_with(' ') {
        text.push(' ');
    }
}
