/**
 * Faithful port of crates/snipx-core/src/parser.rs.
 *
 * Internally all positions are UTF-16 code-unit indices into the JS
 * source string (the Rust original uses UTF-8 byte indices; the
 * algorithms are unit-agnostic because both advance by whole code
 * points). Diagnostic spans are converted to UTF-8 byte offsets at the
 * `parse()` boundary so nothing downstream ever sees UTF-16 units.
 */

import { isWhitespace } from "./indexMaps.js";
import type { Diagnostic, DiagnosticCode } from "./diagnostic.js";
import { buildTree, type Event, type SyntaxKind, SyntaxNode } from "./syntax.js";

export type InputForm = "commentaria" | "marginalia" | "intralinea";

export interface Parse {
  root: SyntaxNode;
  /**
   * Diagnostic spans in UTF-16 code units of the source; converted to
   * UTF-8 bytes at the export boundary.
   */
  diagnostics: Diagnostic[];
  /** The original source text. */
  source: string;
}

interface RegionParse {
  events: Event[];
  diagnostics: Diagnostic[];
}

type RegionContext = "commentaria" | "marginalia" | "intralinea";

export function parse(source: string, inputForm: InputForm): Parse {
  let region: RegionParse;
  switch (inputForm) {
    case "commentaria":
      region = parseSnipxRegion(source, 0, "commentaria");
      break;
    case "marginalia":
      region = parseMarginalia(source);
      break;
    case "intralinea":
      region = parseIntralinea(source);
      break;
  }
  const root = buildTree(region.events);
  return { root, diagnostics: region.diagnostics, source };
}

function parseMarginalia(source: string): RegionParse {
  const events: Event[] = [{ type: "start", kind: "Root" }];
  const diagnostics: Diagnostic[] = [];
  let cursor = 0;

  while (cursor < source.length) {
    const markerStart = marginaliaSlashMarker(source, cursor);
    if (markerStart !== null) {
      const lineEnd = findLineEnd(source, cursor);
      const contentEnd = lineContentEnd(source, lineEnd);
      const nextLine = nextLineStart(source, lineEnd);
      events.push({ type: "start", kind: "LineComment" });
      if (markerStart > cursor) {
        events.push({ type: "token", kind: "Whitespace", text: source.slice(cursor, markerStart) });
      }
      events.push({ type: "token", kind: "SlashSlashSlash", text: "///" });

      const tailStart = markerStart + 3;
      const tail = source.slice(tailStart, contentEnd);
      const whitespaceLen = leadingWsLen(tail);
      const whitespace = tail.slice(0, whitespaceLen);
      const remainder = tail.slice(whitespaceLen);
      if (whitespace.length > 0) {
        events.push({ type: "token", kind: "Whitespace", text: whitespace });
      }
      if (remainder.length > 0) {
        const regionOffset = contentEnd - remainder.length;
        const region = parseSnipxRegion(remainder, regionOffset, "marginalia");
        replayWithoutRoot(region.events, events);
        diagnostics.push(...region.diagnostics);
      }

      events.push({ type: "finish" });
      if (contentEnd < nextLine) {
        events.push({ type: "token", kind: "Whitespace", text: source.slice(contentEnd, nextLine) });
      }
      cursor = nextLine;
      continue;
    }

    const openingMarker = marginaliaFenceMarker(source, cursor);
    if (openingMarker !== null) {
      const openingEnd = findLineEnd(source, cursor);
      const openingContentEnd = lineContentEnd(source, openingEnd);
      const bodyStart = nextLineStart(source, openingEnd);
      const rawInfo = source.slice(openingMarker + 3, openingContentEnd);
      const info = trimUnicode(rawInfo);
      const closingFence = findClosingFence(source, bodyStart);

      events.push({ type: "start", kind: "Fence" });
      if (openingMarker > cursor) {
        events.push({ type: "token", kind: "Whitespace", text: source.slice(cursor, openingMarker) });
      }
      events.push({ type: "token", kind: "Backtick", text: "```" });
      if (rawInfo.length > 0) {
        events.push({ type: "token", kind: "FenceInfo", text: rawInfo });
      }
      if (openingContentEnd < bodyStart) {
        events.push({
          type: "token",
          kind: "Whitespace",
          text: source.slice(openingContentEnd, bodyStart),
        });
      }

      const bodyEnd = closingFence !== null ? closingFence[0] : source.length;
      const body = source.slice(bodyStart, bodyEnd);
      if (info.length === 0 || info === "snipx") {
        const region = parseSnipxRegion(body, bodyStart, "marginalia");
        replayWithoutRoot(region.events, events);
        diagnostics.push(...region.diagnostics);
      } else if (body.length > 0) {
        events.push({ type: "token", kind: "FenceBody", text: body });
      }

      if (closingFence !== null) {
        const [closingLineStart, closingMarker] = closingFence;
        if (closingMarker > closingLineStart) {
          events.push({
            type: "token",
            kind: "Whitespace",
            text: source.slice(closingLineStart, closingMarker),
          });
        }
        events.push({ type: "token", kind: "Backtick", text: "```" });
        const closingEnd = findLineEnd(source, closingLineStart);
        const closingContentEnd = lineContentEnd(source, closingEnd);
        const closingSuffix = source.slice(closingMarker + 3, closingContentEnd);
        if (closingSuffix.length > 0) {
          events.push({ type: "token", kind: "FenceInfo", text: closingSuffix });
        }
        const nextLine = nextLineStart(source, closingEnd);
        if (closingContentEnd < nextLine) {
          events.push({
            type: "token",
            kind: "Whitespace",
            text: source.slice(closingContentEnd, nextLine),
          });
        }
        cursor = nextLine;
      } else {
        diagnostics.push({
          code: "ParseError",
          severity: "warning",
          message: "Unterminated fence",
          span: { start: cursor, end: source.length },
          related: [],
        });
        cursor = source.length;
      }

      events.push({ type: "finish" });
      continue;
    }

    const nextSpecial = findNextMarginaliaRegion(source, cursor);
    const text = source.slice(cursor, nextSpecial);
    if (text.length > 0) {
      events.push({ type: "token", kind: "MarginaliaText", text });
    }
    cursor = nextSpecial;
  }

  events.push({ type: "finish" });
  return { events, diagnostics };
}

