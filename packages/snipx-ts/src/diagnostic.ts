/** Mirror of crates/snipx-core/src/diagnostic.rs. */

export interface SourceSpan {
  start: number;
  end: number;
}

export type Severity = "error" | "warning";

export type DiagnosticCode =
  | "ParseError"
  | "UnterminatedSnippet"
  | "UnterminatedString"
  | "UnterminatedBlockComment"
  | "UnterminatedIntralineaBlock"
  | "InvalidDirectivePosition"
  | "DuplicateDirective"
  | "InvalidLocalSubjectMarker"
  | "EmptyLocalSubject"
  | "InvalidCliUsage"
  | "MissingAmbientSubject"
  | "InvalidDecorationTarget"
  | "InvalidStatementTerminator"
  | "UnsupportedProfile"
  | "RawHtmlOmitted"
  | "InvalidSnippet"
  | "InvalidNumber"
  | "SnippetNotFound"
  | "SnippetAmbiguous";

export interface RelatedSpan {
  message: string;
  span: SourceSpan;
}

export interface Diagnostic {
  code: DiagnosticCode;
  severity: Severity;
  message: string;
  span: SourceSpan | null;
  related: RelatedSpan[];
}

export const DIAGNOSTIC_CODE_NAMES: Record<DiagnosticCode, string> = {
  ParseError: "PARSE_ERROR",
  UnterminatedSnippet: "UNTERMINATED_SNIPPET",
  UnterminatedString: "UNTERMINATED_STRING",
  UnterminatedBlockComment: "UNTERMINATED_BLOCK_COMMENT",
  UnterminatedIntralineaBlock: "UNTERMINATED_INTRALINEA_BLOCK",
  InvalidDirectivePosition: "INVALID_DIRECTIVE_POSITION",
  DuplicateDirective: "DUPLICATE_DIRECTIVE",
  InvalidLocalSubjectMarker: "INVALID_LOCAL_SUBJECT_MARKER",
  EmptyLocalSubject: "EMPTY_LOCAL_SUBJECT",
  InvalidCliUsage: "INVALID_CLI_USAGE",
  MissingAmbientSubject: "MISSING_AMBIENT_SUBJECT",
  InvalidDecorationTarget: "INVALID_DECORATION_TARGET",
  InvalidStatementTerminator: "INVALID_STATEMENT_TERMINATOR",
  UnsupportedProfile: "UNSUPPORTED_PROFILE",
  RawHtmlOmitted: "RAW_HTML_OMITTED",
  InvalidSnippet: "INVALID_SNIPPET",
  InvalidNumber: "INVALID_NUMBER",
  SnippetNotFound: "SNIPPET_NOT_FOUND",
  SnippetAmbiguous: "SNIPPET_AMBIGUOUS",
};
