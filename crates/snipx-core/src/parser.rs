use rowan::{GreenNodeBuilder, NodeOrToken};

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
use crate::input::{InputForm, ParseOptions};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Debug, Clone)]
pub struct Parse {
    root: SyntaxNode,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
struct RegionParse {
    events: Vec<Event>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegionContext {
    Commentaria,
    Marginalia,
    Intralinea,
}

#[derive(Debug, Clone)]
enum Event {
    Start(SyntaxKind),
    Token(SyntaxKind, String),
    Finish,
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

pub fn parse(source: &str, options: ParseOptions) -> Parse {
    match options.input_form {
        InputForm::Commentaria => parse_commentaria(source),
        InputForm::Marginalia => parse_marginalia(source),
        InputForm::Intralinea => parse_intralinea(source),
    }
}

fn parse_commentaria(source: &str) -> Parse {
    let region = parse_snipx_region(source, 0, RegionContext::Commentaria);
    build_parse(region)
}

fn parse_marginalia(source: &str) -> Parse {
    let mut events = vec![Event::Start(SyntaxKind::Root)];
    let mut diagnostics = Vec::new();
    let mut cursor = 0;

    while cursor < source.len() {
        if let Some(marker_start) = marginalia_slash_marker(source, cursor) {
            let line_end = find_line_end(source, cursor);
            let content_end = line_content_end(source, line_end);
            let next_line = next_line_start(source, line_end);
            events.push(Event::Start(SyntaxKind::LineComment));
            if marker_start > cursor {
                events.push(Event::Token(
                    SyntaxKind::Whitespace,
                    source[cursor..marker_start].to_string(),
                ));
            }
            events.push(Event::Token(SyntaxKind::SlashSlashSlash, "///".to_string()));

            let tail_start = marker_start + 3;
            let tail = &source[tail_start..content_end];
            let whitespace_len = leading_ws_len(tail);
            let (whitespace, remainder) = tail.split_at(whitespace_len);
            if !whitespace.is_empty() {
                events.push(Event::Token(SyntaxKind::Whitespace, whitespace.to_string()));
            }
            if !remainder.is_empty() {
                let region_offset = content_end - remainder.len();
                let region =
                    parse_snipx_region(remainder, region_offset, RegionContext::Marginalia);
                replay_without_root(&region.events, &mut events);
                diagnostics.extend(region.diagnostics);
            }

            events.push(Event::Finish);
            if content_end < next_line {
                events.push(Event::Token(
                    SyntaxKind::Whitespace,
                    source[content_end..next_line].to_string(),
                ));
            }
            cursor = next_line;
            continue;
        }

        if let Some(opening_marker) = marginalia_fence_marker(source, cursor) {
            let opening_end = find_line_end(source, cursor);
            let opening_content_end = line_content_end(source, opening_end);
            let body_start = next_line_start(source, opening_end);
            let raw_info = &source[opening_marker + 3..opening_content_end];
            let info = raw_info.trim();
            let closing_fence = find_closing_fence(source, body_start);

            events.push(Event::Start(SyntaxKind::Fence));
            if opening_marker > cursor {
                events.push(Event::Token(
                    SyntaxKind::Whitespace,
                    source[cursor..opening_marker].to_string(),
                ));
            }
            events.push(Event::Token(SyntaxKind::Backtick, "```".to_string()));
            if !raw_info.is_empty() {
                events.push(Event::Token(SyntaxKind::FenceInfo, raw_info.to_string()));
            }
            if opening_content_end < body_start {
                events.push(Event::Token(
                    SyntaxKind::Whitespace,
                    source[opening_content_end..body_start].to_string(),
                ));
            }

            let body_end = closing_fence
                .map(|(closing_line_start, _)| closing_line_start)
                .unwrap_or(source.len());
            let body = &source[body_start..body_end];
            if info.is_empty() || info == "snipx" {
                let region = parse_snipx_region(body, body_start, RegionContext::Marginalia);
                replay_without_root(&region.events, &mut events);
                diagnostics.extend(region.diagnostics);
            } else if !body.is_empty() {
                events.push(Event::Token(SyntaxKind::FenceBody, body.to_string()));
            }

            if let Some((closing_line_start, closing_marker)) = closing_fence {
                if closing_marker > closing_line_start {
                    events.push(Event::Token(
                        SyntaxKind::Whitespace,
                        source[closing_line_start..closing_marker].to_string(),
                    ));
                }
                events.push(Event::Token(SyntaxKind::Backtick, "```".to_string()));
                let closing_end = find_line_end(source, closing_line_start);
                let closing_content_end = line_content_end(source, closing_end);
                let closing_suffix = &source[closing_marker + 3..closing_content_end];
                if !closing_suffix.is_empty() {
                    events.push(Event::Token(
                        SyntaxKind::FenceInfo,
                        closing_suffix.to_string(),
                    ));
                }
                let next_line = next_line_start(source, closing_end);
                if closing_content_end < next_line {
                    events.push(Event::Token(
                        SyntaxKind::Whitespace,
                        source[closing_content_end..next_line].to_string(),
                    ));
                }
                cursor = next_line;
            } else {
                diagnostics.push(Diagnostic {
                    code: DiagnosticCode::ParseError,
                    severity: Severity::Warning,
                    message: "Unterminated fence".to_string(),
                    span: Some(SourceSpan {
                        start: cursor,
                        end: source.len(),
                    }),
                });
                cursor = source.len();
            }

            events.push(Event::Finish);
            continue;
        }

        let next_special = find_next_marginalia_region(source, cursor);
        let text = &source[cursor..next_special];
        if !text.is_empty() {
            events.push(Event::Token(SyntaxKind::MarginaliaText, text.to_string()));
        }
        cursor = next_special;
    }

    events.push(Event::Finish);
    build_parse(RegionParse {
        events,
        diagnostics,
    })
}

fn parse_intralinea(source: &str) -> Parse {
    let mut events = vec![Event::Start(SyntaxKind::Root)];
    let mut diagnostics = Vec::new();
    let mut cursor = 0;

    while cursor < source.len() {
        if let Some(start) = source[cursor..].find("{{") {
            let start = cursor + start;
            if start > cursor {
                events.push(Event::Token(
                    SyntaxKind::IntralineaText,
                    source[cursor..start].to_string(),
                ));
            }

            events.push(Event::Start(SyntaxKind::IntralineaBlock));
            events.push(Event::Token(SyntaxKind::LBrace, "{".to_string()));
            events.push(Event::Token(SyntaxKind::LBrace, "{".to_string()));

            let body_start = start + 2;
            if let Some(body_end) = find_intralinea_close(source, body_start) {
                let body = &source[body_start..body_end];
                let region = parse_snipx_region(body, body_start, RegionContext::Intralinea);
                replay_without_root(&region.events, &mut events);
                diagnostics.extend(region.diagnostics);
                events.push(Event::Token(SyntaxKind::RBrace, "}".to_string()));
                events.push(Event::Token(SyntaxKind::RBrace, "}".to_string()));
                events.push(Event::Finish);
                cursor = body_end + 2;
            } else {
                let body = &source[body_start..];
                let region = parse_snipx_region(body, body_start, RegionContext::Intralinea);
                replay_without_root(&region.events, &mut events);
                diagnostics.extend(region.diagnostics);
                diagnostics.push(Diagnostic {
                    code: DiagnosticCode::UnterminatedIntralineaBlock,
                    severity: Severity::Error,
                    message: "Unterminated intralinea block".to_string(),
                    span: Some(SourceSpan {
                        start,
                        end: source.len(),
                    }),
                });
                events.push(Event::Finish);
                cursor = source.len();
            }
            continue;
        }

        events.push(Event::Token(
            SyntaxKind::IntralineaText,
            source[cursor..].to_string(),
        ));
        break;
    }

    events.push(Event::Finish);
    build_parse(RegionParse {
        events,
        diagnostics,
    })
}

fn parse_snipx_region(source: &str, offset: usize, context: RegionContext) -> RegionParse {
    let mut parser = RegionParser::new(source, offset, context);
    parser.parse();
    RegionParse {
        events: parser.events,
        diagnostics: parser.diagnostics,
    }
}

fn parse_inline_value_region(source: &str, offset: usize, context: RegionContext) -> RegionParse {
    let mut parser = RegionParser::new(source, offset, context);
    parser.parse_inline_value_until(source.len());
    parser.events.push(Event::Finish);
    RegionParse {
        events: parser.events,
        diagnostics: parser.diagnostics,
    }
}

struct RegionParser<'a> {
    source: &'a str,
    offset: usize,
    context: RegionContext,
    pos: usize,
    statement_seen: bool,
    unterminated_statement: Option<(usize, usize)>,
    events: Vec<Event>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Default, Clone, Copy)]
struct PredicateChainState {
    has_predicate: bool,
    has_error: bool,
}

#[derive(Debug, Default, Clone, Copy)]
struct ObjectListState {
    has_object: bool,
    has_error: bool,
}

#[derive(Debug, Default, Clone, Copy)]
struct ValueParseState {
    has_value: bool,
    has_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatementValueContext {
    SubjectOrObject,
    PredicateRecovery,
}

impl<'a> RegionParser<'a> {
    fn new(source: &'a str, offset: usize, context: RegionContext) -> Self {
        Self {
            source,
            offset,
            context,
            pos: 0,
            statement_seen: false,
            unterminated_statement: None,
            events: vec![Event::Start(SyntaxKind::Root)],
            diagnostics: Vec::new(),
        }
    }

