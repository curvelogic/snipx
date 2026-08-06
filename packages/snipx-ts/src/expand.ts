/** Mirror of crates/snipx-core/src/expand.rs. */

import {
  nodeSpan,
  objectDecorations,
  objectListObjects,
  statementDecorations,
  statementObjectLists,
  statementPredicates,
  statementSubject,
} from "./ast.js";
import type { Diagnostic, SourceSpan } from "./diagnostic.js";
import type { Parse } from "./parser.js";
import type { TextSpan } from "./match.js";
import { snippetValueFromNode, type SnippetValue } from "./snippet.js";
import type { SyntaxNode } from "./syntax.js";
import { isWhitespace } from "./indexMaps.js";

export type LocalScope = "sentence" | "paragraph";
export type LocalRegion = "before" | "after" | "whole";

export interface LocalSubject {
  marker: string;
  scope: LocalScope;
  region: LocalRegion;
  textSpan: boolean;
  /** UTF-16 code-unit span of the enclosing intralinea block. */
  blockSpan: SourceSpan;
}

export type Value =
  | { type: "name"; value: string }
  | { type: "predicate"; value: string }
  | { type: "string"; value: string }
  | { type: "number"; value: number }
  | { type: "invalidNumber"; source: string }
  | { type: "boolean"; value: boolean }
  | { type: "uri"; value: string }
  | { type: "snippet"; snippet: SnippetValue }
  | { type: "textSpanSnippet"; snippet: SnippetValue }
  | { type: "resolvedTextSpan"; snippet: SnippetValue; span: TextSpan }
  | { type: "localSubject"; local: LocalSubject }
  | { type: "wholeDocument" }
  | { type: "unresolved"; source: string }
  | { type: "unresolvedLocalSubject"; marker: string };

export interface ExpandedStatement {
  subject: Value;
  /** Spans in UTF-16 code units of the source. */
  subjectSpan: SourceSpan | null;
  predicate: Value;
  predicateSpan: SourceSpan | null;
  object: Value;
  objectSpan: SourceSpan | null;
  statementSpan: SourceSpan;
}

export interface ExpandOptions {
  ambientSubject: Value | null;
}

export interface ExpandResult {
  statements: ExpandedStatement[];
  diagnostics: Diagnostic[];
}

export function expand(parse: Parse, options: ExpandOptions): ExpandResult {
  const result: ExpandResult = {
    statements: [],
    diagnostics: [...parse.diagnostics],
  };

  for (const node of parse.root.descendants()) {
    if (node.kind === "Statement") {
      expandStatement(node, options, result, parse);
    }
  }

  return result;
}

function expandStatement(
  statement: SyntaxNode,
  options: ExpandOptions,
  result: ExpandResult,
  parse: Parse,
): void {
  const statementSpan = nodeSpan(statement);
  const explicitSubject = statementSubject(statement);
  let subjectSpan: SourceSpan | null =
    explicitSubject !== null ? nodeSpan(explicitSubject) : null;
  let subject: Value | null = null;
  if (explicitSubject !== null) {
    subject = valueFromNode(explicitSubject);
  }
  if (subject === null) {
    const local = localSubjectValue(statement);
    if (local !== null) {
      subjectSpan = local[1];
      subject = local[0];
    } else {
      subject = options.ambientSubject;
      if (explicitSubject === null) {
        subjectSpan = null;
      }
    }
  }

  if (subject === null) {
    result.diagnostics.push({
      code: "MissingAmbientSubject",
      severity: "error",
      message: "Subjectless statement requires an ambient subject",
      span: nodeSpan(statement),
      related: [],
    });
    return;
  }
  diagnoseInvalidNumber(subject, subjectSpan, result);

  for (const decoration of statementDecorations(statement)) {
    pushDecoration(subject, subjectSpan, decoration, result);
  }

  const predicates = statementPredicates(statement);
  const objectLists = statementObjectLists(statement);
  const pairCount = Math.min(predicates.length, objectLists.length);
  for (let i = 0; i < pairCount; i += 1) {
    const predicateNode = predicates[i];
    const objectList = objectLists[i];
    if (predicateNode === undefined || objectList === undefined) continue;
    const predicateSpan = nodeSpan(predicateNode);
    const predicate: Value = { type: "predicate", value: predicateText(predicateNode) };

    for (const object of objectListObjects(objectList)) {
      const value = valueFromNode(object);
      if (value === null) {
        continue;
      }
      const objectSpan = nodeSpan(object);
      diagnoseInvalidNumber(value, objectSpan, result);
      result.statements.push({
        subject,
        subjectSpan,
        predicate,
        predicateSpan,
        object: value,
        objectSpan,
        statementSpan,
      });

      for (const decoration of objectDecorations(object)) {
        pushDecoration(value, objectSpan, decoration, result);
      }
    }
  }
  void parse;
}