function parseIntralinea(source: string): RegionParse {
  const events: Event[] = [{ type: "start", kind: "Root" }];
  const diagnostics: Diagnostic[] = [];
  let cursor = 0;

  while (cursor < source.length) {
    const found = source.indexOf("{{", cursor);
    if (found >= 0) {
      const start = found;
      if (start > cursor) {
        events.push({ type: "token", kind: "IntralineaText", text: source.slice(cursor, start) });
      }

      events.push({ type: "start", kind: "IntralineaBlock" });
      events.push({ type: "token", kind: "LBrace", text: "{" });
      events.push({ type: "token", kind: "LBrace", text: "{" });

      const bodyStart = start + 2;
      const bodyEnd = findIntralineaClose(source, bodyStart);
      if (bodyEnd !== null) {
        const body = source.slice(bodyStart, bodyEnd);
        const region = parseSnipxRegion(body, bodyStart, "intralinea");
        replayWithoutRoot(region.events, events);
        diagnostics.push(...region.diagnostics);
        events.push({ type: "token", kind: "RBrace", text: "}" });
        events.push({ type: "token", kind: "RBrace", text: "}" });
        events.push({ type: "finish" });
        cursor = bodyEnd + 2;
      } else {
        const body = source.slice(bodyStart);
        const region = parseSnipxRegion(body, bodyStart, "intralinea");
        replayWithoutRoot(region.events, events);
        diagnostics.push(...region.diagnostics);
        diagnostics.push({
          code: "UnterminatedIntralineaBlock",
          severity: "error",
          message: "Unterminated intralinea block",
          span: { start, end: source.length },
          related: [],
        });
        events.push({ type: "finish" });
        cursor = source.length;
      }
      continue;
    }

    events.push({ type: "token", kind: "IntralineaText", text: source.slice(cursor) });
    break;
  }

  events.push({ type: "finish" });
  return { events, diagnostics };
}

function parseSnipxRegion(source: string, offset: number, context: RegionContext): RegionParse {
  const parser = new RegionParser(source, offset, context);
  parser.parse();
  return { events: parser.events, diagnostics: parser.diagnostics };
}

function parseInlineValueRegion(
  source: string,
  offset: number,
  context: RegionContext,
): RegionParse {
  const parser = new RegionParser(source, offset, context);
  parser.parseInlineValueUntil(source.length);
  parser.events.push({ type: "finish" });
  return { events: parser.events, diagnostics: parser.diagnostics };
}

interface PredicateChainState {
  hasPredicate: boolean;
  hasError: boolean;
}

interface ObjectListState {
  hasObject: boolean;
  hasError: boolean;
}

interface ValueParseState {
  hasValue: boolean;
  hasError: boolean;
}

type StatementValueContext = "subjectOrObject" | "predicateRecovery";

class RegionParser {
  source: string;
  offset: number;
  context: RegionContext;
  pos = 0;
  statementSeen = false;
  unterminatedStatement: [number, number] | null = null;
  events: Event[] = [{ type: "start", kind: "Root" }];
  diagnostics: Diagnostic[] = [];

  constructor(source: string, offset: number, context: RegionContext) {
    this.source = source;
    this.offset = offset;
    this.context = context;
  }

  parse(): void {
    while (this.pos < this.source.length) {
      if (this.startsWith("//") && !this.startsWith("///")) {
        this.parseLineComment();
      } else if (this.startsWith("/*")) {
        this.parseBlockComment();
      } else if (this.peekChar() === "@") {
        this.parseDirective();
      } else {
        const ch = this.peekChar();
        if (ch !== null && isWhitespace(ch)) {
          this.parseWhitespace();
        } else {
          this.beginStatement();
          this.parseStatement();
        }
      }
    }
    if (this.context === "commentaria") {
      this.reportUnterminatedStatement();
    }
    this.events.push({ type: "finish" });
  }

  beginStatement(): void {
    this.reportUnterminatedStatement();
    this.statementSeen = true;
  }

  reportUnterminatedStatement(): void {
    if (this.unterminatedStatement !== null) {
      const [start, end] = this.unterminatedStatement;
      this.unterminatedStatement = null;
      this.pushDiagnostic("ParseError", "Expected '.' statement terminator", start, end);
    }
  }

  parseWhitespace(): void {
    const start = this.pos;
    this.consumeWhile(isWhitespace);
    this.tokenFrom("Whitespace", start, this.pos);
  }

  parseLineComment(): void {
    const start = this.pos;
    const end = findLineEnd(this.source, this.pos);
    this.events.push({ type: "start", kind: "LineComment" });
    this.tokenFrom("Text", start, end);
    this.events.push({ type: "finish" });
    this.pos = end;
  }

  parseBlockComment(): void {
    const start = this.pos;
    this.events.push({ type: "start", kind: "BlockComment" });
    this.pos += 2;
    const endRel = this.source.indexOf("*/", this.pos);
    if (endRel >= 0) {
      this.pos = endRel + 2;
      this.tokenFrom("Text", start, this.pos);
    } else {
      this.pos = this.source.length;
      this.tokenFrom("Text", start, this.pos);
      this.pushDiagnostic("UnterminatedBlockComment", "Unterminated block comment", start, this.pos);
    }
    this.events.push({ type: "finish" });
  }

  parseDirective(): void {
    const start = this.pos;
    const nameStart = start + 1;
    let nameLen = 0;
    for (const ch of this.source.slice(nameStart)) {
      if (!isIdentifierContinue(ch)) break;
      nameLen += ch.length;
    }
    const name = this.source.slice(nameStart, nameStart + nameLen);
    const kind: SyntaxKind =
      name === "target" ? "TargetDirective" : name === "profile" ? "ProfileDirective" : "Directive";
    if (this.statementSeen || !isLineStart(this.source, this.pos)) {
      this.pushDiagnostic(
        "InvalidDirectivePosition",
        "Directives must appear in the header before the first statement",
        start,
        start + 1,
      );
    }

    const lineEnd = findLineEnd(this.source, this.pos);
    const contentEnd = lineContentEnd(this.source, lineEnd);
    this.events.push({ type: "start", kind });
    this.token("At", "@");
    this.pos += 1;
    this.consumeIdentifierLike();
    this.tokenFrom("Identifier", start + 1, this.pos);
    if (this.pos < contentEnd) {
      const wsLen = leadingWsLen(this.source.slice(this.pos, contentEnd));
      this.tokenFrom("Whitespace", this.pos, this.pos + wsLen);
      this.pos += wsLen;
      if (this.pos < contentEnd) {
        const valueStart = this.pos;
        const region = parseInlineValueRegion(
          this.source.slice(valueStart, contentEnd),
          this.offset + valueStart,
          this.context,
        );
        replayWithoutRoot(region.events, this.events);
        this.diagnostics.push(...region.diagnostics);
      }
    }
    this.events.push({ type: "finish" });
    this.pos = contentEnd;
  }

  parseDecoration(): void {
    this.events.push({ type: "start", kind: "Decoration" });
    this.token("ColonColon", "::");
    this.pos += 2;
    this.consumeStatementTrivia(false);
    if (this.peekChar() === '"') {
      this.parseString();
    } else {
      const start = this.pos;
      this.events.push({ type: "start", kind: "Error" });
      this.parseValue();
      this.events.push({ type: "finish" });
      this.pushDiagnostic("ParseError", "Expected quoted string after decoration", start, this.pos);
    }
    this.events.push({ type: "finish" });
  }

