export { buildIndexMap, isWhitespace, utf8LengthOfCodePoint, type IndexMap } from "./indexMaps.js";
export type {
  Diagnostic,
  DiagnosticCode,
  RelatedSpan,
  Severity,
  SourceSpan,
} from "./diagnostic.js";
export { parse, type InputForm, type Parse } from "./parser.js";
export { SyntaxNode, type SyntaxKind, type SyntaxToken } from "./syntax.js";
export {
  expand,
  type ExpandOptions,
  type ExpandResult,
  type ExpandedStatement,
  type LocalRegion,
  type LocalScope,
  type LocalSubject,
  type Value,
} from "./expand.js";
export {
  matchSnippet,
  normalize,
  type MatchResult,
  type TextSpan,
} from "./match.js";
export {
  resolve,
  type IntralineaAnchor,
  type ResolveOptions,
  type ResolveResult,
  type SnippetResolution,
} from "./resolve.js";
export {
  extractVisibleText,
  profileFromName,
  type Profile,
  type VisibleText,
} from "./visibleText.js";
export type { Cardinality, SnippetPart, SnippetValue } from "./snippet.js";
export {
  exportJson,
  IMPLEMENTATION_VERSION,
  SPEC_VERSION,
  type ExportDocument,
  type ExportRequest,
  type JsonDiagnostic,
  type JsonFact,
  type JsonFactSource,
  type JsonImplementation,
  type JsonInput,
  type JsonRelatedSpan,
  type JsonResolution,
  type JsonSpan,
  type JsonTarget,
  type JsonValue,
  type JsonVisibleText,
} from "./export.js";
