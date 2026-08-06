/**
 * Mirror of crates/snipx-core/src/json.rs: `export_json` and the
 * canonical JSON document shape.
 *
 * This is the offset conversion boundary. Internally spans are UTF-16
 * code units (source and target texts) or Unicode scalars (visible
 * text). On the way out:
 *
 * - source spans (facts, resolutions sourceSpan, diagnostics) become
 *   UTF-8 byte offsets via the source index map;
 * - RAW_HTML_OMITTED spans become UTF-8 byte offsets via the target
 *   index map;
 * - visible-text spans stay Unicode scalars, as computed.
 */

import { headerDirectives, type HeaderDirectives } from "./ast.js";
import type { Diagnostic } from "./diagnostic.js";
import { DIAGNOSTIC_CODE_NAMES, type SourceSpan } from "./diagnostic.js";
import { expand, type ExpandedStatement, type Value } from "./expand.js";
import { buildIndexMap, type IndexMap } from "./indexMaps.js";
import type { TextSpan } from "./match.js";
import { parse, type InputForm, type Parse } from "./parser.js";
import { resolve, type IntralineaAnchor, type SnippetResolution } from "./resolve.js";
import { extractVisibleText, profileFromName, type Profile } from "./visibleText.js";

/**
 * The specification version this implementation targets, mirrored as
 * `snipxVersion` in canonical JSON output. Kept in step with the
 * version declared in docs/language-spec.md.
 */
export const SPEC_VERSION = "0.1";

/** Implementation version reported in the (non-normative) implementation block. */
export const IMPLEMENTATION_VERSION = "0.1.0";

export interface ExportRequest {
  source: string;
  inputForm: InputForm;
  targetText?: string | undefined;
  /**
   * Explicitly requested profile. When absent, a commentaria
   * `@profile` directive is honoured, falling back to plain.
   */
  profile?: Profile | undefined;
  path?: string | undefined;
  targetUri?: string | undefined;
  ambientSubject?: Value | undefined;
}

export interface JsonSpan {
  start: number;
  end: number;
}

export interface JsonImplementation {
  name: string;
  version: string;
}

export interface JsonInput {
  form: string;
  path?: string;
}

export interface JsonTarget {
  uri?: string;
  profile: string;
}

export interface JsonVisibleText {
  normalisation: string;
  length: number;
}

export type JsonValue =
  | { kind: "name"; value: string }
  | { kind: "predicate"; value: string }
  | { kind: "string"; value: string }
  | { kind: "number"; value: number }
  | { kind: "boolean"; value: boolean }
  | { kind: "uri"; value: string }
  | { kind: "snippet"; source: string }
  | { kind: "textSpanSnippet"; source: string; span?: JsonSpan }
  | { kind: "localSubject"; marker: string; scope: string; region: string }
  | { kind: "textSpanLocalSubject"; marker: string; scope: string; region: string }
  | { kind: "wholeDocument" }
  | { kind: "unresolvedSnippet"; source: string }
  | { kind: "unresolvedLocalSubject"; marker: string }
  | { kind: "unresolvedNumber"; source: string };

export interface JsonFactSource {
  statement: JsonSpan;
  subject?: JsonSpan;
  predicate?: JsonSpan;
  object?: JsonSpan;
}

export interface JsonFact {
  subject: JsonValue;
  predicate: JsonValue;
  object: JsonValue;
  source: JsonFactSource;
}

export interface JsonResolution {
  source: string;
  sourceSpan?: JsonSpan;
  spans: JsonSpan[];
}

export interface JsonRelatedSpan {
  message: string;
  span: JsonSpan;
}

export interface JsonDiagnostic {
  code: string;
  severity: string;
  message: string;
  span?: JsonSpan;
  related?: JsonRelatedSpan[];
}

export interface ExportDocument {
  snipxVersion: string;
  implementation: JsonImplementation;
  input: JsonInput;
  target?: JsonTarget;
  visibleText?: JsonVisibleText;
  facts: JsonFact[];
  resolutions: JsonResolution[];
  diagnostics: JsonDiagnostic[];
}

function scalarCount(text: string): number {
  let count = 0;
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  for (const _ of text) {
    count += 1;
  }
  return count;
}