  parseStatement(): void {
    const start = this.pos;
    let hasSubject = false;
    let hasDecoration = false;
    let hasError = false;
    let predicateState: PredicateChainState;

    this.events.push({ type: "start", kind: "Statement" });
    if (this.context !== "commentaria" && this.startsWith("::")) {
      hasDecoration = true;
      this.parseDecoration();
      this.consumeStatementTrivia(true);
      predicateState = this.parseSemicolonContinuation();
    } else if (this.isAmbientPredicateStart()) {
      predicateState = this.parsePredicateChain();
    } else {
      this.events.push({ type: "start", kind: "Subject" });
      const subjectStart = this.pos;
      const subjectState = this.parseSubjectLike();
      if (subjectState.hasValue) {
        hasSubject = true;
      }
      if (subjectState.hasError) {
        hasError = true;
      }
      if (!subjectState.hasValue && subjectState.hasError) {
        this.pushDiagnostic("ParseError", "Expected subject", subjectStart, this.pos);
      } else if (!subjectState.hasValue) {
        this.pushEmptyError();
        this.pushDiagnostic("ParseError", "Expected subject", subjectStart, subjectStart);
      }
      this.events.push({ type: "finish" });

      this.consumeStatementTrivia(false);

      if (this.startsWith("::")) {
        hasDecoration = true;
        this.parseDecoration();
        this.consumeStatementTrivia(true);
        predicateState = this.parseSemicolonContinuation();
      } else {
        predicateState = this.parsePredicateChain();
      }
    }

    hasError = hasError || predicateState.hasError;
    if (!hasError && !hasDecoration && !predicateState.hasPredicate) {
      this.pushEmptyError();
      this.pushDiagnostic(
        "ParseError",
        hasSubject ? "Expected predicate after subject" : "Expected statement content",
        start,
        this.pos,
      );
    }

    const marker = localSubjectMarkerAt(this.source, this.pos);
    if (marker.kind === "valid" || marker.kind === "invalid") {
      this.parseLocalSubjectMarker();
      this.consumeStatementTrivia(false);
    }

    let terminated = false;
    if (this.peekChar() === ".") {
      this.token("Dot", ".");
      this.pos += 1;
      terminated = true;
    }

    this.events.push({ type: "finish" });
    if (!terminated) {
      this.unterminatedStatement = [start, this.pos];
    }
  }

  isAmbientPredicateStart(): boolean {
    if (this.context === "commentaria") {
      return false;
    }
    const ch = this.peekChar();
    if (ch === "`") return true;
    return ch !== null && (isAsciiLowercase(ch) || ch === "=");
  }

  parsePredicateChain(): PredicateChainState {
    const state: PredicateChainState = { hasPredicate: false, hasError: false };
    while (!this.atStatementEnd() && this.peekChar() !== ".") {
      const predicateStart = this.pos;
      this.events.push({ type: "start", kind: "Predicate" });
      const predicate = this.parsePredicateLike();
      if (predicate.hasValue) {
        state.hasPredicate = true;
      }
      if (predicate.hasError) {
        this.pushDiagnostic("ParseError", "Expected predicate", predicateStart, this.pos);
        state.hasError = true;
      }
      this.events.push({ type: "finish" });

      this.consumeStatementTrivia(false);

      const markerHere = localSubjectMarkerAt(this.source, this.pos);
      if (
        !this.atStatementEnd() &&
        this.peekChar() !== "." &&
        this.peekChar() !== ";" &&
        markerHere.kind === "none"
      ) {
        const objectState = this.parseObjectList();
        state.hasError = state.hasError || objectState.hasError;
        if (!objectState.hasObject) {
          state.hasError = true;
        }
      } else {
        this.pushEmptyError();
        this.pushDiagnostic("ParseError", "Expected object after predicate", predicateStart, this.pos);
        state.hasError = true;
      }

      if (this.peekChar() === ";") {
        const semicolonStart = this.pos;
        this.token("Semicolon", ";");
        this.pos += 1;
        this.consumeStatementTrivia(true);
        if (this.pos >= this.source.length || this.peekChar() === "." || this.peekChar() === "\n") {
          this.pushEmptyError();
          this.pushDiagnostic(
            "ParseError",
            "Expected predicate after semicolon continuation",
            semicolonStart,
            this.pos,
          );
          state.hasError = true;
          break;
        }
        continue;
      }

      break;
    }
    return state;
  }

  parseObjectList(): ObjectListState {
    const state: ObjectListState = { hasObject: false, hasError: false };
    this.events.push({ type: "start", kind: "ObjectList" });
    for (;;) {
      this.events.push({ type: "start", kind: "Object" });
      const objectStart = this.pos;
      const object = this.parseObjectLike();
      if (object.hasValue) {
        state.hasObject = true;
      }
      if (object.hasError) {
        state.hasError = true;
      }
      if (!object.hasValue) {
        if (!object.hasError) {
          this.pushEmptyError();
        }
        this.pushDiagnostic(
          "ParseError",
          "Expected object",
          objectStart,
          object.hasError ? this.pos : objectStart,
        );
        state.hasError = true;
      }
      this.events.push({ type: "finish" });

      if (this.startsWith("::")) {
        this.parseDecoration();
        this.consumeStatementTrivia(false);
      }

      while (this.startsStatementValue(false)) {
        const errorStart = this.pos;
        this.events.push({ type: "start", kind: "Error" });
        const parsedExtraValue = this.parseValue();
        if (!parsedExtraValue) {
          this.pushEmptyError();
        }
        this.events.push({ type: "finish" });
        this.pushDiagnostic(
          "ParseError",
          "Expected comma between object values",
          errorStart,
          this.pos,
        );
        state.hasError = true;
        this.consumeStatementTrivia(false);

        if (this.startsWith("::")) {
          this.parseDecoration();
          this.consumeStatementTrivia(false);
        }
      }

      if (this.peekChar() !== ",") {
        break;
      }
      this.token("Comma", ",");
      this.pos += 1;
      this.consumeStatementTrivia(false);
    }
    this.events.push({ type: "finish" });
    return state;
  }

  parseSemicolonContinuation(): PredicateChainState {
    let state: PredicateChainState = { hasPredicate: false, hasError: false };
    if (this.peekChar() === ";") {
      const semicolonStart = this.pos;
      this.token("Semicolon", ";");
      this.pos += 1;
      this.consumeStatementTrivia(true);
      state = this.parsePredicateChain();
      if (!state.hasPredicate && !state.hasError) {
        this.pushEmptyError();
        this.pushDiagnostic(
          "ParseError",
          "Expected predicate after semicolon continuation",
          semicolonStart,
          this.pos,
        );
        state.hasError = true;
      }
    }
    return state;
  }

  parseSubjectLike(): ValueParseState {
    const marker = localSubjectMarkerAt(this.source, this.pos);
    if (marker.kind === "valid" && this.context === "intralinea") {
      this.parseLocalSubjectMarker();
      this.consumeStatementTrivia(false);
      return { hasValue: true, hasError: false };
    }
    if (marker.kind === "valid" || marker.kind === "invalid") {
      this.parseLocalSubjectMarker();
      this.consumeStatementTrivia(false);
      return { hasValue: false, hasError: true };
    }
    return this.parseStatementValue("subjectOrObject", false);
  }

