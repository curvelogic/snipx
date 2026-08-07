/** Mirror of crates/snipx-core/src/resolve.rs. */

import type { Diagnostic, DiagnosticCode, SourceSpan } from "./diagnostic.js";
import type { ExpandResult, ExpandedStatement, LocalRegion, LocalScope, LocalSubject, Value } from "./expand.js";
import { matchSnippet, type TextSpan } from "./match.js";
import type { Profile, VisibleText } from "./visibleText.js";
import { isWhitespace } from "./indexMaps.js";

export interface ResolveOptions {
  profile: Profile | null;
  /**
   * Visible-text anchor for each intralinea block, used to resolve
   * local subject markers. `visibleOffset` counts Unicode scalar values
   * into the NFC visible text at the block's position; `blockSpan` is
   * in UTF-16 units of the source.
   */
  intralineaAnchors: IntralineaAnchor[];
}

export interface IntralineaAnchor {
  blockSpan: SourceSpan;
  visibleOffset: number;
}

export interface SnippetResolution {
  source: string;
  /** UTF-16 units of the snipx source. */
  sourceSpan: SourceSpan | null;
  /** Unicode scalar offsets into the visible text. */
  spans: TextSpan[];
}

export interface ResolveResult {
  statements: ExpandedStatement[];
  resolutions: SnippetResolution[];
  diagnostics: Diagnostic[];
}

export function resolve(
  expanded: ExpandResult,
  visibleText: VisibleText,
  options: ResolveOptions,
): ResolveResult {
  const profile = options.profile ?? visibleText.profile;
  const result: ResolveResult = {
    statements: [],
    resolutions: [],
    diagnostics: [...expanded.diagnostics],
  };

  for (const original of expanded.statements) {
    const statement: ExpandedStatement = { ...original };
    const subjectResolved = resolveValue(
      statement.subject,
      statement.subjectSpan,
      visibleText,
      profile,
      options.intralineaAnchors,
      result.resolutions,
      result.diagnostics,
    );
    statement.subject = subjectResolved.value;
    const objectResolved = resolveValue(
      statement.object,
      statement.objectSpan,
      visibleText,
      profile,
      options.intralineaAnchors,
      result.resolutions,
      result.diagnostics,
    );
    statement.object = objectResolved.value;
    distribute(statement, subjectResolved.spans, objectResolved.spans, result.statements);
  }

  return result;
}

/**
 * Spec (Denotation And Text Spans): text-span snippets distribute one
 * fact per matched span; both sides distributing yields the Cartesian
 * product. Denotational values pass through as a single alternative.
 */
function distribute(
  statement: ExpandedStatement,
  subjectSpans: TextSpan[] | null,
  objectSpans: TextSpan[] | null,
  statements: ExpandedStatement[],
): void {
  const subjects = valueAlternatives(statement.subject, subjectSpans);
  const objects = valueAlternatives(statement.object, objectSpans);
  for (const subject of subjects) {
    for (const object of objects) {
      statements.push({ ...statement, subject, object });
    }
  }
}

function valueAlternatives(value: Value, spans: TextSpan[] | null): Value[] {
  if (value.type === "textSpanSnippet" && spans !== null) {
    return spans.map((span) => ({
      type: "resolvedTextSpan",
      snippet: value.snippet,
      span,
    }));
  }
  return [value];
}

interface ResolvedValue {
  value: Value;
  spans: TextSpan[] | null;
}