    fn parse(&mut self) {
        while self.pos < self.source.len() {
            if self.starts_with("//") && !self.starts_with("///") {
                self.parse_line_comment();
            } else if self.starts_with("/*") {
                self.parse_block_comment();
            } else if self.peek_char() == Some('@') {
                self.parse_directive();
            } else if self.peek_char().is_some_and(char::is_whitespace) {
                self.parse_whitespace();
            } else {
                self.begin_statement();
                self.parse_statement();
            }
        }
        if self.context == RegionContext::Commentaria {
            self.report_unterminated_statement();
        }
        self.events.push(Event::Finish);
    }

    fn begin_statement(&mut self) {
        self.report_unterminated_statement();
        self.statement_seen = true;
    }

    fn report_unterminated_statement(&mut self) {
        if let Some((start, end)) = self.unterminated_statement.take() {
            self.push_diagnostic(
                DiagnosticCode::ParseError,
                "Expected '.' statement terminator",
                start,
                end,
            );
        }
    }

    fn parse_whitespace(&mut self) {
        let start = self.pos;
        self.consume_while(|ch| ch.is_whitespace());
        self.token_from(SyntaxKind::Whitespace, start, self.pos);
    }

    fn parse_line_comment(&mut self) {
        let start = self.pos;
        let end = find_line_end(self.source, self.pos);
        self.events.push(Event::Start(SyntaxKind::LineComment));
        self.token_from(SyntaxKind::Text, start, end);
        self.events.push(Event::Finish);
        self.pos = end;
    }

    fn parse_block_comment(&mut self) {
        let start = self.pos;
        self.events.push(Event::Start(SyntaxKind::BlockComment));
        self.pos += 2;
        if let Some(end_rel) = self.source[self.pos..].find("*/") {
            self.pos += end_rel + 2;
            self.token_from(SyntaxKind::Text, start, self.pos);
        } else {
            self.pos = self.source.len();
            self.token_from(SyntaxKind::Text, start, self.pos);
            self.push_diagnostic(
                DiagnosticCode::UnterminatedBlockComment,
                "Unterminated block comment",
                start,
                self.pos,
            );
        }
        self.events.push(Event::Finish);
    }

    fn parse_directive(&mut self) {
        let start = self.pos;
        let name_start = start + 1;
        let name_len = self.source[name_start..]
            .chars()
            .take_while(|ch| is_identifier_continue(*ch))
            .map(char::len_utf8)
            .sum::<usize>();
        let name = &self.source[name_start..name_start + name_len];
        let kind = match name {
            "target" => SyntaxKind::TargetDirective,
            "profile" => SyntaxKind::ProfileDirective,
            _ => SyntaxKind::Directive,
        };
        if self.statement_seen || !is_line_start(self.source, self.pos) {
            self.push_diagnostic(
                DiagnosticCode::InvalidDirectivePosition,
                "Directives must appear in the header before the first statement",
                start,
                start + 1,
            );
        }

        let line_end = find_line_end(self.source, self.pos);
        let content_end = line_content_end(self.source, line_end);
        self.events.push(Event::Start(kind));
        self.token(SyntaxKind::At, "@");
        self.pos += 1;
        self.consume_identifier_like();
        self.token_from(SyntaxKind::Identifier, start + 1, self.pos);
        if self.pos < content_end {
            self.token_from(
                SyntaxKind::Whitespace,
                self.pos,
                self.pos + leading_ws_len(&self.source[self.pos..content_end]),
            );
            let ws_end = self.pos + leading_ws_len(&self.source[self.pos..content_end]);
            self.pos = ws_end;
            if self.pos < content_end {
                let value_start = self.pos;
                let region = parse_inline_value_region(
                    &self.source[value_start..content_end],
                    self.offset + value_start,
                    self.context,
                );
                replay_without_root(&region.events, &mut self.events);
                self.diagnostics.extend(region.diagnostics);
            }
        }
        self.events.push(Event::Finish);
        self.pos = content_end;
    }

    fn parse_decoration(&mut self) {
        self.events.push(Event::Start(SyntaxKind::Decoration));
        self.token(SyntaxKind::ColonColon, "::");
        self.pos += 2;
        self.consume_statement_trivia(false);
        if self.peek_char() == Some('"') {
            self.parse_string();
        } else {
            let start = self.pos;
            self.events.push(Event::Start(SyntaxKind::Error));
            self.parse_value();
            self.events.push(Event::Finish);
            self.push_diagnostic(
                DiagnosticCode::ParseError,
                "Expected quoted string after decoration",
                start,
                self.pos,
            );
        }
        self.events.push(Event::Finish);
    }

