pub mod diagnostic;
pub mod input;
pub mod syntax;

mod parser;

pub use diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
pub use input::{InputForm, ParseOptions};
pub use parser::{parse, Parse};
pub use syntax::{SnipxLanguage, SyntaxKind, SyntaxNode, SyntaxToken};
