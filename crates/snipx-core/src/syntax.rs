use std::fmt;

use rowan::{Language, SyntaxKind as RowanSyntaxKind};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    Root,
    Statement,
    Snippet,
    Identifier,
    Whitespace,
    Error,
    LBrack,
    RBrack,
    Dot,
    Text,
}

impl fmt::Debug for SyntaxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Root => "ROOT",
            Self::Statement => "STATEMENT",
            Self::Snippet => "SNIPPET",
            Self::Identifier => "IDENT",
            Self::Whitespace => "WHITESPACE",
            Self::Error => "ERROR",
            Self::LBrack => "L_BRACK",
            Self::RBrack => "R_BRACK",
            Self::Dot => "DOT",
            Self::Text => "TEXT",
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
            1 => SyntaxKind::Statement,
            2 => SyntaxKind::Snippet,
            3 => SyntaxKind::Identifier,
            4 => SyntaxKind::Whitespace,
            5 => SyntaxKind::Error,
            6 => SyntaxKind::LBrack,
            7 => SyntaxKind::RBrack,
            8 => SyntaxKind::Dot,
            9 => SyntaxKind::Text,
            _ => SyntaxKind::Error,
        }
    }

    fn kind_to_raw(kind: Self::Kind) -> RowanSyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<SnipxLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<SnipxLanguage>;