    fn parse_statement(&mut self) {
        let start = self.pos;
        let mut has_subject = false;
        let mut has_decoration = false;
        let mut has_error = false;
        let predicate_state;

        self.events.push(Event::Start(SyntaxKind::Statement));
        if self.context != RegionContext::Commentaria && self.starts_with("::") {
            has_decoration = true;
            self.parse_decoration();
            self.consume_statement_trivia(true);
            predicate_state = self.parse_semicolon_continuation();
        } else if self.is_ambient_predicate_start() {
            predicate_state = self.parse_predicate_chain();
        } else {
            self.events.push(Event::Start(SyntaxKind::Subject));
            let subject_start = self.pos;
            let subject_state = self.parse_subject_like();
            if subject_state.has_value {
                has_subject = true;
            }
            if subject_state.has_error {
                has_error = true;
            }
            if !subject_state.has_value && subject_state.has_error {
                self.push_diagnostic(
                    DiagnosticCode::ParseError,
                    "Expected subject",
                    subject_start,
                    self.pos,
                );
            } else if !subject_state.has_value {
                self.push_empty_error();
                self.push_diagnostic(
                    DiagnosticCode::ParseError,
                    "Expected subject",
                    subject_start,
                    subject_start,
                );
            }
            self.events.push(Event::Finish);

            self.consume_statement_trivia(false);

            if self.starts_with("::") {
                has_decoration = true;
                self.parse_decoration();
                self.consume_statement_trivia(true);
                predicate_state = self.parse_semicolon_continuation();
            } else {
                predicate_state = self.parse_predicate_chain();
            }
        }

        has_error |= predicate_state.has_error;
        if !has_error && !has_decoration && !predicate_state.has_predicate {
            self.push_empty_error();
            self.push_diagnostic(
                DiagnosticCode::ParseError,
                if has_subject {
                    "Expected predicate after subject"
                } else {
                    "Expected statement content"
                },
                start,
                self.pos,
            );
        }

        if matches!(
            local_subject_marker_at(self.source, self.pos),
            LocalSubjectMarkerMatch::Valid(_) | LocalSubjectMarkerMatch::Invalid(_)
        ) {
            self.parse_local_subject_marker();
            self.consume_statement_trivia(false);
        }

        let terminated = if self.peek_char() == Some('.') {
            self.token(SyntaxKind::Dot, ".");
            self.pos += 1;
            true
        } else {
            false
        };

        self.events.push(Event::Finish);
        if !terminated {
            self.unterminated_statement = Some((start, self.pos));
        }
    }

    fn is_ambient_predicate_start(&self) -> bool {
        if self.context == RegionContext::Commentaria {
            return false;
        }

        matches!(self.peek_char(), Some('`'))
            || self
                .peek_char()
                .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '=')
    }

    fn parse_predicate_chain(&mut self) -> PredicateChainState {
        let mut state = PredicateChainState::default();
        while !self.at_statement_end() && self.peek_char() != Some('.') {
            let predicate_start = self.pos;
            self.events.push(Event::Start(SyntaxKind::Predicate));
            let predicate = self.parse_predicate_like();
            if predicate.has_value {
                state.has_predicate = true;
            }
            if predicate.has_error {
                self.push_diagnostic(
                    DiagnosticCode::ParseError,
                    "Expected predicate",
                    predicate_start,
                    self.pos,
                );
                state.has_error = true;
            }
            self.events.push(Event::Finish);

            self.consume_statement_trivia(false);

            if !self.at_statement_end()
                && !matches!(self.peek_char(), Some('.') | Some(';'))
                && !matches!(
                    local_subject_marker_at(self.source, self.pos),
                    LocalSubjectMarkerMatch::Valid(_) | LocalSubjectMarkerMatch::Invalid(_)
                )
            {
                let object_state = self.parse_object_list();
                state.has_error |= object_state.has_error;
                if !object_state.has_object {
                    state.has_error = true;
                }
            } else {
                self.push_empty_error();
                self.push_diagnostic(
                    DiagnosticCode::ParseError,
                    "Expected object after predicate",
                    predicate_start,
                    self.pos,
                );
                state.has_error = true;
            }

            if self.peek_char() == Some(';') {
                let semicolon_start = self.pos;
                self.token(SyntaxKind::Semicolon, ";");
                self.pos += 1;
                self.consume_statement_trivia(true);
                if self.pos >= self.source.len()
                    || matches!(self.peek_char(), Some('.') | Some('\n'))
                {
                    self.push_empty_error();
                    self.push_diagnostic(
                        DiagnosticCode::ParseError,
                        "Expected predicate after semicolon continuation",
                        semicolon_start,
                        self.pos,
                    );
                    state.has_error = true;
                    break;
                }
                continue;
            }

            break;
        }
        state
    }

    fn parse_object_list(&mut self) -> ObjectListState {
        let mut state = ObjectListState::default();
        self.events.push(Event::Start(SyntaxKind::ObjectList));
        loop {
            self.events.push(Event::Start(SyntaxKind::Object));
            let object_start = self.pos;
            let object = self.parse_object_like();
            if object.has_value {
                state.has_object = true;
            }
            if object.has_error {
                state.has_error = true;
            }
            if !object.has_value {
                if !object.has_error {
                    self.push_empty_error();
                }
                self.push_diagnostic(
                    DiagnosticCode::ParseError,
                    "Expected object",
                    object_start,
                    if object.has_error {
                        self.pos
                    } else {
                        object_start
                    },
                );
                state.has_error = true;
            }
            self.events.push(Event::Finish);

            if self.starts_with("::") {
                self.parse_decoration();
                self.consume_statement_trivia(false);
            }

            while self.starts_statement_value(false) {
                let error_start = self.pos;
                self.events.push(Event::Start(SyntaxKind::Error));
                let parsed_extra_value = self.parse_value();
                if !parsed_extra_value {
                    self.push_empty_error();
                }
                self.events.push(Event::Finish);
                self.push_diagnostic(
                    DiagnosticCode::ParseError,
                    "Expected comma between object values",
                    error_start,
                    self.pos,
                );
                state.has_error = true;
                self.consume_statement_trivia(false);

                if self.starts_with("::") {
                    self.parse_decoration();
                    self.consume_statement_trivia(false);
                }
            }

            if self.peek_char() != Some(',') {
                break;
            }
            self.token(SyntaxKind::Comma, ",");
            self.pos += 1;
            self.consume_statement_trivia(false);
        }
        self.events.push(Event::Finish);
        state
    }

    fn parse_semicolon_continuation(&mut self) -> PredicateChainState {
        let mut state = PredicateChainState::default();
        if self.peek_char() == Some(';') {
            let semicolon_start = self.pos;
            self.token(SyntaxKind::Semicolon, ";");
            self.pos += 1;
            self.consume_statement_trivia(true);
            state = self.parse_predicate_chain();
            if !state.has_predicate && !state.has_error {
                self.push_empty_error();
                self.push_diagnostic(
                    DiagnosticCode::ParseError,
                    "Expected predicate after semicolon continuation",
                    semicolon_start,
                    self.pos,
                );
                state.has_error = true;
            }
        }
        state
    }

    fn parse_subject_like(&mut self) -> ValueParseState {
        match local_subject_marker_at(self.source, self.pos) {
            LocalSubjectMarkerMatch::Valid(_) if self.context == RegionContext::Intralinea => {
                self.parse_local_subject_marker();
                self.consume_statement_trivia(false);
                ValueParseState {
                    has_value: true,
                    has_error: false,
                }
            }
            LocalSubjectMarkerMatch::Valid(_) | LocalSubjectMarkerMatch::Invalid(_) => {
                self.parse_local_subject_marker();
                self.consume_statement_trivia(false);
                ValueParseState {
                    has_value: false,
                    has_error: true,
                }
            }
            LocalSubjectMarkerMatch::None => {
                self.parse_statement_value(StatementValueContext::SubjectOrObject, false)
            }
        }
    }

