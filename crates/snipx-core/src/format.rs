use rowan::NodeOrToken;

use crate::diagnostic::Diagnostic;
use crate::input::{InputForm, ParseOptions};
use crate::parser::parse;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    pub input_form: InputForm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatResult {
    pub output: String,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn format(source: &str, options: FormatOptions) -> FormatResult {
    let parsed = parse(
        source,
        ParseOptions {
            input_form: options.input_form,
        },
    );
    let diagnostics = parsed.diagnostics().to_vec();

    if !diagnostics.is_empty() {
        return FormatResult {
            output: source.to_string(),
            diagnostics,
        };
    }

    let output = match options.input_form {
        InputForm::Commentaria => format_snipx_region(parsed.syntax()),
        InputForm::Marginalia => format_marginalia(parsed.syntax()),
        InputForm::Intralinea => format_intralinea(parsed.syntax()),
    };

    FormatResult {
        output,
        diagnostics,
    }
}

fn format_marginalia(root: &SyntaxNode) -> String {
    let mut output = String::new();
    for element in root.children_with_tokens() {
        match element {
            NodeOrToken::Node(node)
                if matches!(node.kind(), SyntaxKind::LineComment | SyntaxKind::Fence) =>
            {
                output.push_str(&format_embedded_snipx_container(&node));
            }
            NodeOrToken::Node(node) => output.push_str(&node.to_string()),
            NodeOrToken::Token(token) => output.push_str(token.text()),
        }
    }
    output
}

fn format_intralinea(root: &SyntaxNode) -> String {
    let mut output = String::new();
    for element in root.children_with_tokens() {
        match element {
            NodeOrToken::Node(node) if node.kind() == SyntaxKind::IntralineaBlock => {
                output.push_str(&format_embedded_snipx_container(&node));
            }
            NodeOrToken::Node(node) => output.push_str(&node.to_string()),
            NodeOrToken::Token(token) => output.push_str(token.text()),
        }
    }
    output
}

fn format_embedded_snipx_container(node: &SyntaxNode) -> String {
    let mut output = String::new();
    for element in node.children_with_tokens() {
        match element {
            NodeOrToken::Node(child) if is_snipx_syntax_node(child.kind()) => {
                output.push_str(&format_snipx_node(&child));
            }
            NodeOrToken::Node(child) => output.push_str(&child.to_string()),
            NodeOrToken::Token(token) => output.push_str(token.text()),
        }
    }
    output
}

fn format_snipx_region(node: &SyntaxNode) -> String {
    let mut output = String::new();
    for element in node.children_with_tokens() {
        match element {
            NodeOrToken::Node(child) if is_snipx_syntax_node(child.kind()) => {
                output.push_str(&format_snipx_node(&child));
            }
            NodeOrToken::Node(child) => output.push_str(&child.to_string()),
            NodeOrToken::Token(token) => output.push_str(token.text()),
        }
    }
    output
}

fn format_snipx_node(node: &SyntaxNode) -> String {
    match node.kind() {
        SyntaxKind::Statement
        | SyntaxKind::Subject
        | SyntaxKind::Predicate
        | SyntaxKind::Object
        | SyntaxKind::ObjectList
        | SyntaxKind::Decoration
        | SyntaxKind::Directive
        | SyntaxKind::TargetDirective
        | SyntaxKind::ProfileDirective => format_compact_node(node),
        SyntaxKind::Snippet
        | SyntaxKind::RangeSnippet
        | SyntaxKind::QuotedSnippetPart
        | SyntaxKind::Capture
        | SyntaxKind::Quantifier
        | SyntaxKind::Uri
        | SyntaxKind::String
        | SyntaxKind::TripleString
        | SyntaxKind::Number
        | SyntaxKind::Boolean
        | SyntaxKind::BacktickPredicate
        | SyntaxKind::LocalSubjectMarker
        | SyntaxKind::Identifier
        | SyntaxKind::Error => node.to_string(),
        SyntaxKind::LineComment | SyntaxKind::BlockComment => node.to_string(),
        _ => format_snipx_region(node),
    }
}

fn format_compact_node(node: &SyntaxNode) -> String {
    if contains_comment_or_error(node) {
        return node.to_string();
    }

    let mut output = String::new();
    for element in node.children_with_tokens() {
        match element {
            NodeOrToken::Node(child) => {
                let part = format_snipx_node(&child);
                append_compact_part(&mut output, &part);
            }
            NodeOrToken::Token(token) if token.kind() == SyntaxKind::Whitespace => {}
            NodeOrToken::Token(token) => append_compact_part(&mut output, token.text()),
        }
    }
    output
}

fn append_compact_part(output: &mut String, part: &str) {
    if part.is_empty() {
        return;
    }

    if should_trim_before(part) {
        trim_trailing_space(output);
    } else if needs_space(output, part) {
        output.push(' ');
    }

    output.push_str(part);
}

fn needs_space(output: &str, part: &str) -> bool {
    if output.is_empty() {
        return false;
    }

    let previous = output.chars().last().expect("output is non-empty");
    let current = part.chars().next().expect("part is non-empty");

    if matches!(current, '.' | ',' | ';' | ']' | '}') {
        return false;
    }
    if part.starts_with("::") {
        return !matches!(previous, ':' | '@');
    }
    if matches!(previous, '@' | '~' | ':' | '[' | '{' | '`' | '"') {
        return false;
    }

    true
}

fn should_trim_before(part: &str) -> bool {
    part.starts_with('.') || part.starts_with(',') || part.starts_with(';')
}

fn trim_trailing_space(output: &mut String) {
    while output.ends_with(' ') {
        output.pop();
    }
}

fn contains_comment_or_error(node: &SyntaxNode) -> bool {
    node.descendants().any(|child| {
        matches!(
            child.kind(),
            SyntaxKind::LineComment | SyntaxKind::BlockComment | SyntaxKind::Error
        )
    })
}

fn is_snipx_syntax_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Statement
            | SyntaxKind::Directive
            | SyntaxKind::TargetDirective
            | SyntaxKind::ProfileDirective
    )
}
