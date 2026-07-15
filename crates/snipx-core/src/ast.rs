use rowan::NodeOrToken;

use crate::syntax::{SyntaxKind, SyntaxNode};

pub trait AstNode: Sized {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(node: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;

    fn text(&self) -> String {
        self.syntax().to_string()
    }
}

macro_rules! ast_node {
    ($name:ident, $($kind:pat_param)|+ $(,)?) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            syntax: SyntaxNode,
        }

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                matches!(kind, $($kind)|+)
            }

            fn cast(node: SyntaxNode) -> Option<Self> {
                Self::can_cast(node.kind()).then_some(Self { syntax: node })
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.syntax
            }
        }
    };
}

ast_node!(Root, SyntaxKind::Root);
ast_node!(
    SnipxRegion,
    SyntaxKind::Root | SyntaxKind::LineComment | SyntaxKind::Fence | SyntaxKind::IntralineaBlock
);
ast_node!(Statement, SyntaxKind::Statement);
ast_node!(Subject, SyntaxKind::Subject);
ast_node!(Predicate, SyntaxKind::Predicate);
ast_node!(Object, SyntaxKind::Object);
ast_node!(ObjectList, SyntaxKind::ObjectList);
ast_node!(Snippet, SyntaxKind::Snippet);
ast_node!(RangeSnippet, SyntaxKind::RangeSnippet);
ast_node!(Capture, SyntaxKind::Capture);
ast_node!(
    Directive,
    SyntaxKind::Directive | SyntaxKind::TargetDirective | SyntaxKind::ProfileDirective
);
ast_node!(Decoration, SyntaxKind::Decoration);
ast_node!(IntralineaBlock, SyntaxKind::IntralineaBlock);

impl Root {
    pub fn statements(&self) -> impl Iterator<Item = Statement> + '_ {
        self.syntax.descendants().filter_map(Statement::cast)
    }

    pub fn regions(&self) -> Vec<SnipxRegion> {
        let child_regions: Vec<_> = self
            .syntax
            .children()
            .filter_map(SnipxRegion::cast)
            .filter(SnipxRegion::contains_snipx_items)
            .collect();

        if !child_regions.is_empty() {
            child_regions
        } else if SnipxRegion::contains_snipx_items_for(&self.syntax) {
            vec![SnipxRegion {
                syntax: self.syntax.clone(),
            }]
        } else {
            Vec::new()
        }
    }

    pub fn directives(&self) -> impl Iterator<Item = Directive> + '_ {
        self.syntax.descendants().filter_map(Directive::cast)
    }
}

impl SnipxRegion {
    pub fn statements(&self) -> impl Iterator<Item = Statement> + '_ {
        self.syntax.children().filter_map(Statement::cast)
    }

    pub fn directives(&self) -> impl Iterator<Item = Directive> + '_ {
        self.syntax.children().filter_map(Directive::cast)
    }

    pub fn intralinea_blocks(&self) -> impl Iterator<Item = IntralineaBlock> + '_ {
        self.syntax.children().filter_map(IntralineaBlock::cast)
    }

    fn contains_snipx_items(&self) -> bool {
        Self::contains_snipx_items_for(&self.syntax)
    }

    fn contains_snipx_items_for(node: &SyntaxNode) -> bool {
        node.children().any(|child| {
            matches!(
                child.kind(),
                SyntaxKind::Statement
                    | SyntaxKind::Directive
                    | SyntaxKind::TargetDirective
                    | SyntaxKind::ProfileDirective
                    | SyntaxKind::IntralineaBlock
            )
        })
    }
}

impl Statement {
    pub fn subject(&self) -> Option<Subject> {
        child(&self.syntax)
    }

    pub fn predicates(&self) -> impl Iterator<Item = Predicate> + '_ {
        self.syntax.children().filter_map(Predicate::cast)
    }

    pub fn predicate(&self) -> Option<Predicate> {
        self.predicates().next()
    }

    pub fn object_lists(&self) -> impl Iterator<Item = ObjectList> + '_ {
        self.syntax.children().filter_map(ObjectList::cast)
    }

    pub fn object_list(&self) -> Option<ObjectList> {
        self.object_lists().next()
    }

    pub fn decorations(&self) -> impl Iterator<Item = Decoration> + '_ {
        self.syntax.children().filter_map(Decoration::cast)
    }
}

impl Subject {
    pub fn snippets(&self) -> impl Iterator<Item = Snippet> + '_ {
        self.syntax.descendants().filter_map(Snippet::cast)
    }

    pub fn range_snippets(&self) -> impl Iterator<Item = RangeSnippet> + '_ {
        self.syntax.descendants().filter_map(RangeSnippet::cast)
    }
}

impl ObjectList {
    pub fn objects(&self) -> impl Iterator<Item = Object> + '_ {
        self.syntax.children().filter_map(Object::cast)
    }
}

impl Object {
    pub fn decorations(&self) -> impl Iterator<Item = Decoration> {
        let decorations = self
            .syntax
            .parent()
            .filter(|parent| parent.kind() == SyntaxKind::ObjectList)
            .map(|parent| {
                let mut seen_object = false;
                let mut decorations = Vec::new();

                for child in parent.children() {
                    if !seen_object {
                        seen_object = child == self.syntax;
                        continue;
                    }

                    if child.kind() == SyntaxKind::Object {
                        break;
                    }
                    if let Some(decoration) = Decoration::cast(child) {
                        decorations.push(decoration);
                    }
                }

                decorations
            })
            .unwrap_or_default();

        decorations.into_iter()
    }
}

impl Directive {
    pub fn name(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .find_map(|element| match element {
                NodeOrToken::Node(node) if node.kind() == SyntaxKind::Identifier => {
                    Some(node.to_string())
                }
                NodeOrToken::Token(token) if token.kind() == SyntaxKind::Identifier => {
                    Some(token.text().to_string())
                }
                _ => None,
            })
    }
}

impl Snippet {
    pub fn captures(&self) -> impl Iterator<Item = Capture> + '_ {
        self.syntax.descendants().filter_map(Capture::cast)
    }
}

fn child<N: AstNode>(node: &SyntaxNode) -> Option<N> {
    node.children().find_map(N::cast)
}