    fn parse_predicate_like(&mut self) -> ValueParseState {
        if self.peek_char() == Some('`') {
            self.events
                .push(Event::Start(SyntaxKind::BacktickPredicate));
            self.parse_backtick_chunk();
            self.events.push(Event::Finish);
            ValueParseState {
                has_value: true,
                has_error: false,
            }
        } else if self.peek_char() == Some('=') {
            self.token(SyntaxKind::Text, "=");
            self.pos += 1;
            ValueParseState {
                has_value: true,
                has_error: false,
            }
        } else if self.peek_char().is_some_and(|ch| ch.is_ascii_lowercase()) {
            self.parse_predicate_identifier()
        } else {
            self.events.push(Event::Start(SyntaxKind::Error));
            let invalid =
                self.parse_statement_value(StatementValueContext::PredicateRecovery, false);
            if !invalid.has_value && !invalid.has_error {
                self.push_empty_error();
            }
            self.events.push(Event::Finish);
            ValueParseState {
                has_value: false,
                has_error: true,
            }
        }
    }

    fn parse_object_like(&mut self) -> ValueParseState {
        if self.at_statement_end()
            || matches!(self.peek_char(), Some(',') | Some(';') | Some('.'))
            || self.starts_with("::")
            || matches!(
                local_subject_marker_at(self.source, self.pos),
                LocalSubjectMarkerMatch::Valid(_) | LocalSubjectMarkerMatch::Invalid(_)
            )
        {
            ValueParseState::default()
        } else {
            let value = self.parse_statement_value(StatementValueContext::SubjectOrObject, false);
            if value.has_value || value.has_error {
                self.consume_statement_trivia(false);
            }
            value
        }
    }

    fn parse_inline_value_until(&mut self, end: usize) {
        while self.pos < end {
            if self.peek_char().is_some_and(char::is_whitespace) {
                let start = self.pos;
                while self.pos < end && self.peek_char().is_some_and(char::is_whitespace) {
                    self.advance_char();
                }
                self.token_from(SyntaxKind::Whitespace, start, self.pos);
            } else if !self.parse_value() {
                let start = self.pos;
                self.advance_char();
                self.token_from(SyntaxKind::Text, start, self.pos);
            }
        }
    }

    fn parse_statement_value(
        &mut self,
        context: StatementValueContext,
        allow_equals: bool,
    ) -> ValueParseState {
        match self.peek_char() {
            Some('[') => {
                self.parse_snippet();
                ValueParseState {
                    has_value: true,
                    has_error: false,
                }
            }
            Some('{') => {
                if context == StatementValueContext::PredicateRecovery {
                    self.parse_capture();
                    ValueParseState {
                        has_value: true,
                        has_error: false,
                    }
                } else {
                    self.parse_disallowed_statement_value(
                        "Captures are only valid inside snippets",
                        Self::parse_capture,
                    )
                }
            }
            Some('"') => {
                self.parse_string();
                ValueParseState {
                    has_value: true,
                    has_error: false,
                }
            }
            Some('<') => {
                self.parse_uri();
                ValueParseState {
                    has_value: true,
                    has_error: false,
                }
            }
            Some('`') => {
                if context == StatementValueContext::PredicateRecovery {
                    self.parse_backtick_chunk();
                    ValueParseState {
                        has_value: true,
                        has_error: false,
                    }
                } else {
                    self.parse_disallowed_statement_value(
                        "Backtick chunks are only valid as predicates",
                        Self::parse_backtick_chunk,
                    )
                }
            }
            Some('~') if self.source[self.pos..].starts_with("~[") => {
                self.token(SyntaxKind::Tilde, "~");
                self.pos += 1;
                self.parse_snippet();
                ValueParseState {
                    has_value: true,
                    has_error: false,
                }
            }
            Some('+') | Some('*') | Some('?') => {
                self.parse_invalid_statement_value();
                ValueParseState {
                    has_value: false,
                    has_error: true,
                }
            }
            Some(ch) if ch.is_ascii_digit() => {
                self.parse_number();
                ValueParseState {
                    has_value: true,
                    has_error: false,
                }
            }
            Some(ch) if is_identifier_start(ch) => {
                if context == StatementValueContext::SubjectOrObject {
                    self.parse_subject_or_object_identifier()
                } else {
                    self.parse_identifier_like();
                    ValueParseState {
                        has_value: true,
                        has_error: false,
                    }
                }
            }
            Some('=') if allow_equals => {
                self.token(SyntaxKind::Text, "=");
                self.pos += 1;
                ValueParseState {
                    has_value: true,
                    has_error: false,
                }
            }
            Some('.') | Some(',') | Some(';') | None => ValueParseState::default(),
            Some(_) => {
                self.parse_invalid_statement_value();
                ValueParseState {
                    has_value: false,
                    has_error: true,
                }
            }
        }
    }

    fn parse_disallowed_statement_value(
        &mut self,
        message: &'static str,
        parse: fn(&mut Self),
    ) -> ValueParseState {
        let start = self.pos;
        self.events.push(Event::Start(SyntaxKind::Error));
        parse(self);
        self.events.push(Event::Finish);
        self.push_diagnostic(DiagnosticCode::ParseError, message, start, self.pos);
        ValueParseState {
            has_value: false,
            has_error: true,
        }
    }

    fn parse_value(&mut self) -> bool {
        match local_subject_marker_at(self.source, self.pos) {
            LocalSubjectMarkerMatch::Valid(_) | LocalSubjectMarkerMatch::Invalid(_) => {
                self.parse_local_subject_marker();
                return true;
            }
            LocalSubjectMarkerMatch::None => {}
        }

        match self.peek_char() {
            Some('[') => {
                self.parse_snippet();
                true
            }
            Some('{') => {
                self.parse_capture();
                true
            }
            Some('"') => {
                self.parse_string();
                true
            }
            Some('<') => {
                self.parse_uri();
                true
            }
            Some('`') => {
                self.parse_backtick_chunk();
                true
            }
            Some('~') if self.source[self.pos + 1..].starts_with('[') => {
                self.token(SyntaxKind::Tilde, "~");
                self.pos += 1;
                self.parse_snippet();
                true
            }
            Some('+') | Some('*') | Some('?') => {
                let ch = self.peek_char().unwrap_or_default();
                self.events.push(Event::Start(SyntaxKind::Quantifier));
                self.token(SyntaxKind::Text, &ch.to_string());
                self.pos += ch.len_utf8();
                self.events.push(Event::Finish);
                true
            }
            Some(ch) if ch.is_ascii_digit() => {
                self.parse_number();
                true
            }
            Some(ch) if is_identifier_start(ch) => {
                self.parse_identifier_like();
                true
            }
            Some('.') => false,
            Some(',') | Some(';') => false,
            Some(_) => {
                let start = self.pos;
                self.advance_char();
                self.token_from(SyntaxKind::Text, start, self.pos);
                true
            }
            None => false,
        }
    }

