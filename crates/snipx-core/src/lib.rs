pub mod ast;
pub mod diagnostic;
pub mod expand;
pub mod format;
pub mod input;
pub mod json;
pub mod r#match;
pub mod resolve;
pub mod snippet;
pub mod syntax;
pub mod visible_text;

mod parser;

pub use diagnostic::{Diagnostic, DiagnosticCode, RelatedSpan, Severity, SourceSpan};
pub use expand::{
    expand, ExpandOptions, ExpandResult, ExpandedStatement, LocalRegion, LocalScope, LocalSubject,
    Value,
};
pub use format::{format, FormatOptions, FormatResult};
pub use input::{InputForm, ParseOptions};
pub use json::{
    export_json, ExportDocument, ExportRequest, JsonDiagnostic, JsonFact, JsonFactSource,
    JsonImplementation, JsonInput, JsonRelatedSpan, JsonResolution, JsonSpan, JsonTarget,
    JsonValue, JsonVisibleText, SPEC_VERSION,
};
pub use parser::{parse, Parse};
pub use r#match::{match_snippet, TextSpan};
pub use resolve::{resolve, IntralineaAnchor, ResolveOptions, ResolveResult, SnippetResolution};
pub use snippet::{Cardinality, SnippetPart, SnippetValue};
pub use syntax::{SnipxLanguage, SyntaxKind, SyntaxNode, SyntaxToken};
pub use visible_text::{extract_visible_text, Profile, VisibleText};