  parsePredicateLike(): ValueParseState {
    const ch = this.peekChar();
    if (ch === "`") {
      this.events.push({ type: "start", kind: "BacktickPredicate" });
      this.parseBacktickChunk();
      this.events.push({ type: "finish" });
      return { hasValue: true, hasError: false };
    }
    if (ch === "=") {
      this.token("Text", "=");
      this.pos += 1;
      return { hasValue: true, hasError: false };
    }
    if (ch !== null && isAsciiLowercase(ch)) {
      return this.parsePredicateIdentifier();
    }
    this.events.push({ type: "start", kind: "Error" });
    const invalid = this.parseStatementValue("predicateRecovery", false);
    if (!invalid.hasValue && !invalid.hasError) {
      this.pushEmptyError();
    }
    this.events.push({ type: "finish" });
    return { hasValue: false, hasError: true };
  }

  parseObjectLike(): ValueParseState {
    const ch = this.peekChar();
    const marker = localSubjectMarkerAt(this.source, this.pos);
    if (
      this.atStatementEnd() ||
      ch === "," ||
      ch === ";" ||
      ch === "." ||
      this.startsWith("::") ||
      marker.kind === "valid" ||
      marker.kind === "invalid"
    ) {
      return { hasValue: false, hasError: false };
    }
    const value = this.parseStatementValue("subjectOrObject", false);
    if (value.hasValue || value.hasError) {
      this.consumeStatementTrivia(false);
    }
    return value;
  }

  parseInlineValueUntil(end: number): void {
    while (this.pos < end) {
      const ch = this.peekChar();
      if (ch !== null && isWhitespace(ch)) {
        const start = this.pos;
        while (this.pos < end) {
          const current = this.peekChar();
          if (current === null || !isWhitespace(current)) break;
          this.advanceChar();
        }
        this.tokenFrom("Whitespace", start, this.pos);
      } else if (!this.parseValue()) {
        const start = this.pos;
        this.advanceChar();
        this.tokenFrom("Text", start, this.pos);
      }
    }
  }

  parseStatementValue(context: StatementValueContext, allowEquals: boolean): ValueParseState {
    const ch = this.peekChar();
    switch (ch) {
      case "[":
        this.parseSnippet();
        return { hasValue: true, hasError: false };
      case "{":
        if (context === "predicateRecovery") {
          this.parseCapture();
          return { hasValue: true, hasError: false };
        }
        return this.parseDisallowedStatementValue("Captures are only valid inside snippets", () =>
          this.parseCapture(),
        );
      case '"':
        this.parseString();
        return { hasValue: true, hasError: false };
      case "<":
        this.parseUri();
        return { hasValue: true, hasError: false };
      case "`":
        if (context === "predicateRecovery") {
          this.parseBacktickChunk();
          return { hasValue: true, hasError: false };
        }
        return this.parseDisallowedStatementValue("Backtick chunks are only valid as predicates", () =>
          this.parseBacktickChunk(),
        );
      case "~":
        if (this.source.startsWith("~[", this.pos)) {
          this.token("Tilde", "~");
          this.pos += 1;
          this.parseSnippet();
          return { hasValue: true, hasError: false };
        }
        this.parseInvalidStatementValue();
        return { hasValue: false, hasError: true };
      case "+":
      case "*":
      case "?":
        this.parseInvalidStatementValue();
        return { hasValue: false, hasError: true };
      case "-":
        if (this.atNegativeNumber()) {
          this.parseNumber();
          return { hasValue: true, hasError: false };
        }
        this.parseInvalidStatementValue();
        return { hasValue: false, hasError: true };
      case ".":
      case ",":
      case ";":
      case null:
        return { hasValue: false, hasError: false };
      default:
        if (isAsciiDigit(ch)) {
          this.parseNumber();
          return { hasValue: true, hasError: false };
        }
        if (isIdentifierStart(ch)) {
          if (context === "subjectOrObject") {
            return this.parseSubjectOrObjectIdentifier();
          }
          this.parseIdentifierLike();
          return { hasValue: true, hasError: false };
        }
        if (ch === "=" && allowEquals) {
          this.token("Text", "=");
          this.pos += 1;
          return { hasValue: true, hasError: false };
        }
        this.parseInvalidStatementValue();
        return { hasValue: false, hasError: true };
    }
  }

  parseDisallowedStatementValue(message: string, parse: () => void): ValueParseState {
    const start = this.pos;
    this.events.push({ type: "start", kind: "Error" });
    parse();
    this.events.push({ type: "finish" });
    this.pushDiagnostic("ParseError", message, start, this.pos);
    return { hasValue: false, hasError: true };
  }

  parseValue(): boolean {
    const marker = localSubjectMarkerAt(this.source, this.pos);
    if (marker.kind === "valid" || marker.kind === "invalid") {
      this.parseLocalSubjectMarker();
      return true;
    }

    const ch = this.peekChar();
    switch (ch) {
      case "[":
        this.parseSnippet();
        return true;
      case "{":
        this.parseCapture();
        return true;
      case '"':
        this.parseString();
        return true;
      case "<":
        this.parseUri();
        return true;
      case "`":
        this.parseBacktickChunk();
        return true;
      case "~":
        if (this.source.startsWith("[", this.pos + 1)) {
          this.token("Tilde", "~");
          this.pos += 1;
          this.parseSnippet();
          return true;
        }
        break;
      case "+":
      case "*":
      case "?": {
        this.events.push({ type: "start", kind: "Quantifier" });
        this.token("Text", ch);
        this.pos += 1;
        this.events.push({ type: "finish" });
        return true;
      }
      case "-":
        if (this.atNegativeNumber()) {
          this.parseNumber();
          return true;
        }
        break;
      case ".":
        return false;
      case ",":
      case ";":
        return false;
      case null:
        return false;
      default:
        break;
    }

    if (ch === null) return false;
    if (isAsciiDigit(ch)) {
      this.parseNumber();
      return true;
    }
    if (isIdentifierStart(ch)) {
      this.parseIdentifierLike();
      return true;
    }
    const start = this.pos;
    this.advanceChar();
    this.tokenFrom("Text", start, this.pos);
    return true;
  }

  parseInvalidStatementValue(): void {
    const start = this.pos;
    this.events.push({ type: "start", kind: "Error" });
    const ch = this.peekChar();
    if (ch === "+" || ch === "*" || ch === "?") {
      this.events.push({ type: "start", kind: "Quantifier" });
      this.token("Text", ch);
      this.pos += 1;
      this.events.push({ type: "finish" });
    } else if (ch !== null) {
      const innerStart = this.pos;
      this.advanceChar();
      this.tokenFrom("Text", innerStart, this.pos);
    }
    this.events.push({ type: "finish" });
    this.pushDiagnostic("ParseError", "Unexpected token in statement value", start, this.pos);
  }