    fn parse_invalid_statement_value(&mut self) {
        let start = self.pos;
        self.events.push(Event::Start(SyntaxKind::Error));
        match self.peek_char() {
            Some('+') | Some('*') | Some('?') => {
                let quantifier = self.peek_char().expect("quantifier was matched");
                self.events.push(Event::Start(SyntaxKind::Quantifier));
                self.token(SyntaxKind::Text, &quantifier.to_string());
                self.pos += quantifier.len_utf8();
                self.events.push(Event::Finish);
            }
            Some(_) => {
                let start = self.pos;
                self.advance_char();
                self.token_from(SyntaxKind::Text, start, self.pos);
            }
            None => {}
        }
        self.events.push(Event::Finish);
        self.push_diagnostic(
            DiagnosticCode::ParseError,
            "Unexpected token in statement value",
            start,
            self.pos,
        );
    }

    fn parse_snippet(&mut self) {
        let start = self.pos;
        let is_range = snippet_contains_range(
            &self.source[self.pos..],
            self.context == RegionContext::Intralinea,
        );
        let kind = if is_range {
            SyntaxKind::RangeSnippet
        } else {
            SyntaxKind::Snippet
        };
        self.events.push(Event::Start(kind));
        self.token(SyntaxKind::LBrack, "[");
        self.pos += 1;
        let mut text_start = self.pos;
        let mut capture_count = 0usize;
        while let Some(ch) = self.peek_char() {
            if ch == ']' || matches!(ch, '\n' | '\r') {
                break;
            }
            if ch == '"' {
                if self.pos > text_start {
                    self.token_from(SyntaxKind::Text, text_start, self.pos);
                }
                self.events
                    .push(Event::Start(SyntaxKind::QuotedSnippetPart));
                self.parse_string();
                self.events.push(Event::Finish);
                text_start = self.pos;
            } else if ch == '{' {
                if self.pos > text_start {
                    self.token_from(SyntaxKind::Text, text_start, self.pos);
                }
                let capture_start = self.pos;
                let invalid_capture = is_range || capture_count > 0;
                if invalid_capture {
                    self.events.push(Event::Start(SyntaxKind::Error));
                }
                self.parse_capture();
                if invalid_capture {
                    self.events.push(Event::Finish);
                    self.push_diagnostic(
                        DiagnosticCode::ParseError,
                        if is_range {
                            "Captures are not allowed inside range snippets"
                        } else {
                            "Snippets may contain at most one capture"
                        },
                        capture_start,
                        self.pos,
                    );
                }
                capture_count += 1;
                text_start = self.pos;
            } else {
                self.advance_char();
            }
        }
        if self.pos > text_start {
            self.token_from(SyntaxKind::Text, text_start, self.pos);
        }
        if self.peek_char() == Some(']') {
            self.token(SyntaxKind::RBrack, "]");
            self.pos += 1;
        } else {
            self.push_diagnostic(
                DiagnosticCode::UnterminatedSnippet,
                "Unterminated snippet",
                start,
                self.pos,
            );
        }
        if matches!(self.peek_char(), Some('+') | Some('*') | Some('?')) {
            let quantifier = self.peek_char().expect("quantifier was matched");
            self.events.push(Event::Start(SyntaxKind::Quantifier));
            self.token(SyntaxKind::Text, &quantifier.to_string());
            self.pos += quantifier.len_utf8();
            self.events.push(Event::Finish);
        }
        self.events.push(Event::Finish);
    }

    fn parse_capture(&mut self) {
        let start = self.pos;
        self.events.push(Event::Start(SyntaxKind::Capture));
        self.token(SyntaxKind::LBrace, "{");
        self.pos += 1;
        let inner_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == '}' || matches!(ch, '\n' | '\r') {
                break;
            }
            self.advance_char();
        }
        if self.pos > inner_start {
            self.token_from(SyntaxKind::Text, inner_start, self.pos);
        }
        if self.peek_char() == Some('}') {
            self.token(SyntaxKind::RBrace, "}");
            self.pos += 1;
        } else {
            self.push_diagnostic(
                DiagnosticCode::ParseError,
                "Unterminated capture",
                start,
                self.pos,
            );
        }
        self.events.push(Event::Finish);
    }

