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
    let region = parse_snipx_region(source, 0);
    build_parse(region)
}

fn parse_marginalia(source: &str) -> Parse {
    let mut events = vec![Event::Start(SyntaxKind::Root)];
    let mut diagnostics = Vec::new();
    let mut cursor = 0;

    while cursor < source.len() {
        if is_line_start(source, cursor) && source[cursor..].starts_with("///") {
            let line_end = find_line_end(source, cursor);
            let line = &source[cursor..line_end];
            events.push(Event::Start(SyntaxKind::LineComment));
            events.push(Event::Token(SyntaxKind::SlashSlashSlash, "///".to_string()));

            let tail = &line[3..];
            let whitespace_len = leading_ws_len(tail);
            let (whitespace, remainder) = tail.split_at(whitespace_len);
            if !whitespace.is_empty() {
                events.push(Event::Token(SyntaxKind::Whitespace, whitespace.to_string()));
            }
            if !remainder.is_empty() {
                let region_offset = line_end - remainder.len();
                let region = parse_snipx_region(remainder, region_offset);
                replay_without_root(&region.events, &mut events);
                diagnostics.extend(region.diagnostics);
            }

            events.push(Event::Finish);
            if line_end < source.len() {
                events.push(Event::Token(SyntaxKind::Whitespace, "\n".to_string()));
            }
            cursor = next_line_start(source, line_end);
            continue;
        }

        if is_line_start(source, cursor) && source[cursor..].starts_with("```") {
            let opening_end = find_line_end(source, cursor);
            let info = source[cursor + 3..opening_end].trim();
            let body_start = next_line_start(source, opening_end);
            let closing_start = find_closing_fence(source, body_start);

            events.push(Event::Start(SyntaxKind::Fence));
            events.push(Event::Token(SyntaxKind::Backtick, "```".to_string()));
            if !info.is_empty() {
                events.push(Event::Token(SyntaxKind::FenceInfo, info.to_string()));
            }
            if opening_end < source.len() {
                events.push(Event::Token(SyntaxKind::Whitespace, "\n".to_string()));
            }

            let body_end = closing_start.unwrap_or(source.len());
            let body = &source[body_start..body_end];
            if info.is_empty() || info == "snipx" {
                let region = parse_snipx_region(body, body_start);
                replay_without_root(&region.events, &mut events);
                diagnostics.extend(region.diagnostics);
            } else if !body.is_empty() {
                events.push(Event::Token(SyntaxKind::FenceBody, body.to_string()));
            }

            if let Some(closing_start) = closing_start {
                if body_end > body_start && !body.ends_with('\n') {
                    events.push(Event::Token(SyntaxKind::Whitespace, "\n".to_string()));
                }
                events.push(Event::Token(SyntaxKind::Backtick, "```".to_string()));
                let closing_end = find_line_end(source, closing_start);
                if closing_end < source.len() {
                    events.push(Event::Token(SyntaxKind::Whitespace, "\n".to_string()));
                }
                cursor = next_line_start(source, closing_end);
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

            if let Some(end_rel) = source[start + 2..].find("}}") {
                let body_start = start + 2;
                let body_end = body_start + end_rel;
                let body = &source[body_start..body_end];
                let region = parse_snipx_region(body, body_start);
                replay_without_root(&region.events, &mut events);
                diagnostics.extend(region.diagnostics);
                events.push(Event::Token(SyntaxKind::RBrace, "}".to_string()));
                events.push(Event::Token(SyntaxKind::RBrace, "}".to_string()));
                events.push(Event::Finish);
                cursor = body_end + 2;
            } else {
                let body_start = start + 2;
                let body = &source[body_start..];
                let region = parse_snipx_region(body, body_start);
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

fn parse_snipx_region(source: &str, offset: usize) -> RegionParse {
    let mut parser = RegionParser::new(source, offset);
    parser.parse();
    RegionParse {
        events: parser.events,
        diagnostics: parser.diagnostics,
    }
}

struct RegionParser<'a> {
    source: &'a str,
    offset: usize,
    pos: usize,
    events: Vec<Event>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> RegionParser<'a> {
    fn new(source: &'a str, offset: usize) -> Self {
        Self {
            source,
            offset,
            pos: 0,
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
            } else if self.peek_char() == Some('~') {
                self.parse_decoration();
            } else {
                self.parse_statement();
            }
        }
        self.events.push(Event::Finish);
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
        let kind = if !is_line_start(self.source, self.pos) {
            self.push_diagnostic(
                DiagnosticCode::InvalidDirectivePosition,
                "Directives must start at the beginning of a line",
                start,
                start + 1,
            );
            SyntaxKind::Directive
        } else if self.source[self.pos..].starts_with("@target") {
            SyntaxKind::TargetDirective
        } else if self.source[self.pos..].starts_with("@profile") {
            SyntaxKind::ProfileDirective
        } else {
            SyntaxKind::Directive
        };

        let end = find_line_end(self.source, self.pos);
        self.events.push(Event::Start(kind));
        self.token(SyntaxKind::At, "@");
        self.pos += 1;
        self.consume_identifier_like();
        self.token_from(SyntaxKind::Identifier, start + 1, self.pos);
        if self.pos < end {
            self.token_from(
                SyntaxKind::Whitespace,
                self.pos,
                self.pos + leading_ws_len(&self.source[self.pos..end]),
            );
            let ws_end = self.pos + leading_ws_len(&self.source[self.pos..end]);
            self.pos = ws_end;
            if self.pos < end {
                self.parse_inline_value_until(end);
            }
        }
        self.events.push(Event::Finish);
        self.pos = end;
    }

    fn parse_decoration(&mut self) {
        self.events.push(Event::Start(SyntaxKind::Decoration));
        self.token(SyntaxKind::Tilde, "~");
        self.pos += 1;
        self.parse_statement_tail();
        self.events.push(Event::Finish);
    }

    fn parse_statement(&mut self) {
        self.events.push(Event::Start(SyntaxKind::Statement));
        self.events.push(Event::Start(SyntaxKind::Subject));
        if !self.parse_subject_like() {
            self.parse_value();
        }
        self.events.push(Event::Finish);

        self.consume_inline_whitespace();

        if !self.at_statement_end() {
            self.events.push(Event::Start(SyntaxKind::Predicate));
            self.parse_predicate_like();
            self.events.push(Event::Finish);
        }

        self.consume_inline_whitespace();

        if !self.at_statement_end() {
            self.events.push(Event::Start(SyntaxKind::ObjectList));
            while !self.at_statement_end() {
                self.events.push(Event::Start(SyntaxKind::Object));
                self.parse_object_like();
                self.events.push(Event::Finish);
                self.consume_inline_whitespace();
                if self.peek_char() == Some(',') {
                    self.token(SyntaxKind::Comma, ",");
                    self.pos += 1;
                    self.consume_inline_whitespace();
                } else if self.starts_with("::") {
                    self.token(SyntaxKind::ColonColon, "::");
                    self.pos += 2;
                    self.consume_inline_whitespace();
                } else if self.peek_char() == Some(';') {
                    self.token(SyntaxKind::Semicolon, ";");
                    self.pos += 1;
                    self.consume_inline_whitespace();
                } else {
                    break;
                }
            }
            self.events.push(Event::Finish);
        }

        if self.peek_char() == Some('.') {
            self.token(SyntaxKind::Dot, ".");
            self.pos += 1;
        }

        self.events.push(Event::Finish);
    }

    fn parse_statement_tail(&mut self) {
        self.consume_inline_whitespace();
        while !self.at_statement_end() {
            self.parse_value();
            self.consume_inline_whitespace();
            if self.peek_char() == Some(',') {
                self.token(SyntaxKind::Comma, ",");
                self.pos += 1;
                self.consume_inline_whitespace();
            } else if self.peek_char() == Some(';') {
                self.token(SyntaxKind::Semicolon, ";");
                self.pos += 1;
                self.consume_inline_whitespace();
            } else if self.starts_with("::") {
                self.token(SyntaxKind::ColonColon, "::");
                self.pos += 2;
                self.consume_inline_whitespace();
            } else if self.peek_char() == Some('.') || self.at_statement_end() {
                break;
            }
        }

        if self.peek_char() == Some('.') {
            self.token(SyntaxKind::Dot, ".");
            self.pos += 1;
        }
    }

    fn parse_subject_like(&mut self) -> bool {
        if self.peek_char() == Some('<')
            && self
                .peek_next_char()
                .is_some_and(|ch| ch == '>' || ch.is_whitespace())
        {
            self.parse_local_subject_marker();
            self.consume_inline_whitespace();
            true
        } else {
            self.parse_value()
        }
    }

    fn parse_predicate_like(&mut self) {
        if self.peek_char() == Some('`') {
            self.events
                .push(Event::Start(SyntaxKind::BacktickPredicate));
            self.parse_backtick_chunk();
            self.events.push(Event::Finish);
        } else {
            self.parse_value();
        }
    }

    fn parse_object_like(&mut self) {
        while !self.at_statement_end() {
            if matches!(self.peek_char(), Some(',') | Some(';') | Some('.'))
                || self.starts_with("::")
            {
                break;
            }
            if !self.parse_value() {
                break;
            }
            self.consume_inline_whitespace();
        }
    }

    fn parse_inline_value_until(&mut self, end: usize) {
        while self.pos < end {
            if self.peek_char().is_some_and(char::is_whitespace) {
                let start = self.pos;
                self.consume_while(|ch| ch.is_whitespace());
                self.token_from(SyntaxKind::Whitespace, start, self.pos);
            } else if !self.parse_value() {
                break;
            }
        }
    }

    fn parse_value(&mut self) -> bool {
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
                if self
                    .peek_next_char()
                    .is_some_and(|ch| ch == '>' || ch.is_whitespace())
                {
                    self.parse_local_subject_marker();
                } else {
                    self.parse_uri();
                }
                true
            }
            Some('`') => {
                self.parse_backtick_chunk();
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

    fn parse_snippet(&mut self) {
        let start = self.pos;
        let kind = if snippet_contains_range(&self.source[self.pos..]) {
            SyntaxKind::RangeSnippet
        } else {
            SyntaxKind::Snippet
        };
        self.events.push(Event::Start(kind));
        self.token(SyntaxKind::LBrack, "[");
        self.pos += 1;
        let mut text_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == ']' {
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
                self.parse_capture();
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
            if ch == '}' {
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
            if ch == '"' {
                break;
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
        self.events
            .push(Event::Start(SyntaxKind::LocalSubjectMarker));
        self.token(SyntaxKind::LAngle, "<");
        self.pos += 1;
        if self.peek_char() == Some('>') {
            self.token(SyntaxKind::RAngle, ">");
            self.pos += 1;
        }
        self.events.push(Event::Finish);
    }

    fn parse_backtick_chunk(&mut self) {
        let start = self.pos;
        self.token(SyntaxKind::Backtick, "`");
        self.pos += 1;
        let content_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == '`' {
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
        self.consume_while(|ch| ch.is_ascii_digit() || ch == '.');
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

    fn at_statement_end(&self) -> bool {
        matches!(self.peek_char(), None | Some('\n'))
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

    fn peek_next_char(&self) -> Option<char> {
        let mut chars = self.source[self.pos..].chars();
        chars.next()?;
        chars.next()
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
            Event::Start(SyntaxKind::Root) if depth == 0 => depth += 1,
            Event::Finish if depth == 1 => depth -= 1,
            _ => out.push(event.clone()),
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

fn next_line_start(source: &str, line_end: usize) -> usize {
    if line_end < source.len() {
        line_end + 1
    } else {
        line_end
    }
}

fn is_line_start(source: &str, pos: usize) -> bool {
    pos == 0 || source[..pos].ends_with('\n')
}

fn find_closing_fence(source: &str, from: usize) -> Option<usize> {
    let mut cursor = from;
    while cursor < source.len() {
        if is_line_start(source, cursor) && source[cursor..].starts_with("```") {
            return Some(cursor);
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
            && (source[cursor..].starts_with("///") || source[cursor..].starts_with("```"))
        {
            return cursor;
        }
        let line_end = find_line_end(source, cursor);
        cursor = next_line_start(source, line_end);
    }
    source.len()
}

fn leading_ws_len(text: &str) -> usize {
    text.char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(text.len())
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/')
}

fn snippet_contains_range(source: &str) -> bool {
    let mut chars = source.char_indices().peekable();
    let mut quoted = false;
    let mut captured = false;

    while let Some((_, ch)) = chars.next() {
        match ch {
            '"' if !captured => quoted = !quoted,
            '{' if !quoted => captured = true,
            '}' if !quoted => captured = false,
            ']' if !quoted && !captured => return false,
            '.' if !quoted && !captured && chars.peek().is_some_and(|(_, next)| *next == '.') => {
                return true;
            }
            _ => {}
        }
    }

    false
}