  parseSnippet(): void {
    const start = this.pos;
    const isRange = snippetContainsRange(this.source.slice(this.pos), this.context === "intralinea");
    const kind: SyntaxKind = isRange ? "RangeSnippet" : "Snippet";
    this.events.push({ type: "start", kind });
    this.token("LBrack", "[");
    this.pos += 1;
    let textStart = this.pos;
    let captureCount = 0;
    for (;;) {
      const ch = this.peekChar();
      if (ch === null) break;
      if (ch === "]" || ch === "\n" || ch === "\r") {
        break;
      }
      if (ch === '"') {
        if (this.pos > textStart) {
          this.tokenFrom("Text", textStart, this.pos);
        }
        this.events.push({ type: "start", kind: "QuotedSnippetPart" });
        this.parseString();
        this.events.push({ type: "finish" });
        textStart = this.pos;
      } else if (ch === "{") {
        if (this.pos > textStart) {
          this.tokenFrom("Text", textStart, this.pos);
        }
        const captureStart = this.pos;
        const invalidCapture = isRange || captureCount > 0;
        if (invalidCapture) {
          this.events.push({ type: "start", kind: "Error" });
        }
        this.parseCapture();
        if (invalidCapture) {
          this.events.push({ type: "finish" });
          this.pushDiagnostic(
            "ParseError",
            isRange
              ? "Captures are not allowed inside range snippets"
              : "Snippets may contain at most one capture",
            captureStart,
            this.pos,
          );
        }
        captureCount += 1;
        textStart = this.pos;
      } else if (isRange && this.startsWith("..")) {
        if (this.pos > textStart) {
          this.tokenFrom("Text", textStart, this.pos);
        }
        this.token("Dot", "..");
        this.pos += 2;
        textStart = this.pos;
      } else {
        this.advanceChar();
      }
    }
    if (this.pos > textStart) {
      this.tokenFrom("Text", textStart, this.pos);
    }
    if (this.peekChar() === "]") {
      this.token("RBrack", "]");
      this.pos += 1;
    } else {
      this.pushDiagnostic("UnterminatedSnippet", "Unterminated snippet", start, this.pos);
    }
    const quantifier = this.peekChar();
    if (quantifier === "+" || quantifier === "*" || quantifier === "?") {
      this.events.push({ type: "start", kind: "Quantifier" });
      this.token("Text", quantifier);
      this.pos += 1;
      this.events.push({ type: "finish" });
    }
    this.events.push({ type: "finish" });
  }

  parseCapture(): void {
    const start = this.pos;
    this.events.push({ type: "start", kind: "Capture" });
    this.token("LBrace", "{");
    this.pos += 1;
    const innerStart = this.pos;
    for (;;) {
      const ch = this.peekChar();
      if (ch === null || ch === "}" || ch === "\n" || ch === "\r") {
        break;
      }
      this.advanceChar();
    }
    if (this.pos > innerStart) {
      this.tokenFrom("Text", innerStart, this.pos);
    }
    if (this.peekChar() === "}") {
      this.token("RBrace", "}");
      this.pos += 1;
    } else {
      this.pushDiagnostic("ParseError", "Unterminated capture", start, this.pos);
    }
    this.events.push({ type: "finish" });
  }

  parseString(): void {
    if (this.source.startsWith('"""', this.pos)) {
      this.parseTripleString();
      return;
    }

    const start = this.pos;
    this.events.push({ type: "start", kind: "String" });
    this.token("Quote", '"');
    this.pos += 1;
    const contentStart = this.pos;
    for (;;) {
      const ch = this.peekChar();
      if (ch === null || ch === '"' || ch === "\n" || ch === "\r") {
        break;
      }
      if (ch === "\\") {
        this.advanceChar();
        const next = this.peekChar();
        if (next !== null && next !== "\n" && next !== "\r") {
          this.advanceChar();
        }
        continue;
      }
      this.advanceChar();
    }
    if (this.pos > contentStart) {
      this.tokenFrom("Text", contentStart, this.pos);
    }
    if (this.peekChar() === '"') {
      this.token("Quote", '"');
      this.pos += 1;
    } else {
      this.pushDiagnostic("UnterminatedString", "Unterminated string", start, this.pos);
    }
    this.events.push({ type: "finish" });
  }

  parseTripleString(): void {
    const start = this.pos;
    this.events.push({ type: "start", kind: "TripleString" });
    this.token("Quote", '"""');
    this.pos += 3;
    const contentStart = this.pos;
    const endRel = this.source.indexOf('"""', this.pos);
    if (endRel >= 0) {
      this.pos = endRel;
      if (this.pos > contentStart) {
        this.tokenFrom("Text", contentStart, this.pos);
      }
      this.token("Quote", '"""');
      this.pos += 3;
    } else {
      this.pos = this.source.length;
      if (this.pos > contentStart) {
        this.tokenFrom("Text", contentStart, this.pos);
      }
      this.pushDiagnostic("UnterminatedString", "Unterminated triple string", start, this.pos);
    }
    this.events.push({ type: "finish" });
  }

  parseUri(): void {
    this.events.push({ type: "start", kind: "Uri" });
    this.token("LAngle", "<");
    this.pos += 1;
    const contentStart = this.pos;
    for (;;) {
      const ch = this.peekChar();
      if (ch === null || ch === ">" || isWhitespace(ch)) {
        break;
      }
      this.advanceChar();
    }
    if (this.pos > contentStart) {
      this.tokenFrom("Text", contentStart, this.pos);
    }
    if (this.peekChar() === ">") {
      this.token("RAngle", ">");
      this.pos += 1;
    } else {
      this.pushDiagnostic(
        "ParseError",
        "Unterminated URI literal",
        Math.max(contentStart - 1, 0),
        this.pos,
      );
    }
    this.events.push({ type: "finish" });
  }

  parseLocalSubjectMarker(): void {
    const marker = localSubjectMarkerAt(this.source, this.pos);
    if (marker.kind === "none") return;
    const len = marker.len;
    const invalid = marker.kind === "invalid";
    const start = this.pos;
    const end = start + len;
    const disallowed = this.context !== "intralinea";
    const kind: SyntaxKind = disallowed ? "Error" : "LocalSubjectMarker";
    this.events.push({ type: "start", kind });
    while (this.pos < end) {
      const ch = this.peekChar();
      if (ch === "~") {
        this.token("Tilde", "~");
      } else if (ch === "<") {
        this.token("LAngle", "<");
      } else if (ch === ">") {
        this.token("RAngle", ">");
      } else {
        break;
      }
      this.advanceChar();
    }
    this.events.push({ type: "finish" });
    if (invalid || disallowed) {
      this.pushDiagnostic(
        "InvalidLocalSubjectMarker",
        disallowed
          ? "Local subject markers are only valid in intralinea regions"
          : "Invalid local subject marker",
        start,
        end,
      );
    }
  }

  parseBacktickChunk(): void {
    const start = this.pos;
    this.token("Backtick", "`");
    this.pos += 1;
    const contentStart = this.pos;
    for (;;) {
      const ch = this.peekChar();
      if (ch === null || ch === "`" || ch === "\n" || ch === "\r") {
        break;
      }
      this.advanceChar();
    }
    if (this.pos > contentStart) {
      this.tokenFrom("Text", contentStart, this.pos);
    }
    if (this.peekChar() === "`") {
      this.token("Backtick", "`");
      this.pos += 1;
    } else {
      this.pushDiagnostic("ParseError", "Unterminated backtick chunk", start, this.pos);
    }
  }