function pushDecoration(
  subject: Value,
  subjectSpan: SourceSpan | null,
  decoration: SyntaxNode,
  result: ExpandResult,
): void {
  const objectNode =
    decoration
      .childNodes()
      .find((child) => child.kind === "String" || child.kind === "TripleString") ?? null;
  const object = objectNode !== null ? valueFromNode(objectNode) : null;

  if (object === null) {
    result.diagnostics.push({
      code: "InvalidDecorationTarget",
      severity: "error",
      message: "Decoration requires a quoted string",
      span: nodeSpan(decoration),
      related: [],
    });
    return;
  }

  result.statements.push({
    subject,
    subjectSpan,
    predicate: { type: "predicate", value: "note" },
    predicateSpan: null,
    object,
    objectSpan: objectNode !== null ? nodeSpan(objectNode) : null,
    statementSpan: nodeSpan(decoration),
  });
}

function localSubjectValue(statement: SyntaxNode): [Value, SourceSpan] | null {
  let markerNode: SyntaxNode | null = null;
  for (const node of statement.descendants()) {
    if (node.kind === "LocalSubjectMarker") {
      markerNode = node;
      break;
    }
  }
  if (markerNode === null) return null;
  const marker = markerNode.toText();
  let textSpan = false;
  let body = marker;
  if (marker.startsWith("~")) {
    textSpan = true;
    body = marker.slice(1);
  }
  let scope: LocalScope;
  let region: LocalRegion;
  switch (body) {
    case "<":
      scope = "sentence";
      region = "before";
      break;
    case ">":
      scope = "sentence";
      region = "after";
      break;
    case "<>":
      scope = "sentence";
      region = "whole";
      break;
    case "<<":
      scope = "paragraph";
      region = "before";
      break;
    case ">>":
      scope = "paragraph";
      region = "after";
      break;
    case "<<>>":
      scope = "paragraph";
      region = "whole";
      break;
    default:
      return null;
  }
  let block: SyntaxNode | null = null;
  for (const ancestor of markerNode.ancestors()) {
    if (ancestor.kind === "IntralineaBlock") {
      block = ancestor;
      break;
    }
  }
  if (block === null) return null;
  const span = nodeSpan(markerNode);
  return [
    {
      type: "localSubject",
      local: {
        marker,
        scope,
        region,
        textSpan,
        blockSpan: nodeSpan(block),
      },
    },
    span,
  ];
}

function predicateText(node: SyntaxNode): string {
  const text = trimRust(node.toText());
  if (text.startsWith("`") && text.endsWith("`") && text.length >= 2) {
    return text.slice(1, -1);
  }
  return text;
}

const VALUE_KINDS = new Set([
  "Snippet",
  "RangeSnippet",
  "Uri",
  "String",
  "TripleString",
  "Number",
  "Boolean",
  "Identifier",
]);

