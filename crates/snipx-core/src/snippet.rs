use rowan::NodeOrToken;

use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Debug, Clone, PartialEq)]
pub enum SnippetPart {
    /// Raw unquoted body text, matched verbatim.
    Text(String),
    /// A quoted run. `raw` includes the delimiters and undecoded escapes;
    /// `decoded` strips the delimiters and decodes only `\"` -> `"`.
    Quoted {
        raw: String,
        decoded: String,
        terminated: bool,
    },
    /// `{...}` capture; `text` is the raw inner text.
    Capture { text: String, terminated: bool },
    /// An unquoted, top-level `..` in a range snippet.
    RangeSeparator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    ExactlyOne,
    OneOrMore,
    ZeroOrMore,
    ZeroOrOne,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnippetValue {
    /// Trimmed source syntax as today (`[Alice]+`), `~` already stripped
    /// by the caller for text-span snippets. Feeds JSON output and
    /// diagnostic messages unchanged.
    pub source: String,
    pub parts: Vec<SnippetPart>,
    pub cardinality: Cardinality,
    /// False when the closing `]` is missing.
    pub terminated: bool,
}

impl SnippetValue {
    /// `node` must be a `Snippet` or `RangeSnippet` node.
    pub fn from_node(node: &SyntaxNode, source: String) -> SnippetValue {
        let is_range = node.kind() == SyntaxKind::RangeSnippet;
        let mut parts = Vec::new();
        let mut cardinality = Cardinality::ExactlyOne;
        let mut terminated = false;

        for element in node.children_with_tokens() {
            match element {
                NodeOrToken::Token(token) => match token.kind() {
                    SyntaxKind::LBrack => {}
                    SyntaxKind::RBrack => terminated = true,
                    SyntaxKind::Dot => parts.push(SnippetPart::RangeSeparator),
                    SyntaxKind::Text => {
                        // In a range snippet, split text on ".." to create separators
                        if is_range {
                            split_on_range_separator(token.text(), &mut parts);
                        } else {
                            parts.push(SnippetPart::Text(token.text().to_owned()));
                        }
                    }
                    _ => parts.push(SnippetPart::Text(token.text().to_owned())),
                },
                NodeOrToken::Node(child) => match child.kind() {
                    SyntaxKind::QuotedSnippetPart => parts.push(quoted_part(&child)),
                    SyntaxKind::Capture => parts.push(capture_part(&child)),
                    SyntaxKind::Quantifier => {
                        cardinality = match child.to_string().as_str() {
                            "+" => Cardinality::OneOrMore,
                            "*" => Cardinality::ZeroOrMore,
                            "?" => Cardinality::ZeroOrOne,
                            _ => Cardinality::ExactlyOne,
                        };
                    }
                    // Invalid captures (second capture, capture in a range)
                    // are wrapped in an Error node by the parser; surface
                    // them so the matcher reports the same InvalidSnippet
                    // errors the string lexer used to.
                    SyntaxKind::Error => match child
                        .descendants()
                        .find(|inner| inner.kind() == SyntaxKind::Capture)
                    {
                        Some(capture) => parts.push(capture_part(&capture)),
                        None => parts.push(SnippetPart::Text(child.to_string())),
                    },
                    _ => parts.push(SnippetPart::Text(child.to_string())),
                },
            }
        }

        SnippetValue {
            source,
            parts,
            cardinality,
            terminated,
        }
    }
}

/// Split text on ".." in a range snippet, creating RangeSeparator parts.
fn split_on_range_separator(text: &str, parts: &mut Vec<SnippetPart>) {
    let mut remaining = text;
    while let Some(pos) = remaining.find("..") {
        if pos > 0 {
            parts.push(SnippetPart::Text(remaining[..pos].to_owned()));
        }
        parts.push(SnippetPart::RangeSeparator);
        remaining = &remaining[pos + 2..];
    }
    if !remaining.is_empty() {
        parts.push(SnippetPart::Text(remaining.to_owned()));
    }
}

/// Snippet quoting is very literal: only the quote delimiter itself is
/// escaped, so decoding maps `\"` to `"` and nothing else.
fn quoted_part(node: &SyntaxNode) -> SnippetPart {
    let raw = node.to_string();
    let mut quotes = 0usize;
    let mut content = String::new();
    for element in node.descendants_with_tokens() {
        if let NodeOrToken::Token(token) = element {
            match token.kind() {
                SyntaxKind::Quote => quotes += 1,
                SyntaxKind::Text => content.push_str(token.text()),
                _ => {}
            }
        }
    }
    SnippetPart::Quoted {
        raw,
        decoded: content.replace("\\\"", "\""),
        terminated: quotes >= 2,
    }
}

fn capture_part(node: &SyntaxNode) -> SnippetPart {
    let mut text = String::new();
    let mut terminated = false;
    for element in node.children_with_tokens() {
        if let NodeOrToken::Token(token) = element {
            match token.kind() {
                SyntaxKind::Text => text.push_str(token.text()),
                SyntaxKind::RBrace => terminated = true,
                _ => {}
            }
        }
    }
    SnippetPart::Capture { text, terminated }
}
