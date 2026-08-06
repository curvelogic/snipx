use crate::ast::{AstNode, Decoration, Statement};
use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
use crate::parser::Parse;
use crate::r#match::TextSpan;
use crate::snippet::SnippetValue;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Name(String),
    Predicate(String),
    String(String),
    Number(f64),
    InvalidNumber(String),
    Boolean(bool),
    Uri(String),
    Snippet(SnippetValue),
    TextSpanSnippet(SnippetValue),
    /// A text-span snippet after resolution, pinned to one concrete
    /// matched span. Produced only by resolve, never by expand.
    ResolvedTextSpan {
        snippet: SnippetValue,
        span: TextSpan,
    },
    LocalSubject(LocalSubject),
    WholeDocument,
    Unresolved(String),
    UnresolvedLocalSubject(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalScope {
    Sentence,
    Paragraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRegion {
    Before,
    After,
    Whole,
}

/// An intralinea local subject marker (`<`, `>`, `<>`, `<<`, `>>`,
/// `<<>>`, optionally `~`-prefixed), anchored to its enclosing
/// `{{ ... }}` block for resolution against the stripped visible text.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalSubject {
    pub marker: String,
    pub scope: LocalScope,
    pub region: LocalRegion,
    pub text_span: bool,
    pub block_span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpandedStatement {
    pub subject: Value,
    pub subject_span: Option<SourceSpan>,
    pub predicate: Value,
    pub predicate_span: Option<SourceSpan>,
    pub object: Value,
    pub object_span: Option<SourceSpan>,
    pub statement_span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExpandOptions {
    pub ambient_subject: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpandResult {
    pub statements: Vec<ExpandedStatement>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn expand(parse: &Parse, options: ExpandOptions) -> ExpandResult {
    let mut result = ExpandResult {
        statements: Vec::new(),
        diagnostics: parse.diagnostics().to_vec(),
    };

    for node in parse
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::Statement)
    {
        if let Some(statement) = Statement::cast(node) {
            expand_statement(&statement, &options, &mut result);
        }
    }

    result
}

fn expand_statement(statement: &Statement, options: &ExpandOptions, result: &mut ExpandResult) {
    let statement_span = source_span(statement.syntax());
    let explicit_subject = statement.subject();
    let mut subject_span = explicit_subject
        .as_ref()
        .map(|subject| source_span(subject.syntax()));
    let subject = match explicit_subject.and_then(|subject| value_from_node(subject.syntax())) {
        Some(value) => Some(value),
        None => match local_subject_value(statement) {
            Some((value, span)) => {
                subject_span = Some(span);
                Some(value)
            }
            None => options.ambient_subject.clone(),
        },
    };

    let Some(subject) = subject else {
        result.diagnostics.push(Diagnostic {
            code: DiagnosticCode::MissingAmbientSubject,
            severity: Severity::Error,
            message: "Subjectless statement requires an ambient subject".to_owned(),
            span: Some(source_span(statement.syntax())),
            related: Vec::new(),
        });
        return;
    };
    diagnose_invalid_number(&subject, subject_span.clone(), result);

    for decoration in statement.decorations() {
        push_decoration(&subject, subject_span.clone(), &decoration, result);
    }

    for (predicate, object_list) in statement.predicates().zip(statement.object_lists()) {
        let predicate_span = Some(source_span(predicate.syntax()));
        let predicate = Value::Predicate(predicate_text(predicate.syntax()));

        for object in object_list.objects() {
            let Some(value) = value_from_node(object.syntax()) else {
                continue;
            };
            let object_span = Some(source_span(object.syntax()));
            diagnose_invalid_number(&value, object_span.clone(), result);
            result.statements.push(ExpandedStatement {
                subject: subject.clone(),
                subject_span: subject_span.clone(),
                predicate: predicate.clone(),
                predicate_span: predicate_span.clone(),
                object: value.clone(),
                object_span: object_span.clone(),
                statement_span: statement_span.clone(),
            });

            for decoration in object.decorations() {
                push_decoration(&value, object_span.clone(), &decoration, result);
            }
        }
    }
}

fn push_decoration(
    subject: &Value,
    subject_span: Option<SourceSpan>,
    decoration: &Decoration,
    result: &mut ExpandResult,
) {
    let object_node = decoration
        .syntax()
        .children()
        .find(|child| matches!(child.kind(), SyntaxKind::String | SyntaxKind::TripleString));
    let object = object_node.as_ref().and_then(value_from_node);

    let Some(object) = object else {
        result.diagnostics.push(Diagnostic {
            code: DiagnosticCode::InvalidDecorationTarget,
            severity: Severity::Error,
            message: "Decoration requires a quoted string".to_owned(),
            span: Some(source_span(decoration.syntax())),
            related: Vec::new(),
        });
        return;
    };

    result.statements.push(ExpandedStatement {
        subject: subject.clone(),
        subject_span,
        predicate: Value::Predicate("note".to_owned()),
        predicate_span: None,
        object,
        object_span: object_node.as_ref().map(source_span),
        statement_span: source_span(decoration.syntax()),
    });
}

fn local_subject_value(statement: &Statement) -> Option<(Value, SourceSpan)> {
    let marker_node = statement
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::LocalSubjectMarker)?;
    let marker = marker_node.to_string();
    let (text_span, body) = match marker.strip_prefix('~') {
        Some(rest) => (true, rest),
        None => (false, marker.as_str()),
    };
    let (scope, region) = match body {
        "<" => (LocalScope::Sentence, LocalRegion::Before),
        ">" => (LocalScope::Sentence, LocalRegion::After),
        "<>" => (LocalScope::Sentence, LocalRegion::Whole),
        "<<" => (LocalScope::Paragraph, LocalRegion::Before),
        ">>" => (LocalScope::Paragraph, LocalRegion::After),
        "<<>>" => (LocalScope::Paragraph, LocalRegion::Whole),
        _ => return None,
    };
    let block = marker_node
        .ancestors()
        .find(|node| node.kind() == SyntaxKind::IntralineaBlock)?;
    let span = source_span(&marker_node);
    Some((
        Value::LocalSubject(LocalSubject {
            marker,
            scope,
            region,
            text_span,
            block_span: source_span(&block),
        }),
        span,
    ))
}

fn predicate_text(node: &SyntaxNode) -> String {
    let text = node.to_string();
    let trimmed = text.trim();
    trimmed
        .strip_prefix('`')
        .and_then(|text| text.strip_suffix('`'))
        .unwrap_or(trimmed)
        .to_owned()
}

fn value_from_node(node: &SyntaxNode) -> Option<Value> {
    let value_node = node.descendants().find(|candidate| {
        matches!(
            candidate.kind(),
            SyntaxKind::Snippet
                | SyntaxKind::RangeSnippet
                | SyntaxKind::Uri
                | SyntaxKind::String
                | SyntaxKind::TripleString
                | SyntaxKind::Number
                | SyntaxKind::Boolean
                | SyntaxKind::Identifier
        )
    })?;
    let text = value_node.to_string();

    match value_node.kind() {
        SyntaxKind::Snippet | SyntaxKind::RangeSnippet => {
            let value_text = node.to_string();
            let syntax = value_text.trim();
            let (text_span, source) = match syntax.strip_prefix('~') {
                Some(rest) => (true, rest),
                None => (false, syntax),
            };
            let snippet = SnippetValue::from_node(&value_node, source.to_owned());
            Some(if text_span {
                Value::TextSpanSnippet(snippet)
            } else {
                Value::Snippet(snippet)
            })
        }
        SyntaxKind::Uri => Some(Value::Uri(
            text.strip_prefix('<')
                .and_then(|text| text.strip_suffix('>'))
                .unwrap_or(&text)
                .to_owned(),
        )),
        SyntaxKind::String => Some(Value::String(unescape(&unquote(&text, 1)))),
        SyntaxKind::TripleString => Some(Value::String(dedent(&unquote(&text, 3)))),
        SyntaxKind::Number => text.parse().ok().map(|number: f64| {
            if number.is_finite() {
                Value::Number(number)
            } else {
                Value::InvalidNumber(text)
            }
        }),
        SyntaxKind::Boolean => match text.as_str() {
            "true" => Some(Value::Boolean(true)),
            "false" => Some(Value::Boolean(false)),
            _ => None,
        },
        SyntaxKind::Identifier => Some(Value::Name(text)),
        _ => None,
    }
}

fn diagnose_invalid_number(value: &Value, span: Option<SourceSpan>, result: &mut ExpandResult) {
    if let Value::InvalidNumber(source) = value {
        result.diagnostics.push(Diagnostic {
            code: DiagnosticCode::InvalidNumber,
            severity: Severity::Error,
            message: format!("JSON number is outside the finite range: {source}"),
            span,
            related: Vec::new(),
        });
    }
}

fn unescape(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('t') => output.push('\t'),
            Some('r') => output.push('\r'),
            Some('\\') => output.push('\\'),
            Some('"') => output.push('"'),
            Some('\'') => output.push('\''),
            Some('0') => output.push('\0'),
            // Unknown escapes stay verbatim rather than silently dropping text.
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}

fn dedent(text: &str) -> String {
    let Some(body) = text
        .strip_prefix('\n')
        .or_else(|| text.strip_prefix("\r\n"))
    else {
        return text.to_owned();
    };

    let lines: Vec<&str> = body.split('\n').collect();
    let common_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start_matches([' ', '\t']).len())
        .min()
        .unwrap_or(0);

    lines
        .iter()
        .map(|line| {
            line.get(common_indent.min(indent_len(line))..)
                .unwrap_or("")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn indent_len(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

fn unquote(text: &str, quote_width: usize) -> String {
    text.get(quote_width..text.len().saturating_sub(quote_width))
        .unwrap_or(text)
        .to_owned()
}

fn source_span(node: &SyntaxNode) -> SourceSpan {
    let range = node.text_range();
    SourceSpan {
        start: u32::from(range.start()) as usize,
        end: u32::from(range.end()) as usize,
    }
}