export function exportJson(request: ExportRequest): ExportDocument {
  const parsed = parse(request.source, request.inputForm);
  const sourceMap = buildIndexMap(request.source);

  const header: HeaderDirectives =
    request.inputForm === "commentaria"
      ? headerDirectives(parsed.root)
      : { profile: null, target: null, duplicates: [] };

  const directiveDiagnostics: Diagnostic[] = header.duplicates.map(([name, span]) => ({
    code: "DuplicateDirective",
    severity: "warning",
    message: `Duplicate @${name} directive is ignored`,
    span,
    related: [],
  }));

  let profile: Profile;
  if (request.profile !== undefined) {
    profile = request.profile;
  } else if (header.profile !== null) {
    const named = profileFromName(header.profile.value);
    if (named !== null) {
      profile = named;
    } else {
      directiveDiagnostics.push({
        code: "UnsupportedProfile",
        severity: "error",
        message: `Unsupported profile: ${header.profile.value}`,
        span: header.profile.span,
        related: [],
      });
      profile = "plain";
    }
  } else {
    profile = "plain";
  }

  let implicitTarget: string | null = null;
  let intralineaAnchors: IntralineaAnchor[] = [];
  if (request.inputForm === "intralinea") {
    const [text, anchors] = intralineaVisibleSource(parsed);
    implicitTarget = text;
    // Anchors only apply when resolving against the stripped host text
    // itself; an explicit target may be entirely different.
    intralineaAnchors = request.targetText === undefined ? anchors : [];
  }

  const expanded = expand(parsed, { ambientSubject: request.ambientSubject ?? null });

  const targetText = request.targetText ?? implicitTarget ?? undefined;
  let visibleTextJson: JsonVisibleText | undefined;
  let statements: ExpandedStatement[];
  let resolutions: SnippetResolution[];
  let diagnostics: JsonDiagnostic[];

  if (targetText !== undefined) {
    const visible = extractVisibleText(targetText, profile);
    const targetMap = buildIndexMap(targetText);
    const extractionDiagnostics = visible.diagnostics.map((diagnostic) =>
      jsonDiagnostic(diagnostic, targetMap),
    );
    const resolved = resolve(expanded, visible, {
      profile,
      intralineaAnchors,
    });
    visibleTextJson = {
      normalisation: visible.normalisation,
      length: scalarCount(visible.text),
    };
    statements = resolved.statements;
    resolutions = resolved.resolutions;
    diagnostics = [
      ...resolved.diagnostics.map((diagnostic) => jsonDiagnostic(diagnostic, sourceMap)),
      ...extractionDiagnostics,
    ];
  } else {
    statements = expanded.statements;
    resolutions = [];
    diagnostics = expanded.diagnostics.map((diagnostic) => jsonDiagnostic(diagnostic, sourceMap));
  }

  if (statements.some(statementHasNonFiniteNumber)) {
    diagnostics.push({
      code: "INVALID_NUMBER",
      severity: "error",
      message: "JSON numbers must be finite",
    });
  }
  diagnostics.push(
    ...directiveDiagnostics.map((diagnostic) => jsonDiagnostic(diagnostic, sourceMap)),
  );

  const targetUri = request.targetUri ?? header.target?.value;
  const target: JsonTarget = { profile };
  if (targetUri !== undefined) {
    target.uri = targetUri;
  }

  const input: JsonInput = { form: request.inputForm };
  if (request.path !== undefined) {
    input.path = request.path;
  }

  const document: ExportDocument = {
    snipxVersion: SPEC_VERSION,
    implementation: {
      name: "snipx-ts",
      version: IMPLEMENTATION_VERSION,
    },
    input,
    target,
    facts: statements.map((statement) => jsonFact(statement, sourceMap)),
    resolutions: resolutions.map((resolution) => jsonResolution(resolution, sourceMap)),
    diagnostics,
  };
  if (visibleTextJson !== undefined) {
    document.visibleText = visibleTextJson;
  }
  return document;
}

function intralineaVisibleSource(parsed: Parse): [string, IntralineaAnchor[]] {
  let text = "";
  const anchors: IntralineaAnchor[] = [];
  for (const element of parsed.root.children) {
    if (element.type === "node" && element.kind === "IntralineaBlock") {
      anchors.push({
        blockSpan: { start: element.start, end: element.end },
        // The visible text is NFC-normalised before matching, so the
        // anchor counts scalars of the normalised prefix.
        visibleOffset: scalarCount(text.normalize("NFC")),
      });
    } else if (element.type === "node") {
      text += element.toText();
    } else {
      text += element.text;
    }
  }
  return [text, anchors];
}