  atNegativeNumber(): boolean {
    if (this.peekChar() !== "-") return false;
    const next = codePointAt(this.source, this.pos + 1);
    return next !== null && isAsciiDigit(next);
  }

  parseNumber(): void {
    const start = this.pos;
    if (this.peekChar() === "-") {
      this.pos += 1;
    }
    this.consumeWhile(isAsciiDigit);
    if (this.peekChar() === ".") {
      const next = codePointAt(this.source, this.pos + 1);
      if (next !== null && isAsciiDigit(next)) {
        this.pos += 1;
        this.consumeWhile(isAsciiDigit);
      }
    }
    this.events.push({ type: "start", kind: "Number" });
    this.tokenFrom("Text", start, this.pos);
    this.events.push({ type: "finish" });
  }

  parseIdentifierLike(): void {
    const start = this.pos;
    this.consumeWhile(isIdentifierContinue);
    const text = this.source.slice(start, this.pos);
    const kind: SyntaxKind = text === "true" || text === "false" ? "Boolean" : "Identifier";
    this.events.push({ type: "start", kind });
    this.tokenFrom("Text", start, this.pos);
    this.events.push({ type: "finish" });
  }

  parseSubjectOrObjectIdentifier(): ValueParseState {
    const start = this.pos;
    this.consumeWhile(isIdentifierContinue);
    const text = this.source.slice(start, this.pos);
    const kind: SyntaxKind = text === "true" || text === "false" ? "Boolean" : "Identifier";
    const first = text.length > 0 ? text[0] : undefined;
    const invalid =
      kind === "Identifier" && first !== undefined && (isAsciiLowercase(first) || first === "_");

    if (invalid) {
      this.events.push({ type: "start", kind: "Error" });
    }
    this.events.push({ type: "start", kind });
    this.tokenFrom("Text", start, this.pos);
    this.events.push({ type: "finish" });
    if (invalid) {
      this.events.push({ type: "finish" });
      this.pushDiagnostic(
        "ParseError",
        "Lowercase identifiers are only valid as predicates",
        start,
        this.pos,
      );
      return { hasValue: true, hasError: true };
    }
    return { hasValue: true, hasError: false };
  }

  parsePredicateIdentifier(): ValueParseState {
    const start = this.pos;
    this.consumeWhile(isIdentifierContinue);
    const text = this.source.slice(start, this.pos);
    const kind: SyntaxKind = text === "true" || text === "false" ? "Boolean" : "Identifier";
    if (kind === "Boolean") {
      this.events.push({ type: "start", kind: "Error" });
    }
    this.events.push({ type: "start", kind });
    this.tokenFrom("Text", start, this.pos);
    this.events.push({ type: "finish" });
    if (kind === "Boolean") {
      this.events.push({ type: "finish" });
      return { hasValue: false, hasError: true };
    }
    return { hasValue: true, hasError: false };
  }

  consumeInlineWhitespace(): void {
    for (;;) {
      const ch = this.peekChar();
      if (ch === null || ch === "\n") {
        break;
      }
      if (isWhitespace(ch)) {
        const start = this.pos;
        this.consumeWhile((current) => isWhitespace(current) && current !== "\n");
        this.tokenFrom("Whitespace", start, this.pos);
      } else {
        break;
      }
    }
  }

  consumeStatementTrivia(allowNewline: boolean): void {
    for (;;) {
      if (this.startsWith("//") && !this.startsWith("///")) {
        this.parseLineComment();
        if (!allowNewline) {
          break;
        }
        continue;
      }
      if (this.startsWith("/*")) {
        this.parseBlockComment();
        continue;
      }
      const ch = this.peekChar();
      if (ch !== null && isWhitespace(ch)) {
        if (allowNewline) {
          this.parseWhitespace();
        } else {
          if (ch === "\n") {
            break;
          }
          this.consumeInlineWhitespace();
        }
        continue;
      }
      break;
    }
  }

  atStatementEnd(): boolean {
    const ch = this.peekChar();
    return ch === null || ch === "\n";
  }

  startsStatementValue(allowEquals: boolean): boolean {
    const marker = localSubjectMarkerAt(this.source, this.pos);
    if (this.startsWith("::") || marker.kind === "valid" || marker.kind === "invalid") {
      return false;
    }

    const ch = this.peekChar();
    if (ch === null) return false;
    if (ch === "[" || ch === "{" || ch === '"' || ch === "<" || ch === "`") return true;
    if (ch === "~") return this.source.startsWith("~[", this.pos);
    if (ch === "+" || ch === "*" || ch === "?") return true;
    if (ch === "-") return this.atNegativeNumber();
    if (isAsciiDigit(ch)) return true;
    if (isIdentifierStart(ch)) return true;
    if (ch === "=" && allowEquals) return true;
    return false;
  }

  consumeIdentifierLike(): void {
    this.consumeWhile(isIdentifierContinue);
  }

  consumeWhile(predicate: (ch: string) => boolean): void {
    for (;;) {
      const ch = this.peekChar();
      if (ch === null || !predicate(ch)) {
        break;
      }
      this.advanceChar();
    }
  }

  advanceChar(): void {
    const ch = this.peekChar();
    if (ch !== null) {
      this.pos += ch.length;
    }
  }

  peekChar(): string | null {
    return codePointAt(this.source, this.pos);
  }

  startsWith(pattern: string): boolean {
    return this.source.startsWith(pattern, this.pos);
  }

  token(kind: SyntaxKind, text: string): void {
    this.events.push({ type: "token", kind, text });
  }

  tokenFrom(kind: SyntaxKind, start: number, end: number): void {
    if (end > start) {
      this.events.push({ type: "token", kind, text: this.source.slice(start, end) });
    }
  }

  pushDiagnostic(code: DiagnosticCode, message: string, start: number, end: number): void {
    this.diagnostics.push({
      code,
      severity: "error",
      message,
      span: { start: this.offset + start, end: this.offset + end },
      related: [],
    });
  }

  pushEmptyError(): void {
    this.events.push({ type: "start", kind: "Error" });
    this.events.push({ type: "finish" });
  }
}

function replayWithoutRoot(events: Event[], out: Event[]): void {
  let depth = 0;
  for (const event of events) {
    switch (event.type) {
      case "start":
        if (event.kind === "Root" && depth === 0) {
          depth = 1;
        } else {
          depth += 1;
          out.push(event);
        }
        break;
      case "finish":
        if (depth === 1) {
          depth = 0;
        } else {
          depth = Math.max(depth - 1, 0);
          out.push(event);
        }
        break;
      case "token":
        out.push(event);
        break;
    }
  }
}

function codePointAt(source: string, index: number): string | null {
  if (index >= source.length || index < 0) return null;
  const cp = source.codePointAt(index);
  if (cp === undefined) return null;
  return String.fromCodePoint(cp);
}

export function findLineEnd(source: string, from: number): number {
  const idx = source.indexOf("\n", from);
  return idx < 0 ? source.length : idx;
}

