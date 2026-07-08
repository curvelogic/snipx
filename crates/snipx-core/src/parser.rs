use rowan::{GreenNodeBuilder, NodeOrToken};

use crate::diagnostic::Diagnostic;
use crate::input::ParseOptions;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Debug, Clone)]
pub struct Parse {
    root: SyntaxNode,
    diagnostics: Vec<Diagnostic>,
}

impl Parse {
    pub fn syntax(&self) -> &SyntaxNode {
        &self.root
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn debug_tree(&self) -> String {
        let mut out = String::new();
        format_node(&self.root, 0, &mut out);
        out
    }
}

pub fn parse(source: &str, _options: ParseOptions) -> Parse {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind::Root.into());

    if let Some(rest) = source.strip_suffix('\n') {
        if let Some((label, remainder)) =
            rest.strip_prefix('[').and_then(|tail| tail.split_once(']'))
        {
            if let Some(statement_tokens) = parse_statement_tokens(remainder) {
                builder.start_node(SyntaxKind::Statement.into());
                builder.start_node(SyntaxKind::Snippet.into());
                builder.token(SyntaxKind::LBrack.into(), "[");
                builder.token(SyntaxKind::Text.into(), label);
                builder.token(SyntaxKind::RBrack.into(), "]");
                builder.finish_node();

                for (kind, text) in statement_tokens {
                    builder.token(kind.into(), text);
                }

                builder.finish_node();
                builder.token(SyntaxKind::Whitespace.into(), "\n");
                builder.finish_node();

                return Parse {
                    root: SyntaxNode::new_root(builder.finish()),
                    diagnostics: Vec::new(),
                };
            }
        }
    }

    builder.token(SyntaxKind::Text.into(), source);
    builder.finish_node();

    Parse {
        root: SyntaxNode::new_root(builder.finish()),
        diagnostics: Vec::new(),
    }
}

fn parse_statement_tokens(remainder: &str) -> Option<Vec<(SyntaxKind, &str)>> {
    let mut tokens = Vec::new();
    let mut cursor = remainder;

    while !cursor.is_empty() {
        let first = cursor.chars().next()?;

        if first.is_whitespace() {
            let len = cursor
                .char_indices()
                .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
                .unwrap_or(cursor.len());
            let (ws, rest) = cursor.split_at(len);
            tokens.push((SyntaxKind::Whitespace, ws));
            cursor = rest;
            continue;
        }

        if first == '.' {
            let (dot, rest) = cursor.split_at(1);
            tokens.push((SyntaxKind::Dot, dot));
            cursor = rest;
            continue;
        }

        if first.is_ascii_alphabetic() {
            let len = cursor
                .char_indices()
                .find_map(|(idx, ch)| (!ch.is_ascii_alphabetic()).then_some(idx))
                .unwrap_or(cursor.len());
            let (ident, rest) = cursor.split_at(len);
            tokens.push((SyntaxKind::Identifier, ident));
            cursor = rest;
            continue;
        }

        return None;
    }

    Some(tokens)
}

fn format_node(node: &SyntaxNode, indent: usize, out: &mut String) {
    push_indent(indent, out);
    out.push_str(&format!("{:?}\n", node.kind()));

    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Node(child) => format_node(&child, indent + 1, out),
            NodeOrToken::Token(token) => {
                push_indent(indent + 1, out);
                out.push_str(&format!("{:?} {:?}\n", token.kind(), token.text()));
            }
        }
    }
}

fn push_indent(indent: usize, out: &mut String) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}
