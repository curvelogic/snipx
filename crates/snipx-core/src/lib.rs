pub mod ast;
pub mod diagnostic;
pub mod expand;
pub mod format;
pub mod input;
pub mod r#match;
pub mod resolve;
pub mod syntax;
pub mod visible_text;

mod parser;

pub use diagnostic::{Diagnostic, DiagnosticCode, RelatedSpan, Severity, SourceSpan};
pub use expand::{expand, ExpandOptions, ExpandResult, ExpandedStatement, Value};
pub use format::{format, FormatOptions, FormatResult};
pub use input::{InputForm, ParseOptions};
pub use parser::{parse, Parse};
pub use r#match::{match_snippet, TextSpan};
pub use resolve::{resolve, ResolveOptions, ResolveResult, SnippetResolution};
pub use syntax::{SnipxLanguage, SyntaxKind, SyntaxNode, SyntaxToken};
pub use visible_text::{extract_visible_text, Profile, VisibleText};