export function lineContentEnd(source: string, lineEnd: number): number {
  if (lineEnd < source.length && source.slice(0, lineEnd).endsWith("\r")) {
    return lineEnd - 1;
  }
  return lineEnd;
}

export function nextLineStart(source: string, lineEnd: number): number {
  return lineEnd < source.length ? lineEnd + 1 : lineEnd;
}

function marginaliaSlashMarker(source: string, lineStart: number): number | null {
  if (!isLineStart(source, lineStart)) {
    return null;
  }
  const lineEnd = findLineEnd(source, lineStart);
  const contentEnd = lineContentEnd(source, lineEnd);
  const indentation = leadingWsLen(source.slice(lineStart, contentEnd));
  const markerStart = lineStart + indentation;
  return source.slice(markerStart, contentEnd).startsWith("///") ? markerStart : null;
}

function marginaliaFenceMarker(source: string, lineStart: number): number | null {
  if (!isLineStart(source, lineStart)) {
    return null;
  }
  const lineEnd = findLineEnd(source, lineStart);
  const contentEnd = lineContentEnd(source, lineEnd);
  const indentation = leadingWsLen(source.slice(lineStart, contentEnd));
  const markerStart = lineStart + indentation;
  return source.slice(markerStart, contentEnd).startsWith("```") ? markerStart : null;
}

function isLineStart(source: string, pos: number): boolean {
  return pos === 0 || source.slice(0, pos).endsWith("\n");
}

function findClosingFence(source: string, from: number): [number, number] | null {
  let cursor = from;
  while (cursor < source.length) {
    const markerStart = marginaliaFenceMarker(source, cursor);
    if (markerStart !== null) {
      return [cursor, markerStart];
    }
    const lineEnd = findLineEnd(source, cursor);
    cursor = nextLineStart(source, lineEnd);
  }
  return null;
}

function findNextMarginaliaRegion(source: string, from: number): number {
  let cursor = from;
  while (cursor < source.length) {
    if (
      isLineStart(source, cursor) &&
      (marginaliaSlashMarker(source, cursor) !== null ||
        marginaliaFenceMarker(source, cursor) !== null)
    ) {
      return cursor;
    }
    const lineEnd = findLineEnd(source, cursor);
    cursor = nextLineStart(source, lineEnd);
  }
  return source.length;
}

function findIntralineaClose(source: string, from: number): number | null {
  let cursor = from;
  let captureDepth = 0;
  let quote: string | null = null;
  let lineComment = false;
  let blockComment = false;

  while (cursor < source.length) {
    const tail = source.slice(cursor);
    const first = codePointAt(source, cursor);
    if (lineComment) {
      if (tail.startsWith("\n")) {
        lineComment = false;
      }
      if (first === null) return null;
      cursor += first.length;
      continue;
    }
    if (blockComment) {
      const blockEnd = tail.indexOf("*/");
      if (blockEnd >= 0) {
        blockComment = false;
        cursor += blockEnd + 2;
      } else {
        const intralineaClose = tail.indexOf("}}");
        if (intralineaClose >= 0) {
          return cursor + intralineaClose;
        }
        cursor = source.length;
      }
      continue;
    }
    if (quote !== null) {
      if (quote === '"') {
        const boundary = scanSameLineQuoteBoundary(tail, '"', true);
        if (boundary.kind === "lexicalClose" || boundary.kind === "newline") {
          quote = null;
          cursor += boundary.index + 1;
        } else if (boundary.kind === "hostClose") {
          return cursor + boundary.index;
        } else {
          cursor = source.length;
        }
      } else if (quote === '"""') {
        const tripleEnd = tail.indexOf('"""');
        if (tripleEnd >= 0) {
          quote = null;
          cursor += tripleEnd + 3;
        } else {
          const intralineaClose = tail.indexOf("}}");
          if (intralineaClose >= 0) {
            return cursor + intralineaClose;
          }
          cursor = source.length;
        }
      } else if (quote === "`") {
        const boundary = scanSameLineQuoteBoundary(tail, "`", false);
        if (boundary.kind === "lexicalClose" || boundary.kind === "newline") {
          quote = null;
          cursor += boundary.index + 1;
        } else if (boundary.kind === "hostClose") {
          return cursor + boundary.index;
        } else {
          cursor = source.length;
        }
      } else {
        // Unreachable: quote is always one of the three delimiters above.
        if (first === null) return null;
        cursor += first.length;
      }
      continue;
    }

    if (tail.startsWith('"""')) {
      quote = '"""';
      cursor += 3;
    } else if (tail.startsWith('"')) {
      quote = '"';
      cursor += 1;
    } else if (tail.startsWith("`")) {
      quote = "`";
      cursor += 1;
    } else {
      const snippetLen = intralineaSnippetLen(tail);
      if (snippetLen !== null) {
        cursor += snippetLen;
        continue;
      }
      const uriLen = intralineaUriLiteralLen(tail);
      if (uriLen !== null) {
        cursor += uriLen;
        continue;
      }
      if (captureDepth > 0 && (first === "\n" || first === "\r")) {
        captureDepth = 0;
        cursor += first.length;
        continue;
      }
      if (captureDepth === 0 && tail.startsWith("//")) {
        lineComment = true;
        cursor += 2;
        continue;
      }
      if (captureDepth === 0 && tail.startsWith("/*")) {
        blockComment = true;
        cursor += 2;
        continue;
      }
      const prefix = intralineaCloseCapturePrefix(tail, captureDepth);
      if (prefix !== null) {
        if (prefix === 0) {
          return cursor;
        }
        captureDepth -= prefix;
        cursor += prefix;
        continue;
      }
      if (tail.startsWith("{")) {
        captureDepth += 1;
        cursor += 1;
        continue;
      }
      if (tail.startsWith("}") && captureDepth > 0) {
        captureDepth -= 1;
        cursor += 1;
        continue;
      }
      if (first === null) return null;
      cursor += first.length;
    }
  }

  return null;
}

function intralineaSnippetLen(source: string): number | null {
  if (!source.startsWith("[")) {
    return null;
  }

  let cursor = 1;
  let captureDepth = 0;
  let quoted = false;

  while (cursor < source.length) {
    const tail = source.slice(cursor);
    const first = codePointAt(source, cursor);
    if (!quoted) {
      if (first === "\n" || first === "\r") {
        return cursor;
      }
      const prefix = intralineaCloseCapturePrefix(tail, captureDepth);
      if (prefix !== null) {
        if (prefix === 0) {
          return cursor;
        }
        captureDepth -= prefix;
        cursor += prefix;
        continue;
      }
    }

    if (quoted) {
      const boundary = scanSameLineQuoteBoundary(tail, '"', true);
      if (boundary.kind === "lexicalClose" || boundary.kind === "newline") {
        quoted = false;
        cursor += boundary.index + 1;
      } else if (boundary.kind === "hostClose") {
        return cursor + boundary.index;
      } else {
        return source.length;
      }
      continue;
    }

    if (first === null) return source.length;
    switch (first) {
      case '"':
        quoted = true;
        break;
      case "{":
        captureDepth += 1;
        break;
      case "}":
        if (captureDepth > 0) captureDepth -= 1;
        break;
      case "]":
        if (captureDepth === 0) return cursor + 1;
        break;
      default:
        break;
    }
    cursor += first.length;
  }

  return source.length;
}

