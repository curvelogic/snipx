use pulldown_cmark::{Event, Parser, Tag, TagEnd};
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

fn extract_markdown(source: &str, profile: Profile) -> VisibleText {
    let mut text = String::new();
    let mut diagnostics = Vec::new();

    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Text(value) | Event::Code(value) => text.push_str(&value),
            Event::SoftBreak | Event::HardBreak => push_newline(&mut text),
            Event::Start(
                Tag::Paragraph
                | Tag::Heading { .. }
                | Tag::BlockQuote(_)
                | Tag::CodeBlock(_)
                | Tag::Item
                | Tag::TableRow,
            ) => push_newline(&mut text),
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::BlockQuote(_)
                | TagEnd::CodeBlock
                | TagEnd::Item
                | TagEnd::TableRow,
            ) => push_newline(&mut text),
            Event::InlineHtml(_) | Event::Html(_) => diagnostics.push(Diagnostic {
                code: DiagnosticCode::RawHtmlOmitted,
                severity: Severity::Warning,
                message: "Raw HTML is omitted from Markdown visible text".to_owned(),
                span: Some(SourceSpan {
                    start: range.start,
                    end: range.end,
                }),
                related: Vec::new(),
            }),
            Event::Start(_)
            | Event::End(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::FootnoteReference(_)
            | Event::Rule
            | Event::TaskListMarker(_) => {}
        }
    }

    VisibleText {
        text: text.nfc().collect(),
        normalisation: "NFC",
        profile,
        diagnostics,
    }
}

fn push_newline(text: &mut String) {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
}
