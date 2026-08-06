use std::fmt;

use rowan::{Language, SyntaxKind as RowanSyntaxKind};

/// Defines `SyntaxKind` together with its `Debug` names and the raw-value
/// conversion table in a single place.
///
/// Because the enum is `repr(u16)` with no explicit discriminants, each
/// variant's raw value is its declaration index, and `ALL` is built in the
/// same declaration order. `kind_from_raw` therefore indexes `ALL` and can
/// never drift from the enum, no matter how variants are added, removed, or
/// reordered.
macro_rules! syntax_kinds {
    ($($variant:ident => $debug_name:literal,)+) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        pub enum SyntaxKind {
            $($variant,)+
        }

        impl SyntaxKind {
            /// Every variant, indexed by its raw `u16` value.
            const ALL: &'static [SyntaxKind] = &[$(SyntaxKind::$variant,)+];

            fn from_raw(raw: u16) -> SyntaxKind {
                SyntaxKind::ALL
                    .get(usize::from(raw))
                    .copied()
                    .unwrap_or(SyntaxKind::Error)
            }
        }

        impl fmt::Debug for SyntaxKind {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(match self {
                    $(Self::$variant => $debug_name,)+
                })
            }
        }
    };
}

syntax_kinds! {
    Root => "ROOT",
    Directive => "DIRECTIVE",
    TargetDirective => "TARGET_DIRECTIVE",
    ProfileDirective => "PROFILE_DIRECTIVE",
    Statement => "STATEMENT",
    Subject => "SUBJECT",
    Predicate => "PREDICATE",
    Object => "OBJECT",
    ObjectList => "OBJECT_LIST",
    Decoration => "DECORATION",
    Snippet => "SNIPPET",
    RangeSnippet => "RANGE_SNIPPET",
    QuotedSnippetPart => "QUOTED_SNIPPET_PART",
    Capture => "CAPTURE",
    Quantifier => "QUANTIFIER",
    Uri => "URI",
    String => "STRING",
    TripleString => "TRIPLE_STRING",
    Number => "NUMBER",
    Boolean => "BOOLEAN",
    BacktickPredicate => "BACKTICK_PREDICATE",
    LineComment => "LINE_COMMENT",
    BlockComment => "BLOCK_COMMENT",
    MarginaliaText => "MARGINALIA_TEXT",
    Fence => "FENCE",
    FenceInfo => "FENCE_INFO",
    FenceBody => "FENCE_BODY",
    IntralineaText => "INTRALINEA_TEXT",
    IntralineaBlock => "INTRALINEA_BLOCK",
    LocalSubjectMarker => "LOCAL_SUBJECT_MARKER",
    Identifier => "IDENT",
    Whitespace => "WHITESPACE",
    Error => "ERROR",
    Text => "TEXT",
    LBrack => "L_BRACK",
    RBrack => "R_BRACK",
    LBrace => "L_BRACE",
    RBrace => "R_BRACE",
    LAngle => "L_ANGLE",
    RAngle => "R_ANGLE",
    ColonColon => "COLON_COLON",
    At => "AT",
    Tilde => "TILDE",
    Quote => "QUOTE",
    Backtick => "BACKTICK",
    SlashSlashSlash => "SLASH_SLASH_SLASH",
    Semicolon => "SEMICOLON",
    Comma => "COMMA",
    Dot => "DOT",
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
        SyntaxKind::from_raw(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> RowanSyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<SnipxLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<SnipxLanguage>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_round_trips_through_raw() {
        for (index, &kind) in SyntaxKind::ALL.iter().enumerate() {
            let raw = SnipxLanguage::kind_to_raw(kind);
            assert_eq!(
                usize::from(raw.0),
                index,
                "raw value of {kind:?} must equal its declaration index"
            );
            assert_eq!(
                SnipxLanguage::kind_from_raw(raw),
                kind,
                "{kind:?} must survive a raw round-trip"
            );
        }
    }

    #[test]
    fn out_of_range_raw_maps_to_error() {
        let raw = RowanSyntaxKind(SyntaxKind::ALL.len() as u16);
        assert_eq!(SnipxLanguage::kind_from_raw(raw), SyntaxKind::Error);
        assert_eq!(
            SnipxLanguage::kind_from_raw(RowanSyntaxKind(u16::MAX)),
            SyntaxKind::Error
        );
    }
}
