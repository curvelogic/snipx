use rowan::NodeOrToken;
use serde::Serialize;

use crate::ast::AstNode;
use crate::diagnostic::{Diagnostic, DiagnosticCode, RelatedSpan, Severity, SourceSpan};
use crate::expand::{expand, ExpandOptions, ExpandedStatement, Value};
use crate::expand::{LocalRegion, LocalScope};
use crate::input::{InputForm, ParseOptions};
use crate::resolve::{resolve, IntralineaAnchor, ResolveOptions, SnippetResolution};
use crate::syntax::SyntaxKind;
use crate::visible_text::{extract_visible_text, Profile};
use crate::{parse, TextSpan};

#[derive(Debug, Clone)]
pub struct ExportRequest {
    pub source: String,
    pub input_form: InputForm,
    pub target_text: Option<String>,
    /// Explicitly requested profile. When absent, a commentaria
    /// `@profile` directive is honoured, falling back to plain.
    pub profile: Option<Profile>,
    pub path: Option<String>,
    pub target_uri: Option<String>,
    pub ambient_subject: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDocument {
    pub snipx_version: String,
    pub implementation: JsonImplementation,
    pub input: JsonInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<JsonTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_text: Option<JsonVisibleText>,
    pub facts: Vec<JsonFact>,
    pub resolutions: Vec<JsonResolution>,
    pub diagnostics: Vec<JsonDiagnostic>,
}

impl ExportDocument {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "error")
    }

    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "warning")
    }

    pub fn has_unsupported_features(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "UNSUPPORTED_PROFILE")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonImplementation {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonInput {
    pub form: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    pub profile: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonVisibleText {
    pub normalisation: String,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonFact {
    pub subject: JsonValue,
    pub predicate: JsonValue,
    pub object: JsonValue,
    pub source: JsonFactSource,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonFactSource {
    pub statement: JsonSpan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<JsonSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predicate: Option<JsonSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<JsonSpan>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum JsonValue {
    Name {
        value: String,
    },
    Predicate {
        value: String,
    },
    String {
        value: String,
    },
    Number {
        value: f64,
    },
    Boolean {
        value: bool,
    },
    Uri {
        value: String,
    },
    Snippet {
        source: String,
    },
    TextSpanSnippet {
        source: String,
    },
    LocalSubject {
        marker: String,
        scope: String,
        region: String,
    },
    TextSpanLocalSubject {
        marker: String,
        scope: String,
        region: String,
    },
    WholeDocument,
    UnresolvedSnippet {
        source: String,
    },
    UnresolvedLocalSubject {
        marker: String,
    },
    UnresolvedNumber {
        source: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonResolution {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_span: Option<JsonSpan>,
    pub spans: Vec<JsonSpan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<JsonSpan>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<JsonRelatedSpan>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct JsonSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRelatedSpan {
    pub message: String,
    pub span: JsonSpan,
}

pub fn export_json(request: ExportRequest) -> ExportDocument {
    let parsed = parse(
        &request.source,
        ParseOptions {
            input_form: request.input_form,
        },
    );
    let header = (request.input_form == InputForm::Commentaria)
        .then(|| crate::ast::Root::cast(parsed.syntax().clone()))
        .flatten()
        .map(|root| root.header_directives())
        .unwrap_or_default();
    let mut directive_diagnostics: Vec<Diagnostic> = header
        .duplicates
        .iter()
        .map(|(name, span)| Diagnostic {
            code: DiagnosticCode::DuplicateDirective,
            severity: Severity::Warning,
            message: format!("Duplicate @{name} directive is ignored"),
            span: Some(span.clone()),
            related: Vec::new(),
        })
        .collect();
    let profile = match (request.profile, &header.profile) {
        (Some(profile), _) => profile,
        (None, Some(directive)) => match Profile::from_name(&directive.value) {
            Some(profile) => profile,
            None => {
                directive_diagnostics.push(Diagnostic {
                    code: DiagnosticCode::UnsupportedProfile,
                    severity: Severity::Error,
                    message: format!("Unsupported profile: {}", directive.value),
                    span: Some(directive.span.clone()),
                    related: Vec::new(),
                });
                Profile::Plain
            }
        },
        (None, None) => Profile::Plain,
    };

    let (implicit_target, intralinea_anchors) = match request.input_form {
        InputForm::Intralinea => {
            let (text, anchors) = intralinea_visible_source(&parsed);
            // Anchors only apply when resolving against the stripped host
            // text itself; an explicit target may be entirely different.
            if request.target_text.is_none() {
                (Some(text), anchors)
            } else {
                (Some(text), Vec::new())
            }
        }
        _ => (None, Vec::new()),
    };
    let expanded = expand(
        &parsed,
        ExpandOptions {
            ambient_subject: request.ambient_subject,
        },
    );

    let target_text = request.target_text.or(implicit_target);
    let mut visible_text = None;
    let (statements, resolutions, mut diagnostics) = if let Some(target_text) = target_text {
        match extract_visible_text(&target_text, profile) {
            Ok(visible) => {
                let extraction_diagnostics = visible.diagnostics.clone();
                let resolved = resolve(
                    &expanded,
                    &visible,
                    ResolveOptions {
                        profile: Some(profile),
                        intralinea_anchors,
                    },
                );
                visible_text = Some(JsonVisibleText {
                    normalisation: visible.normalisation.to_owned(),
                    length: visible.text.chars().count(),
                });
                (
                    resolved.statements,
                    resolved.resolutions,
                    resolved
                        .diagnostics
                        .into_iter()
                        .chain(extraction_diagnostics)
                        .collect(),
                )
            }
            Err(diagnostic) => {
                let mut diagnostics = expanded.diagnostics.clone();
                diagnostics.push(diagnostic);
                (expanded.statements, Vec::new(), diagnostics)
            }
        }
    } else {
        (expanded.statements, Vec::new(), expanded.diagnostics)
    };
    if statements.iter().any(statement_has_non_finite_number) {
        diagnostics.push(Diagnostic {
            code: DiagnosticCode::InvalidNumber,
            severity: Severity::Error,
            message: "JSON numbers must be finite".to_owned(),
            span: None,
            related: Vec::new(),
        });
    }
    diagnostics.extend(directive_diagnostics);

    let target = Some(JsonTarget {
        uri: request.target_uri.or_else(|| {
            header
                .target
                .as_ref()
                .map(|directive| directive.value.clone())
        }),
        profile: profile_name(profile).to_owned(),
    });

    ExportDocument {
        snipx_version: "0.0".to_owned(),
        implementation: JsonImplementation {
            name: "snipx".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        },
        input: JsonInput {
            form: input_form_name(request.input_form).to_owned(),
            path: request.path,
        },
        target,
        visible_text,
        facts: statements.into_iter().map(json_fact).collect(),
        resolutions: resolutions.into_iter().map(json_resolution).collect(),
        diagnostics: diagnostics.into_iter().map(json_diagnostic).collect(),
    }
}

fn intralinea_visible_source(parsed: &crate::Parse) -> (String, Vec<IntralineaAnchor>) {
    use unicode_normalization::UnicodeNormalization;

    let mut text = String::new();
    let mut anchors = Vec::new();
    for element in parsed.syntax().children_with_tokens() {
        match element {
            NodeOrToken::Node(node) if node.kind() == SyntaxKind::IntralineaBlock => {
                let range = node.text_range();
                anchors.push(IntralineaAnchor {
                    block_span: SourceSpan {
                        start: u32::from(range.start()) as usize,
                        end: u32::from(range.end()) as usize,
                    },
                    // The visible text is NFC-normalised before matching, so
                    // the anchor counts scalars of the normalised prefix.
                    visible_offset: text.nfc().count(),
                });
            }
            NodeOrToken::Node(node) => text.push_str(&node.to_string()),
            NodeOrToken::Token(token) => text.push_str(token.text()),
        }
    }
    (text, anchors)
}

fn json_fact(statement: ExpandedStatement) -> JsonFact {
    JsonFact {
        subject: json_value(statement.subject),
        predicate: json_value(statement.predicate),
        object: json_value(statement.object),
        source: JsonFactSource {
            statement: json_source_span(statement.statement_span),
            subject: statement.subject_span.map(json_source_span),
            predicate: statement.predicate_span.map(json_source_span),
            object: statement.object_span.map(json_source_span),
        },
    }
}

fn json_value(value: Value) -> JsonValue {
    match value {
        Value::Name(value) => JsonValue::Name { value },
        Value::Predicate(value) => JsonValue::Predicate { value },
        Value::String(value) => JsonValue::String { value },
        Value::Number(value) if value.is_finite() => JsonValue::Number { value },
        Value::Number(value) => JsonValue::UnresolvedNumber {
            source: value.to_string(),
        },
        Value::InvalidNumber(source) => JsonValue::UnresolvedNumber { source },
        Value::Boolean(value) => JsonValue::Boolean { value },
        Value::Uri(value) => JsonValue::Uri { value },
        Value::Snippet(source) => JsonValue::Snippet { source },
        Value::TextSpanSnippet(source) => JsonValue::TextSpanSnippet { source },
        Value::LocalSubject(local) => {
            let scope = local_scope_name(local.scope).to_owned();
            let region = local_region_name(local.region).to_owned();
            if local.text_span {
                JsonValue::TextSpanLocalSubject {
                    marker: local.marker,
                    scope,
                    region,
                }
            } else {
                JsonValue::LocalSubject {
                    marker: local.marker,
                    scope,
                    region,
                }
            }
        }
        Value::WholeDocument => JsonValue::WholeDocument,
        Value::Unresolved(source) => JsonValue::UnresolvedSnippet { source },
        Value::UnresolvedLocalSubject(marker) => JsonValue::UnresolvedLocalSubject { marker },
    }
}

fn local_scope_name(scope: LocalScope) -> &'static str {
    match scope {
        LocalScope::Sentence => "sentence",
        LocalScope::Paragraph => "paragraph",
    }
}

fn local_region_name(region: LocalRegion) -> &'static str {
    match region {
        LocalRegion::Before => "before",
        LocalRegion::After => "after",
        LocalRegion::Whole => "whole",
    }
}

fn statement_has_non_finite_number(statement: &ExpandedStatement) -> bool {
    [&statement.subject, &statement.predicate, &statement.object]
        .into_iter()
        .any(|value| matches!(value, Value::Number(number) if !number.is_finite()))
}

fn json_resolution(resolution: SnippetResolution) -> JsonResolution {
    JsonResolution {
        source: resolution.source,
        source_span: resolution.source_span.map(json_source_span),
        spans: resolution.spans.into_iter().map(json_text_span).collect(),
    }
}

fn json_diagnostic(diagnostic: Diagnostic) -> JsonDiagnostic {
    JsonDiagnostic {
        code: diagnostic_code(diagnostic.code).to_owned(),
        severity: severity_name(diagnostic.severity).to_owned(),
        message: diagnostic.message,
        span: diagnostic.span.map(json_source_span),
        related: diagnostic
            .related
            .into_iter()
            .map(json_related_span)
            .collect(),
    }
}

fn json_related_span(related: RelatedSpan) -> JsonRelatedSpan {
    JsonRelatedSpan {
        message: related.message,
        span: json_source_span(related.span),
    }
}

fn json_source_span(span: SourceSpan) -> JsonSpan {
    JsonSpan {
        start: span.start,
        end: span.end,
    }
}

fn json_text_span(span: TextSpan) -> JsonSpan {
    JsonSpan {
        start: span.start,
        end: span.end,
    }
}

fn input_form_name(input_form: InputForm) -> &'static str {
    match input_form {
        InputForm::Commentaria => "commentaria",
        InputForm::Marginalia => "marginalia",
        InputForm::Intralinea => "intralinea",
    }
}

fn profile_name(profile: Profile) -> &'static str {
    match profile {
        Profile::Plain => "plain",
        Profile::PlainLoose => "plain-loose",
        Profile::Markdown => "markdown",
        Profile::MarkdownLoose => "markdown-loose",
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn diagnostic_code(code: DiagnosticCode) -> &'static str {
    match code {
        DiagnosticCode::ParseError => "PARSE_ERROR",
        DiagnosticCode::UnterminatedSnippet => "UNTERMINATED_SNIPPET",
        DiagnosticCode::UnterminatedString => "UNTERMINATED_STRING",
        DiagnosticCode::UnterminatedBlockComment => "UNTERMINATED_BLOCK_COMMENT",
        DiagnosticCode::UnterminatedIntralineaBlock => "UNTERMINATED_INTRALINEA_BLOCK",
        DiagnosticCode::InvalidDirectivePosition => "INVALID_DIRECTIVE_POSITION",
        DiagnosticCode::DuplicateDirective => "DUPLICATE_DIRECTIVE",
        DiagnosticCode::InvalidLocalSubjectMarker => "INVALID_LOCAL_SUBJECT_MARKER",
        DiagnosticCode::EmptyLocalSubject => "EMPTY_LOCAL_SUBJECT",
        DiagnosticCode::InvalidCliUsage => "INVALID_CLI_USAGE",
        DiagnosticCode::MissingAmbientSubject => "MISSING_AMBIENT_SUBJECT",
        DiagnosticCode::InvalidDecorationTarget => "INVALID_DECORATION_TARGET",
        DiagnosticCode::InvalidStatementTerminator => "INVALID_STATEMENT_TERMINATOR",
        DiagnosticCode::UnsupportedProfile => "UNSUPPORTED_PROFILE",
        DiagnosticCode::RawHtmlOmitted => "RAW_HTML_OMITTED",
        DiagnosticCode::InvalidSnippet => "INVALID_SNIPPET",
        DiagnosticCode::InvalidNumber => "INVALID_NUMBER",
        DiagnosticCode::SnippetNotFound => "SNIPPET_NOT_FOUND",
        DiagnosticCode::SnippetAmbiguous => "SNIPPET_AMBIGUOUS",
    }
}