function resolveValue(
  value: Value,
  sourceSpan: SourceSpan | null,
  visibleText: VisibleText,
  profile: Profile,
  anchors: IntralineaAnchor[],
  resolutions: SnippetResolution[],
  diagnostics: Diagnostic[],
): ResolvedValue {
  if (value.type === "localSubject") {
    const replaced = resolveLocalSubject(
      value.local,
      sourceSpan,
      visibleText,
      anchors,
      resolutions,
      diagnostics,
    );
    return { value: replaced ?? value, spans: null };
  }
  if (value.type !== "snippet" && value.type !== "textSpanSnippet") {
    return { value, spans: null };
  }
  const textSpan = value.type === "textSpanSnippet";
  const snippet = value.snippet;

  const matched = matchSnippet(snippet.parts, visibleText, profile);
  // An unterminated snippet with no more specific lexical defect keeps
  // the historical generic diagnostic.
  if (matched.ok && !snippet.terminated) {
    diagnostics.push(
      diagnostic("InvalidSnippet", `Invalid snippet syntax: ${snippet.source}`, sourceSpan),
    );
    return { value: { type: "unresolved", source: snippet.source }, spans: null };
  }
  if (!matched.ok) {
    const error = { ...matched.error };
    if (error.span === null) {
      error.span = sourceSpan;
    }
    diagnostics.push(error);
    return { value: { type: "unresolved", source: snippet.source }, spans: null };
  }
  const spans = matched.spans;

  let errorCode: DiagnosticCode | null = null;
  switch (snippet.cardinality) {
    case "exactlyOne":
      if (spans.length === 0) errorCode = "SnippetNotFound";
      else if (spans.length > 1) errorCode = "SnippetAmbiguous";
      break;
    case "oneOrMore":
      if (spans.length === 0) errorCode = "SnippetNotFound";
      break;
    case "zeroOrOne":
      if (spans.length > 1) errorCode = "SnippetAmbiguous";
      break;
    case "zeroOrMore":
      break;
  }
  if (errorCode !== null) {
    const message =
      errorCode === "SnippetNotFound"
        ? `Snippet did not match: ${snippet.source}`
        : `Snippet matched more than allowed: ${snippet.source}`;
    diagnostics.push(diagnostic(errorCode, message, sourceSpan));
    return { value: { type: "unresolved", source: snippet.source }, spans: null };
  }

  resolutions.push({
    source: snippet.source,
    sourceSpan,
    spans: [...spans],
  });
  return { value, spans: textSpan ? spans : null };
}

function resolveLocalSubject(
  local: LocalSubject,
  sourceSpan: SourceSpan | null,
  visibleText: VisibleText,
  anchors: IntralineaAnchor[],
  resolutions: SnippetResolution[],
  diagnostics: Diagnostic[],
): Value | null {
  const anchor = anchors.find(
    (candidate) =>
      candidate.blockSpan.start === local.blockSpan.start &&
      candidate.blockSpan.end === local.blockSpan.end,
  );
  if (anchor === undefined) {
    diagnostics.push(
      diagnostic(
        "InvalidLocalSubjectMarker",
        `Local subject ${local.marker} cannot be anchored in the visible text`,
        sourceSpan,
      ),
    );
    return { type: "unresolvedLocalSubject", marker: local.marker };
  }

  const chars = [...visibleText.text];
  const span =
    local.scope === "sentence"
      ? sentenceSpan(chars, anchor.visibleOffset, local.region)
      : paragraphSpan(chars, anchor.visibleOffset, local.region);

  if (span.start >= span.end) {
    diagnostics.push(
      diagnostic(
        "EmptyLocalSubject",
        `Local subject ${local.marker} selects no text`,
        sourceSpan,
      ),
    );
    return { type: "unresolvedLocalSubject", marker: local.marker };
  }

  resolutions.push({
    source: local.marker,
    sourceSpan,
    spans: [span],
  });
  return null;
}

function isTerminator(ch: string | undefined): boolean {
  return ch === "." || ch === "?" || ch === "!";
}

/**
 * A sentence boundary is `.`, `?`, or `!` followed by whitespace or end
 * of text (the spec's simple v0 rule).
 */
function isSentenceBoundary(chars: string[], index: number): boolean {
  if (!isTerminator(chars[index])) return false;
  const next = chars[index + 1];
  return next === undefined || isWhitespace(next);
}

function skipWhitespaceBack(chars: string[], pos: number): number {
  while (pos > 0) {
    const prev = chars[pos - 1];
    if (prev === undefined || !isWhitespace(prev)) break;
    pos -= 1;
  }
  return pos;
}

function skipWhitespaceForward(chars: string[], pos: number): number {
  while (pos < chars.length) {
    const ch = chars[pos];
    if (ch === undefined || !isWhitespace(ch)) break;
    pos += 1;
  }
  return pos;
}