function jsonFact(statement: ExpandedStatement, map: IndexMap): JsonFact {
  const source: JsonFactSource = {
    statement: jsonSourceSpan(statement.statementSpan, map),
  };
  if (statement.subjectSpan !== null) {
    source.subject = jsonSourceSpan(statement.subjectSpan, map);
  }
  if (statement.predicateSpan !== null) {
    source.predicate = jsonSourceSpan(statement.predicateSpan, map);
  }
  if (statement.objectSpan !== null) {
    source.object = jsonSourceSpan(statement.objectSpan, map);
  }
  return {
    subject: jsonValue(statement.subject),
    predicate: jsonValue(statement.predicate),
    object: jsonValue(statement.object),
    source,
  };
}

function jsonValue(value: Value): JsonValue {
  switch (value.type) {
    case "name":
      return { kind: "name", value: value.value };
    case "predicate":
      return { kind: "predicate", value: value.value };
    case "string":
      return { kind: "string", value: value.value };
    case "number":
      if (Number.isFinite(value.value)) {
        return { kind: "number", value: value.value };
      }
      return { kind: "unresolvedNumber", source: rustF64ToString(value.value) };
    case "invalidNumber":
      return { kind: "unresolvedNumber", source: value.source };
    case "boolean":
      return { kind: "boolean", value: value.value };
    case "uri":
      return { kind: "uri", value: value.value };
    case "snippet":
      return { kind: "snippet", source: value.snippet.source };
    case "textSpanSnippet":
      return { kind: "textSpanSnippet", source: value.snippet.source };
    case "resolvedTextSpan":
      return {
        kind: "textSpanSnippet",
        source: value.snippet.source,
        span: jsonTextSpan(value.span),
      };
    case "localSubject": {
      const kind = value.local.textSpan ? "textSpanLocalSubject" : "localSubject";
      return {
        kind,
        marker: value.local.marker,
        scope: value.local.scope,
        region: value.local.region,
      };
    }
    case "wholeDocument":
      return { kind: "wholeDocument" };
    case "unresolved":
      return { kind: "unresolvedSnippet", source: value.source };
    case "unresolvedLocalSubject":
      return { kind: "unresolvedLocalSubject", marker: value.marker };
  }
}

/** Rust's `f64::to_string` for the non-finite fallback. */
function rustF64ToString(value: number): string {
  if (Number.isNaN(value)) return "NaN";
  if (value === Infinity) return "inf";
  if (value === -Infinity) return "-inf";
  return String(value);
}

function statementHasNonFiniteNumber(statement: ExpandedStatement): boolean {
  return [statement.subject, statement.predicate, statement.object].some(
    (value) => value.type === "number" && !Number.isFinite(value.value),
  );
}

function jsonResolution(resolution: SnippetResolution, map: IndexMap): JsonResolution {
  const result: JsonResolution = {
    source: resolution.source,
    spans: resolution.spans.map(jsonTextSpan),
  };
  if (resolution.sourceSpan !== null) {
    result.sourceSpan = jsonSourceSpan(resolution.sourceSpan, map);
  }
  return result;
}

function jsonDiagnostic(diagnostic: Diagnostic, map: IndexMap): JsonDiagnostic {
  const result: JsonDiagnostic = {
    code: DIAGNOSTIC_CODE_NAMES[diagnostic.code],
    severity: diagnostic.severity,
    message: diagnostic.message,
  };
  if (diagnostic.span !== null) {
    result.span = jsonSourceSpan(diagnostic.span, map);
  }
  if (diagnostic.related.length > 0) {
    result.related = diagnostic.related.map((related) => ({
      message: related.message,
      span: jsonSourceSpan(related.span, map),
    }));
  }
  return result;
}

/** Convert a UTF-16 span to UTF-8 byte offsets. */
function jsonSourceSpan(span: SourceSpan, map: IndexMap): JsonSpan {
  return { start: map.utf16ToUtf8(span.start), end: map.utf16ToUtf8(span.end) };
}

/** Visible-text spans are already Unicode scalars. */
function jsonTextSpan(span: TextSpan): JsonSpan {
  return { start: span.start, end: span.end };
}