    fn parse_string(&mut self) {
        if self.source[self.pos..].starts_with("\"\"\"") {
            self.parse_triple_string();
            return;
        }

        let start = self.pos;
        self.events.push(Event::Start(SyntaxKind::String));
        self.token(SyntaxKind::Quote, "\"");
        self.pos += 1;
        let content_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == '"' || matches!(ch, '\n' | '\r') {
                break;
            }
            if ch == '\\' {
                self.advance_char();
                if self
                    .peek_char()
                    .is_some_and(|next| !matches!(next, '\n' | '\r'))
                {
                    self.advance_char();
                }
                continue;
            }
            self.advance_char();
        }
        if self.pos > content_start {
            self.token_from(SyntaxKind::Text, content_start, self.pos);
        }
        if self.peek_char() == Some('"') {
            self.token(SyntaxKind::Quote, "\"");
            self.pos += 1;
        } else {
            self.push_diagnostic(
                DiagnosticCode::UnterminatedString,
                "Unterminated string",
                start,
                self.pos,
            );
        }
        self.events.push(Event::Finish);
    }

    fn parse_triple_string(&mut self) {
        let start = self.pos;
        self.events.push(Event::Start(SyntaxKind::TripleString));
        self.token(SyntaxKind::Quote, "\"\"\"");
        self.pos += 3;
        let content_start = self.pos;
        if let Some(end_rel) = self.source[self.pos..].find("\"\"\"") {
            self.pos += end_rel;
            if self.pos > content_start {
                self.token_from(SyntaxKind::Text, content_start, self.pos);
            }
            self.token(SyntaxKind::Quote, "\"\"\"");
            self.pos += 3;
        } else {
            self.pos = self.source.len();
            if self.pos > content_start {
                self.token_from(SyntaxKind::Text, content_start, self.pos);
            }
            self.push_diagnostic(
                DiagnosticCode::UnterminatedString,
                "Unterminated triple string",
                start,
                self.pos,
            );
        }
        self.events.push(Event::Finish);
    }

    fn parse_uri(&mut self) {
        self.events.push(Event::Start(SyntaxKind::Uri));
        self.token(SyntaxKind::LAngle, "<");
        self.pos += 1;
        let content_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == '>' {
                break;
            }
            if ch.is_whitespace() {
                break;
            }
            self.advance_char();
        }
        if self.pos > content_start {
            self.token_from(SyntaxKind::Text, content_start, self.pos);
        }
        if self.peek_char() == Some('>') {
            self.token(SyntaxKind::RAngle, ">");
            self.pos += 1;
        } else {
            self.push_diagnostic(
                DiagnosticCode::ParseError,
                "Unterminated URI literal",
                content_start.saturating_sub(1),
                self.pos,
            );
        }
        self.events.push(Event::Finish);
    }

    fn parse_local_subject_marker(&mut self) {
        let marker = local_subject_marker_at(self.source, self.pos);
        let (len, invalid) = match marker {
            LocalSubjectMarkerMatch::Valid(len) => (len, false),
            LocalSubjectMarkerMatch::Invalid(len) => (len, true),
            LocalSubjectMarkerMatch::None => return,
        };
        let start = self.pos;
        let end = start + len;
        let disallowed = self.context != RegionContext::Intralinea;
        let kind = if disallowed {
            SyntaxKind::Error
        } else {
            SyntaxKind::LocalSubjectMarker
        };
        self.events.push(Event::Start(kind));
        while self.pos < end {
            match self.peek_char() {
                Some('~') => self.token(SyntaxKind::Tilde, "~"),
                Some('<') => self.token(SyntaxKind::LAngle, "<"),
                Some('>') => self.token(SyntaxKind::RAngle, ">"),
                _ => break,
            }
            self.advance_char();
        }
        self.events.push(Event::Finish);
        if invalid || disallowed {
            self.push_diagnostic(
                DiagnosticCode::InvalidLocalSubjectMarker,
                if disallowed {
                    "Local subject markers are only valid in intralinea regions"
                } else {
                    "Invalid local subject marker"
                },
                start,
                end,
            );
        }
    }

    fn parse_backtick_chunk(&mut self) {
        let start = self.pos;
        self.token(SyntaxKind::Backtick, "`");
        self.pos += 1;
        let content_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == '`' || matches!(ch, '\n' | '\r') {
                break;
            }
            self.advance_char();
        }
        if self.pos > content_start {
            self.token_from(SyntaxKind::Text, content_start, self.pos);
        }
        if self.peek_char() == Some('`') {
            self.token(SyntaxKind::Backtick, "`");
            self.pos += 1;
        } else {
            self.push_diagnostic(
                DiagnosticCode::ParseError,
                "Unterminated backtick chunk",
                start,
                self.pos,
            );
        }
    }

    fn parse_number(&mut self) {
        let start = self.pos;
        self.consume_while(|ch| ch.is_ascii_digit());
        if self.peek_char() == Some('.')
            && self.source[self.pos + 1..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_digit())
        {
            self.pos += 1;
            self.consume_while(|ch| ch.is_ascii_digit());
        }
        self.events.push(Event::Start(SyntaxKind::Number));
        self.token_from(SyntaxKind::Text, start, self.pos);
        self.events.push(Event::Finish);
    }

    fn parse_identifier_like(&mut self) {
        let start = self.pos;
        self.consume_while(is_identifier_continue);
        let text = &self.source[start..self.pos];
        let kind = if matches!(text, "true" | "false") {
            SyntaxKind::Boolean
        } else {
            SyntaxKind::Identifier
        };
        self.events.push(Event::Start(kind));
        self.token_from(SyntaxKind::Text, start, self.pos);
        self.events.push(Event::Finish);
    }

    fn parse_subject_or_object_identifier(&mut self) -> ValueParseState {
        let start = self.pos;
        self.consume_while(is_identifier_continue);
        let text = &self.source[start..self.pos];
        let kind = if matches!(text, "true" | "false") {
            SyntaxKind::Boolean
        } else {
            SyntaxKind::Identifier
        };
        let invalid = kind == SyntaxKind::Identifier
            && text
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_');

        if invalid {
            self.events.push(Event::Start(SyntaxKind::Error));
        }
        self.events.push(Event::Start(kind));
        self.token_from(SyntaxKind::Text, start, self.pos);
        self.events.push(Event::Finish);
        if invalid {
            self.events.push(Event::Finish);
            self.push_diagnostic(
                DiagnosticCode::ParseError,
                "Lowercase identifiers are only valid as predicates",
                start,
                self.pos,
            );
            ValueParseState {
                has_value: true,
                has_error: true,
            }
        } else {
            ValueParseState {
                has_value: true,
                has_error: false,
            }
        }
    }

    fn parse_predicate_identifier(&mut self) -> ValueParseState {
        let start = self.pos;
        self.consume_while(is_identifier_continue);
        let kind = if matches!(&self.source[start..self.pos], "true" | "false") {
            SyntaxKind::Boolean
        } else {
            SyntaxKind::Identifier
        };
        if kind == SyntaxKind::Boolean {
            self.events.push(Event::Start(SyntaxKind::Error));
        }
        self.events.push(Event::Start(kind));
        self.token_from(SyntaxKind::Text, start, self.pos);
        self.events.push(Event::Finish);
        if kind == SyntaxKind::Boolean {
            self.events.push(Event::Finish);
            ValueParseState {
                has_value: false,
                has_error: true,
            }
        } else {
            ValueParseState {
                has_value: true,
                has_error: false,
            }
        }
    }

    fn consume_inline_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch == '\n' {
                break;
            }
            if ch.is_whitespace() {
                let start = self.pos;
                self.consume_while(|current| current.is_whitespace() && current != '\n');
                self.token_from(SyntaxKind::Whitespace, start, self.pos);
            } else {
                break;
            }
        }
    }

    fn consume_statement_trivia(&mut self, allow_newline: bool) {
        loop {
            if self.starts_with("//") && !self.starts_with("///") {
                self.parse_line_comment();
                if !allow_newline {
                    break;
                }
                continue;
            }
            if self.starts_with("/*") {
                self.parse_block_comment();
                continue;
            }
            if self.peek_char().is_some_and(char::is_whitespace) {
                if allow_newline {
                    self.parse_whitespace();
                } else {
                    if matches!(self.peek_char(), Some('\n')) {
                        break;
                    }
                    self.consume_inline_whitespace();
                }
                continue;
            }
            break;
        }
    }

    fn at_statement_end(&self) -> bool {
        matches!(self.peek_char(), None | Some('\n'))
    }

    fn starts_statement_value(&self, allow_equals: bool) -> bool {
        if self.starts_with("::")
            || matches!(
                local_subject_marker_at(self.source, self.pos),
                LocalSubjectMarkerMatch::Valid(_) | LocalSubjectMarkerMatch::Invalid(_)
            )
        {
            return false;
        }

        match self.peek_char() {
            Some('[' | '{' | '"' | '<' | '`') => true,
            Some('~') => self.source[self.pos..].starts_with("~["),
            Some('+') | Some('*') | Some('?') => true,
            Some(ch) if ch.is_ascii_digit() => true,
            Some(ch) if is_identifier_start(ch) => true,
            Some('=') if allow_equals => true,
            _ => false,
        }
    }

    fn consume_identifier_like(&mut self) {
        self.consume_while(is_identifier_continue);
    }

    fn consume_while<F>(&mut self, mut predicate: F)
    where
        F: FnMut(char) -> bool,
    {
        while let Some(ch) = self.peek_char() {
            if !predicate(ch) {
                break;
            }
            self.advance_char();
        }
    }

    fn advance_char(&mut self) {
        if let Some(ch) = self.peek_char() {
            self.pos += ch.len_utf8();
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn starts_with(&self, pattern: &str) -> bool {
        self.source[self.pos..].starts_with(pattern)
    }

    fn token(&mut self, kind: SyntaxKind, text: &str) {
        self.events.push(Event::Token(kind, text.to_string()));
    }

    fn token_from(&mut self, kind: SyntaxKind, start: usize, end: usize) {
        if end > start {
            self.events
                .push(Event::Token(kind, self.source[start..end].to_string()));
        }
    }

    fn push_diagnostic(&mut self, code: DiagnosticCode, message: &str, start: usize, end: usize) {
        self.diagnostics.push(Diagnostic {
            code,
            severity: Severity::Error,
            message: message.to_string(),
            span: Some(SourceSpan {
                start: self.offset + start,
                end: self.offset + end,
            }),
        });
    }

    fn push_empty_error(&mut self) {
        self.events.push(Event::Start(SyntaxKind::Error));
        self.events.push(Event::Finish);
    }
}

fn build_parse(region: RegionParse) -> Parse {
    let mut builder = GreenNodeBuilder::new();
    replay(&region.events, &mut builder);
    Parse {
        root: SyntaxNode::new_root(builder.finish()),
        diagnostics: region.diagnostics,
    }
}

fn replay(events: &[Event], builder: &mut GreenNodeBuilder<'_>) {
    for event in events {
        match event {
            Event::Start(kind) => builder.start_node((*kind).into()),
            Event::Token(kind, text) => builder.token((*kind).into(), text),
            Event::Finish => builder.finish_node(),
        }
    }
}

fn replay_without_root(events: &[Event], out: &mut Vec<Event>) {
    let mut depth = 0usize;
    for event in events {
        match event {
            Event::Start(SyntaxKind::Root) if depth == 0 => {
                depth = 1;
            }
            Event::Start(_) => {
                depth += 1;
                out.push(event.clone());
            }
            Event::Finish if depth == 1 => {
                depth = 0;
            }
            Event::Finish => {
                depth = depth.saturating_sub(1);
                out.push(event.clone());
            }
            Event::Token(_, _) => out.push(event.clone()),
        }
    }
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

fn find_line_end(source: &str, from: usize) -> usize {
    source[from..]
        .find('\n')
        .map_or(source.len(), |idx| from + idx)
}

fn line_content_end(source: &str, line_end: usize) -> usize {
    if line_end < source.len() && source[..line_end].ends_with('\r') {
        line_end - 1
    } else {
        line_end
    }
}

fn next_line_start(source: &str, line_end: usize) -> usize {
    if line_end < source.len() {
        line_end + 1
    } else {
        line_end
    }
}

fn marginalia_slash_marker(source: &str, line_start: usize) -> Option<usize> {
    if !is_line_start(source, line_start) {
        return None;
    }

    let line_end = find_line_end(source, line_start);
    let content_end = line_content_end(source, line_end);
    let indentation = leading_ws_len(&source[line_start..content_end]);
    let marker_start = line_start + indentation;
    source[marker_start..content_end]
        .starts_with("///")
        .then_some(marker_start)
}

fn marginalia_fence_marker(source: &str, line_start: usize) -> Option<usize> {
    if !is_line_start(source, line_start) {
        return None;
    }

    let line_end = find_line_end(source, line_start);
    let content_end = line_content_end(source, line_end);
    let indentation = leading_ws_len(&source[line_start..content_end]);
    let marker_start = line_start + indentation;
    source[marker_start..content_end]
        .starts_with("```")
        .then_some(marker_start)
}

fn is_line_start(source: &str, pos: usize) -> bool {
    pos == 0 || source[..pos].ends_with('\n')
}

fn find_closing_fence(source: &str, from: usize) -> Option<(usize, usize)> {
    let mut cursor = from;
    while cursor < source.len() {
        if let Some(marker_start) = marginalia_fence_marker(source, cursor) {
            return Some((cursor, marker_start));
        }
        let line_end = find_line_end(source, cursor);
        cursor = next_line_start(source, line_end);
    }
    None
}

fn find_next_marginalia_region(source: &str, from: usize) -> usize {
    let mut cursor = from;
    while cursor < source.len() {
        if is_line_start(source, cursor)
            && (marginalia_slash_marker(source, cursor).is_some()
                || marginalia_fence_marker(source, cursor).is_some())
        {
            return cursor;
        }
        let line_end = find_line_end(source, cursor);
        cursor = next_line_start(source, line_end);
    }
    source.len()
}

fn find_intralinea_close(source: &str, from: usize) -> Option<usize> {
    let mut cursor = from;
    let mut capture_depth = 0usize;
    let mut quote: Option<&'static str> = None;
    let mut line_comment = false;
    let mut block_comment = false;

    while cursor < source.len() {
        let tail = &source[cursor..];
        if line_comment {
            if tail.starts_with('\n') {
                line_comment = false;
            }
            cursor += tail.chars().next()?.len_utf8();
            continue;
        }
        if block_comment {
            let block_end = tail.find("*/");
            if let Some(block_end) = block_end {
                block_comment = false;
                cursor += block_end + 2;
            } else if let Some(intralinea_close) = tail.find("}}") {
                return Some(cursor + intralinea_close);
            } else {
                cursor = source.len();
            }
            continue;
        }
        if let Some(delimiter) = quote {
            if delimiter == "\"" {
                match scan_same_line_quote_boundary(tail, '"', true) {
                    SameLineQuoteBoundary::LexicalClose(idx) => {
                        quote = None;
                        cursor += idx + 1;
                    }
                    SameLineQuoteBoundary::Newline(idx) => {
                        quote = None;
                        cursor += idx + 1;
                    }
                    SameLineQuoteBoundary::HostClose(idx) => return Some(cursor + idx),
                    SameLineQuoteBoundary::Eof => cursor = source.len(),
                }
            } else if delimiter == "\"\"\"" {
                if let Some(triple_end) = tail.find("\"\"\"") {
                    quote = None;
                    cursor += triple_end + 3;
                } else if let Some(intralinea_close) = tail.find("}}") {
                    return Some(cursor + intralinea_close);
                } else {
                    cursor = source.len();
                }
            } else if delimiter == "`" {
                match scan_same_line_quote_boundary(tail, '`', false) {
                    SameLineQuoteBoundary::LexicalClose(idx) => {
                        quote = None;
                        cursor += idx + 1;
                    }
                    SameLineQuoteBoundary::Newline(idx) => {
                        quote = None;
                        cursor += idx + 1;
                    }
                    SameLineQuoteBoundary::HostClose(idx) => return Some(cursor + idx),
                    SameLineQuoteBoundary::Eof => cursor = source.len(),
                }
            } else if tail.starts_with(delimiter) {
                quote = None;
                cursor += delimiter.len();
            } else {
                cursor += tail.chars().next()?.len_utf8();
            }
            continue;
        }

        if tail.starts_with("\"\"\"") {
            quote = Some("\"\"\"");
            cursor += 3;
        } else if tail.starts_with('"') {
            quote = Some("\"");
            cursor += 1;
        } else if tail.starts_with('`') {
            quote = Some("`");
            cursor += 1;
        } else if let Some(len) = intralinea_snippet_len(tail) {
            cursor += len;
        } else if let Some(len) = intralinea_uri_literal_len(tail) {
            cursor += len;
        } else if capture_depth > 0 && matches!(tail.chars().next(), Some('\n' | '\r')) {
            capture_depth = 0;
            cursor += tail
                .chars()
                .next()
                .expect("tail is non-empty while cursor < source.len()")
                .len_utf8();
        } else if capture_depth == 0 && tail.starts_with("//") {
            line_comment = true;
            cursor += 2;
        } else if capture_depth == 0 && tail.starts_with("/*") {
            block_comment = true;
            cursor += 2;
        } else if let Some(prefix) = intralinea_close_capture_prefix(tail, capture_depth) {
            if prefix == 0 {
                return Some(cursor);
            }
            capture_depth -= prefix;
            cursor += prefix;
        } else if tail.starts_with('{') {
            capture_depth += 1;
            cursor += 1;
        } else if tail.starts_with('}') && capture_depth > 0 {
            capture_depth -= 1;
            cursor += 1;
        } else {
            cursor += tail.chars().next()?.len_utf8();
        }
    }

    None
}

fn intralinea_snippet_len(source: &str) -> Option<usize> {
    if !source.starts_with('[') {
        return None;
    }

    let mut cursor = 1;
    let mut capture_depth = 0usize;
    let mut quoted = false;

    while cursor < source.len() {
        let tail = &source[cursor..];
        if !quoted {
            if matches!(tail.chars().next(), Some('\n' | '\r')) {
                return Some(cursor);
            }
            if let Some(prefix) = intralinea_close_capture_prefix(tail, capture_depth) {
                if prefix == 0 {
                    return Some(cursor);
                }
                capture_depth -= prefix;
                cursor += prefix;
                continue;
            }
        }

        if quoted {
            match scan_same_line_quote_boundary(tail, '"', true) {
                SameLineQuoteBoundary::LexicalClose(idx) => {
                    quoted = false;
                    cursor += idx + 1;
                }
                SameLineQuoteBoundary::Newline(idx) => {
                    quoted = false;
                    cursor += idx + 1;
                }
                SameLineQuoteBoundary::HostClose(idx) => return Some(cursor + idx),
                SameLineQuoteBoundary::Eof => return Some(source.len()),
            }
            continue;
        }

        let ch = tail.chars().next()?;
        match ch {
            '"' => quoted = true,
            '{' => capture_depth += 1,
            '}' if capture_depth > 0 => capture_depth -= 1,
            ']' if capture_depth == 0 => return Some(cursor + 1),
            _ => {}
        }
        cursor += ch.len_utf8();
    }

    Some(source.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SameLineQuoteBoundary {
    LexicalClose(usize),
    Newline(usize),
    HostClose(usize),
    Eof,
}

fn scan_same_line_quote_boundary(
    source: &str,
    lexical_close: char,
    supports_escapes: bool,
) -> SameLineQuoteBoundary {
    let mut cursor = 0;
    let mut recovery_close = None;

    while cursor < source.len() {
        let tail = &source[cursor..];
        if tail.starts_with("}}") {
            recovery_close.get_or_insert(cursor);
            cursor += 2;
            continue;
        }

        let ch = tail
            .chars()
            .next()
            .expect("tail is non-empty while cursor < source.len()");
        if matches!(ch, '\n' | '\r') {
            return recovery_close
                .map(SameLineQuoteBoundary::HostClose)
                .unwrap_or(SameLineQuoteBoundary::Newline(cursor));
        }
        if ch == lexical_close {
            return SameLineQuoteBoundary::LexicalClose(cursor);
        }
        if supports_escapes && ch == '\\' {
            cursor += ch.len_utf8();
            if source[cursor..].starts_with("}}") {
                return SameLineQuoteBoundary::HostClose(cursor);
            }
            if cursor < source.len() {
                let next = source[cursor..]
                    .chars()
                    .next()
                    .expect("cursor checked against source length");
                if !matches!(next, '\n' | '\r') {
                    cursor += next.len_utf8();
                }
            }
        } else {
            cursor += ch.len_utf8();
        }
    }

    recovery_close
        .map(SameLineQuoteBoundary::HostClose)
        .unwrap_or(SameLineQuoteBoundary::Eof)
}

fn intralinea_close_capture_prefix(source: &str, capture_depth: usize) -> Option<usize> {
    if !source.starts_with("}}") {
        return None;
    }

    let closing_run = source.chars().take_while(|ch| *ch == '}').count();
    Some(capture_depth.min(closing_run.saturating_sub(2)))
}

fn intralinea_uri_literal_len(source: &str) -> Option<usize> {
    if !source.starts_with('<') {
        return None;
    }

    let mut chars = source.char_indices();
    chars.next()?;
    let (_, first) = chars.next()?;
    if first.is_whitespace() || matches!(first, '<' | '>') {
        return None;
    }

    let mut recovery_close = None;
    for (idx, ch) in chars {
        if source[idx..].starts_with("}}") {
            recovery_close.get_or_insert(idx);
        }
        if ch == '>' {
            return Some(idx + 1);
        }
        if ch.is_whitespace() {
            return Some(recovery_close.unwrap_or(idx));
        }
    }

    Some(recovery_close.unwrap_or(source.len()))
}

fn leading_ws_len(text: &str) -> usize {
    text.char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(text.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalSubjectMarkerMatch {
    None,
    Valid(usize),
    Invalid(usize),
}

fn local_subject_marker_at(source: &str, pos: usize) -> LocalSubjectMarkerMatch {
    let tail = &source[pos..];
    let marker_start = usize::from(tail.starts_with('~'));
    let marker_tail = &tail[marker_start..];
    let angle_len = marker_tail
        .char_indices()
        .find_map(|(index, ch)| (!matches!(ch, '<' | '>')).then_some(index))
        .unwrap_or(marker_tail.len());

    if angle_len == 0 {
        return LocalSubjectMarkerMatch::None;
    }

    let marker_len = marker_start + angle_len;
    if !is_local_subject_marker_boundary(tail[marker_len..].chars().next()) {
        return LocalSubjectMarkerMatch::None;
    }

    match &marker_tail[..angle_len] {
        "<" | ">" | "<>" | "<<" | ">>" | "<<>>" => LocalSubjectMarkerMatch::Valid(marker_len),
        _ => LocalSubjectMarkerMatch::Invalid(marker_len),
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/')
}

fn snippet_contains_range(source: &str, intralinea_close_aware: bool) -> bool {
    let mut cursor = usize::from(source.starts_with('['));
    let mut quoted = false;
    let mut capture_depth = 0usize;

    while cursor < source.len() {
        let tail = &source[cursor..];
        if intralinea_close_aware && !quoted {
            if let Some(prefix) = intralinea_close_capture_prefix(tail, capture_depth) {
                if prefix == 0 {
                    return false;
                }
                capture_depth -= prefix;
                cursor += prefix;
                continue;
            }
        }

        let ch = tail.chars().next().expect("cursor stays within source");
        if matches!(ch, '\n' | '\r') {
            return false;
        }
        if quoted {
            if ch == '\\' {
                cursor += 1;
                if cursor < source.len() {
                    let next = source[cursor..]
                        .chars()
                        .next()
                        .expect("cursor stays within source");
                    if !matches!(next, '\n' | '\r') {
                        cursor += next.len_utf8();
                    }
                }
                continue;
            }
            if ch == '"' {
                quoted = false;
            }
        } else {
            match ch {
                '"' if capture_depth == 0 => quoted = true,
                '{' => capture_depth += 1,
                '}' if capture_depth > 0 => capture_depth -= 1,
                ']' | '\n' | '\r' if capture_depth == 0 => return false,
                '.' if capture_depth == 0 && tail.starts_with("..") => return true,
                _ => {}
            }
        }
        cursor += ch.len_utf8();
    }

    false
}

fn is_local_subject_marker_boundary(next: Option<char>) -> bool {
    next.is_none_or(|ch| ch.is_whitespace() || matches!(ch, '.' | ',' | ';'))
}
