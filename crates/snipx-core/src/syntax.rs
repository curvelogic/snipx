use std::fmt;

use rowan::{Language, SyntaxKind as RowanSyntaxKind};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    Root,
    Directive,
    TargetDirective,
    ProfileDirective,
    Statement,
    Subject,
    Predicate,
    Object,
    ObjectList,
    Decoration,
    Snippet,
    RangeSnippet,
    QuotedSnippetPart,
    Capture,
    Quantifier,
    Uri,
    String,
    TripleString,
    Number,
    Boolean,
    BacktickPredicate,
    LineComment,
    BlockComment,
    MarginaliaText,
    Fence,
    FenceInfo,
    FenceBody,
    IntralineaText,
    IntralineaBlock,
    LocalSubjectMarker,
    Identifier,
    Whitespace,
    Error,
    Text,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
    LAngle,
    RAngle,
    ColonColon,
    At,
    Tilde,
    Quote,
    Backtick,
    SlashSlashSlash,
    Semicolon,
    Comma,
    Dot,
}

impl fmt::Debug for SyntaxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Root => "ROOT",
            Self::Directive => "DIRECTIVE",
            Self::TargetDirective => "TARGET_DIRECTIVE",
            Self::ProfileDirective => "PROFILE_DIRECTIVE",
            Self::Statement => "STATEMENT",
            Self::Subject => "SUBJECT",
            Self::Predicate => "PREDICATE",
            Self::Object => "OBJECT",
            Self::ObjectList => "OBJECT_LIST",
            Self::Decoration => "DECORATION",
            Self::Snippet => "SNIPPET",
            Self::RangeSnippet => "RANGE_SNIPPET",
            Self::QuotedSnippetPart => "QUOTED_SNIPPET_PART",
            Self::Capture => "CAPTURE",
            Self::Quantifier => "QUANTIFIER",
            Self::Uri => "URI",
            Self::String => "STRING",
            Self::TripleString => "TRIPLE_STRING",
            Self::Number => "NUMBER",
            Self::Boolean => "BOOLEAN",
            Self::BacktickPredicate => "BACKTICK_PREDICATE",
            Self::LineComment => "LINE_COMMENT",
            Self::BlockComment => "BLOCK_COMMENT",
            Self::MarginaliaText => "MARGINALIA_TEXT",
            Self::Fence => "FENCE",
            Self::FenceInfo => "FENCE_INFO",
            Self::FenceBody => "FENCE_BODY",
            Self::IntralineaText => "INTRALINEA_TEXT",
            Self::IntralineaBlock => "INTRALINEA_BLOCK",
            Self::LocalSubjectMarker => "LOCAL_SUBJECT_MARKER",
            Self::Identifier => "IDENT",
            Self::Whitespace => "WHITESPACE",
            Self::Error => "ERROR",
            Self::Text => "TEXT",
            Self::LBrack => "L_BRACK",
            Self::RBrack => "R_BRACK",
            Self::LBrace => "L_BRACE",
            Self::RBrace => "R_BRACE",
            Self::LAngle => "L_ANGLE",
            Self::RAngle => "R_ANGLE",
            Self::ColonColon => "COLON_COLON",
            Self::At => "AT",
            Self::Tilde => "TILDE",
            Self::Quote => "QUOTE",
            Self::Backtick => "BACKTICK",
            Self::SlashSlashSlash => "SLASH_SLASH_SLASH",
            Self::Semicolon => "SEMICOLON",
            Self::Comma => "COMMA",
            Self::Dot => "DOT",
        })
    }
}

impl From<SyntaxKind> for RowanSyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SnipxLanguage {}

impl Language for SnipxLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: RowanSyntaxKind) -> Self::Kind {
        match raw.0 {
            0 => SyntaxKind::Root,
            1 => SyntaxKind::Directive,
            2 => SyntaxKind::TargetDirective,
            3 => SyntaxKind::ProfileDirective,
            4 => SyntaxKind::Statement,
            5 => SyntaxKind::Subject,
            6 => SyntaxKind::Predicate,
            7 => SyntaxKind::Object,
            8 => SyntaxKind::ObjectList,
            9 => SyntaxKind::Decoration,
            10 => SyntaxKind::Snippet,
            11 => SyntaxKind::RangeSnippet,
            12 => SyntaxKind::QuotedSnippetPart,
            13 => SyntaxKind::Capture,
            14 => SyntaxKind::Quantifier,
            15 => SyntaxKind::Uri,
            16 => SyntaxKind::String,
            17 => SyntaxKind::TripleString,
            18 => SyntaxKind::Number,
            19 => SyntaxKind::Boolean,
            20 => SyntaxKind::BacktickPredicate,
            21 => SyntaxKind::LineComment,
            22 => SyntaxKind::BlockComment,
            23 => SyntaxKind::MarginaliaText,
            24 => SyntaxKind::Fence,
            25 => SyntaxKind::FenceInfo,
            26 => SyntaxKind::FenceBody,
            27 => SyntaxKind::IntralineaText,
            28 => SyntaxKind::IntralineaBlock,
            29 => SyntaxKind::LocalSubjectMarker,
            30 => SyntaxKind::Identifier,
            31 => SyntaxKind::Whitespace,
            32 => SyntaxKind::Error,
            33 => SyntaxKind::Text,
            34 => SyntaxKind::LBrack,
            35 => SyntaxKind::RBrack,
            36 => SyntaxKind::LBrace,
            37 => SyntaxKind::RBrace,
            38 => SyntaxKind::LAngle,
            39 => SyntaxKind::RAngle,
            40 => SyntaxKind::ColonColon,
            41 => SyntaxKind::At,
            42 => SyntaxKind::Tilde,
            43 => SyntaxKind::Quote,
            44 => SyntaxKind::Backtick,
            45 => SyntaxKind::SlashSlashSlash,
            46 => SyntaxKind::Semicolon,
            47 => SyntaxKind::Comma,
            48 => SyntaxKind::Dot,
            _ => SyntaxKind::Error,
        }
    }

    fn kind_to_raw(kind: Self::Kind) -> RowanSyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<SnipxLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<SnipxLanguage>;