function forwardSentenceEnd(chars: string[], from: number): number {
  let index = from;
  while (index < chars.length) {
    if (isSentenceBoundary(chars, index)) {
      return index + 1;
    }
    index += 1;
  }
  return chars.length;
}

function sentenceSpan(chars: string[], anchor: number, region: LocalRegion): TextSpan {
  anchor = Math.min(anchor, chars.length);
  if (region === "after") {
    const start = skipWhitespaceForward(chars, anchor);
    return { start, end: forwardSentenceEnd(chars, start) };
  }

  // `<` and `<>` attach backwards: a marker placed just after a
  // completed sentence refers to that sentence.
  const attach = skipWhitespaceBack(chars, anchor);
  const endsAtAttach = attach > 0 && isTerminator(chars[attach - 1]);
  const scanFrom = endsAtAttach ? attach - 1 : attach;
  let start = 0;
  let index = scanFrom;
  while (index > 0) {
    if (isSentenceBoundary(chars, index - 1)) {
      start = index;
      break;
    }
    index -= 1;
  }
  start = Math.min(skipWhitespaceForward(chars, start), attach);

  let end: number;
  if (region === "before") {
    end = attach;
  } else if (endsAtAttach) {
    end = attach;
  } else {
    end = forwardSentenceEnd(chars, attach);
  }
  return { start, end };
}

function paragraphSpan(chars: string[], anchor: number, region: LocalRegion): TextSpan {
  anchor = Math.min(anchor, chars.length);
  if (region === "after") {
    const start = skipWhitespaceForward(chars, anchor);
    const [, boundEnd] = paragraphBounds(chars, start);
    return { start, end: Math.max(boundEnd, start) };
  }

  const attach = skipWhitespaceBack(chars, anchor);
  const [boundStart, boundEnd] = paragraphBounds(chars, Math.max(attach - 1, 0));
  const start = Math.min(boundStart, attach);
  const end = region === "before" ? attach : Math.max(boundEnd, attach);
  return { start, end };
}

/**
 * The paragraph containing `pos`: the surrounding maximal run of
 * non-blank lines, with the end trimmed of trailing whitespace.
 */
function paragraphBounds(chars: string[], pos: number): [number, number] {
  const lines = lineRanges(chars);
  if (lines.length === 0) {
    return [0, 0];
  }
  pos = Math.min(pos, chars.length);
  let line = lines.findIndex((range) => pos >= range[0] && pos <= range[1]);
  if (line < 0) {
    line = lines.length - 1;
  }

  const blank = (range: [number, number]): boolean => {
    for (let i = range[0]; i < range[1]; i += 1) {
      const ch = chars[i];
      if (ch !== undefined && !isWhitespace(ch)) return false;
    }
    return true;
  };
  const current = lines[line];
  if (current !== undefined && blank(current)) {
    return [current[0], current[0]];
  }
  let first = line;
  for (;;) {
    const prev = lines[first - 1];
    if (first > 0 && prev !== undefined && !blank(prev)) {
      first -= 1;
    } else {
      break;
    }
  }
  for (;;) {
    const next = lines[line + 1];
    if (line + 1 < lines.length && next !== undefined && !blank(next)) {
      line += 1;
    } else {
      break;
    }
  }
  const firstLine = lines[first];
  const lastLine = lines[line];
  const start = firstLine !== undefined ? firstLine[0] : 0;
  const end = skipWhitespaceBack(chars, lastLine !== undefined ? lastLine[1] : chars.length);
  return [start, end];
}

/** Line ranges as `[start, end)` scalar offsets, excluding the newline. */
function lineRanges(chars: string[]): [number, number][] {
  const lines: [number, number][] = [];
  let start = 0;
  for (let index = 0; index < chars.length; index += 1) {
    if (chars[index] === "\n") {
      lines.push([start, index]);
      start = index + 1;
    }
  }
  lines.push([start, chars.length]);
  return lines;
}

function diagnostic(
  code: DiagnosticCode,
  message: string,
  span: SourceSpan | null,
): Diagnostic {
  return {
    code,
    severity: "error",
    message,
    span,
    related: [],
  };
}