export function valueFromNode(node: SyntaxNode): Value | null {
  let valueNode: SyntaxNode | null = null;
  for (const candidate of node.descendants()) {
    if (VALUE_KINDS.has(candidate.kind)) {
      valueNode = candidate;
      break;
    }
  }
  if (valueNode === null) return null;
  const text = valueNode.toText();

  switch (valueNode.kind) {
    case "Snippet":
    case "RangeSnippet": {
      const valueText = node.toText();
      const syntax = trimRust(valueText);
      let textSpan = false;
      let source = syntax;
      if (syntax.startsWith("~")) {
        textSpan = true;
        source = syntax.slice(1);
      }
      const snippet = snippetValueFromNode(valueNode, source);
      return textSpan
        ? { type: "textSpanSnippet", snippet }
        : { type: "snippet", snippet };
    }
    case "Uri": {
      const stripped =
        text.startsWith("<") && text.endsWith(">") && text.length >= 2
          ? text.slice(1, -1)
          : text;
      return { type: "uri", value: stripped };
    }
    case "String":
      return { type: "string", value: unescape(unquote(text, 1)) };
    case "TripleString":
      return { type: "string", value: dedent(unquote(text, 3)) };
    case "Number": {
      const number = parseRustF64(text);
      if (number === null) return null;
      return Number.isFinite(number)
        ? { type: "number", value: number }
        : { type: "invalidNumber", source: text };
    }
    case "Boolean":
      if (text === "true") return { type: "boolean", value: true };
      if (text === "false") return { type: "boolean", value: false };
      return null;
    case "Identifier":
      return { type: "name", value: text };
    default:
      return null;
  }
}

/** Rust `str::parse::<f64>` for the lexical forms parse_number emits. */
function parseRustF64(text: string): number | null {
  if (!/^-?(\d+)(\.\d+)?$/.test(text)) {
    return null;
  }
  return Number(text);
}

function diagnoseInvalidNumber(
  value: Value,
  span: SourceSpan | null,
  result: ExpandResult,
): void {
  if (value.type === "invalidNumber") {
    result.diagnostics.push({
      code: "InvalidNumber",
      severity: "error",
      message: `JSON number is outside the finite range: ${value.source}`,
      span,
      related: [],
    });
  }
}

function unescape(text: string): string {
  let output = "";
  let i = 0;
  while (i < text.length) {
    const ch = text[i];
    if (ch === undefined) break;
    if (ch !== "\\") {
      output += ch;
      i += 1;
      continue;
    }
    const next = text[i + 1];
    switch (next) {
      case "n":
        output += "\n";
        i += 2;
        break;
      case "t":
        output += "\t";
        i += 2;
        break;
      case "r":
        output += "\r";
        i += 2;
        break;
      case "\\":
        output += "\\";
        i += 2;
        break;
      case '"':
        output += '"';
        i += 2;
        break;
      case "'":
        output += "'";
        i += 2;
        break;
      case "0":
        output += "\0";
        i += 2;
        break;
      case undefined:
        output += "\\";
        i += 1;
        break;
      // Unknown escapes stay verbatim rather than silently dropping text.
      default:
        output += "\\" + next;
        i += 2;
        break;
    }
  }
  return output;
}

function dedent(text: string): string {
  let body: string;
  if (text.startsWith("\n")) {
    body = text.slice(1);
  } else if (text.startsWith("\r\n")) {
    body = text.slice(2);
  } else {
    return text;
  }

  const lines = body.split("\n");
  const indents = lines
    .filter((line) => trimRust(line).length > 0)
    .map((line) => indentLen(line));
  const commonIndent = indents.length > 0 ? Math.min(...indents) : 0;

  return lines
    .map((line) => line.slice(Math.min(commonIndent, indentLen(line))))
    .join("\n");
}

function indentLen(line: string): number {
  let i = 0;
  while (i < line.length && (line[i] === " " || line[i] === "\t")) {
    i += 1;
  }
  return i;
}

function unquote(text: string, quoteWidth: number): string {
  if (text.length < 2 * quoteWidth) return text;
  return text.slice(quoteWidth, text.length - quoteWidth);
}

/** Rust `str::trim`: Unicode White_Space from both ends. */
export function trimRust(text: string): string {
  let start = 0;
  while (start < text.length) {
    const cp = text.codePointAt(start);
    if (cp === undefined) break;
    const ch = String.fromCodePoint(cp);
    if (!isWhitespace(ch)) break;
    start += ch.length;
  }
  let end = text.length;
  while (end > start) {
    const code = text.charCodeAt(end - 1);
    const prev = code >= 0xdc00 && code <= 0xdfff && end >= 2 ? end - 2 : end - 1;
    const cp = text.codePointAt(prev);
    if (cp === undefined) break;
    if (!isWhitespace(String.fromCodePoint(cp))) break;
    end = prev;
  }
  return text.slice(start, end);
}