type SameLineQuoteBoundary =
  | { kind: "lexicalClose"; index: number }
  | { kind: "newline"; index: number }
  | { kind: "hostClose"; index: number }
  | { kind: "eof" };

function scanSameLineQuoteBoundary(
  source: string,
  lexicalClose: string,
  supportsEscapes: boolean,
): SameLineQuoteBoundary {
  let cursor = 0;
  let recoveryClose: number | null = null;

  while (cursor < source.length) {
    const tail = source.slice(cursor);
    if (tail.startsWith("}}")) {
      if (recoveryClose === null) {
        recoveryClose = cursor;
      }
      cursor += 2;
      continue;
    }

    const ch = codePointAt(source, cursor);
    if (ch === null) break;
    if (ch === "\n" || ch === "\r") {
      return recoveryClose !== null
        ? { kind: "hostClose", index: recoveryClose }
        : { kind: "newline", index: cursor };
    }
    if (ch === lexicalClose) {
      return { kind: "lexicalClose", index: cursor };
    }
    if (supportsEscapes && ch === "\\") {
      cursor += ch.length;
      if (source.startsWith("}}", cursor)) {
        return { kind: "hostClose", index: cursor };
      }
      if (cursor < source.length) {
        const next = codePointAt(source, cursor);
        if (next !== null && next !== "\n" && next !== "\r") {
          cursor += next.length;
        }
      }
    } else {
      cursor += ch.length;
    }
  }

  return recoveryClose !== null ? { kind: "hostClose", index: recoveryClose } : { kind: "eof" };
}

function intralineaCloseCapturePrefix(source: string, captureDepth: number): number | null {
  if (!source.startsWith("}}")) {
    return null;
  }
  let closingRun = 0;
  for (const ch of source) {
    if (ch !== "}") break;
    closingRun += 1;
  }
  return Math.min(captureDepth, Math.max(closingRun - 2, 0));
}

function intralineaUriLiteralLen(source: string): number | null {
  if (!source.startsWith("<")) {
    return null;
  }

  const first = codePointAt(source, 1);
  if (first === null) return null;
  if (isWhitespace(first) || first === "<" || first === ">") {
    return null;
  }

  let recoveryClose: number | null = null;
  let idx = 1 + first.length;
  while (idx < source.length) {
    const ch = codePointAt(source, idx);
    if (ch === null) break;
    if (source.startsWith("}}", idx)) {
      if (recoveryClose === null) {
        recoveryClose = idx;
      }
    }
    if (ch === ">") {
      return idx + 1;
    }
    if (isWhitespace(ch)) {
      return recoveryClose ?? idx;
    }
    idx += ch.length;
  }

  return recoveryClose ?? source.length;
}

export function leadingWsLen(text: string): number {
  let idx = 0;
  for (const ch of text) {
    if (!isWhitespace(ch)) {
      return idx;
    }
    idx += ch.length;
  }
  return text.length;
}

function trimUnicode(text: string): string {
  let start = 0;
  let end = text.length;
  while (start < end) {
    const ch = codePointAt(text, start);
    if (ch === null || !isWhitespace(ch)) break;
    start += ch.length;
  }
  while (end > start) {
    // Step back over one code point.
    const prev = end >= 2 && isLowSurrogateAt(text, end - 1) ? end - 2 : end - 1;
    const ch = codePointAt(text, prev);
    if (ch === null || !isWhitespace(ch)) break;
    end = prev;
  }
  return text.slice(start, end);
}

function isLowSurrogateAt(text: string, index: number): boolean {
  const code = text.charCodeAt(index);
  return code >= 0xdc00 && code <= 0xdfff;
}

type LocalSubjectMarkerMatch =
  | { kind: "none" }
  | { kind: "valid"; len: number }
  | { kind: "invalid"; len: number };

function localSubjectMarkerAt(source: string, pos: number): LocalSubjectMarkerMatch {
  const tail = source.slice(pos);
  const markerStart = tail.startsWith("~") ? 1 : 0;
  const markerTail = tail.slice(markerStart);
  let angleLen = markerTail.length;
  for (let i = 0; i < markerTail.length; i += 1) {
    const ch = markerTail[i];
    if (ch !== "<" && ch !== ">") {
      angleLen = i;
      break;
    }
  }

  if (angleLen === 0) {
    return { kind: "none" };
  }

  const markerLen = markerStart + angleLen;
  if (!isLocalSubjectMarkerBoundary(codePointAt(tail, markerLen))) {
    return { kind: "none" };
  }

  const body = markerTail.slice(0, angleLen);
  switch (body) {
    case "<":
    case ">":
    case "<>":
    case "<<":
    case ">>":
    case "<<>>":
      return { kind: "valid", len: markerLen };
    default:
      return { kind: "invalid", len: markerLen };
  }
}

function isIdentifierStart(ch: string): boolean {
  return /^[A-Za-z_]$/.test(ch);
}

function isIdentifierContinue(ch: string): boolean {
  return /^[A-Za-z0-9_-]$/.test(ch);
}

function isAsciiLowercase(ch: string): boolean {
  return ch >= "a" && ch <= "z";
}

function isAsciiDigit(ch: string): boolean {
  return ch >= "0" && ch <= "9";
}

function snippetContainsRange(source: string, intralineaCloseAware: boolean): boolean {
  let cursor = source.startsWith("[") ? 1 : 0;
  let quoted = false;
  let captureDepth = 0;

  while (cursor < source.length) {
    const tail = source.slice(cursor);
    if (intralineaCloseAware && !quoted) {
      const prefix = intralineaCloseCapturePrefix(tail, captureDepth);
      if (prefix !== null) {
        if (prefix === 0) {
          return false;
        }
        captureDepth -= prefix;
        cursor += prefix;
        continue;
      }
    }

    const ch = codePointAt(source, cursor);
    if (ch === null) break;
    if (ch === "\n" || ch === "\r") {
      return false;
    }
    if (quoted) {
      if (ch === "\\") {
        cursor += 1;
        if (cursor < source.length) {
          const next = codePointAt(source, cursor);
          if (next !== null && next !== "\n" && next !== "\r") {
            cursor += next.length;
          }
        }
        continue;
      }
      if (ch === '"') {
        quoted = false;
      }
    } else {
      if (ch === '"' && captureDepth === 0) {
        quoted = true;
      } else if (ch === "{") {
        captureDepth += 1;
      } else if (ch === "}" && captureDepth > 0) {
        captureDepth -= 1;
      } else if (ch === "]" && captureDepth === 0) {
        return false;
      } else if (ch === "." && captureDepth === 0 && tail.startsWith("..")) {
        return true;
      }
    }
    cursor += ch.length;
  }

  return false;
}

function isLocalSubjectMarkerBoundary(next: string | null): boolean {
  if (next === null) return true;
  return isWhitespace(next) || next === "." || next === "," || next === ";";
}
