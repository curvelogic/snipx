/**
 * Mirror of crates/snipx-core/src/match.rs.
 *
 * All spans here are Unicode scalar offsets into the NFC-normalised
 * visible text, exactly as the canonical JSON contract requires. The
 * implementation works over arrays of code points so no UTF-16
 * code-unit arithmetic can leak into span values.
 */

import type { Diagnostic, DiagnosticCode } from "./diagnostic.js";
import { isWhitespace as isRustWhitespace } from "./indexMaps.js";
import type { SnippetPart } from "./snippet.js";
import type { Profile, VisibleText } from "./visibleText.js";

export interface TextSpan {
  start: number;
  end: number;
}

interface NormalizedText {
  /** Code points of the normalised text. */
  chars: string[];
  /** For each normalised code point, the scalar start/end in the input. */
  starts: number[];
  ends: number[];
}

export type MatchResult =
  | { ok: true; spans: TextSpan[] }
  | { ok: false; error: Diagnostic };

function scalarCount(text: string): number {
  let count = 0;
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  for (const _ of text) {
    count += 1;
  }
  return count;
}

function matchPattern(
  pattern: string,
  capture: { start: number; end: number } | null,
  visibleText: VisibleText,
  profile: Profile,
): MatchResult {
  const loose = profile === "plain-loose" || profile === "markdown-loose";
  const haystack = normalize(visibleText.text, loose);
  const needle = normalize(pattern, loose);

  if (needle.chars.length === 0) {
    return { ok: true, spans: [{ start: 0, end: scalarCount(visibleText.text) }] };
  }

  let captureNormalized: { start: number; end: number } | null = null;
  if (capture !== null) {
    const patternChars = [...pattern];
    const prefix = patternChars.slice(0, capture.start).join("");
    const throughCapture = patternChars.slice(0, capture.end).join("");
    captureNormalized = {
      start: normalize(prefix, loose).chars.length,
      end: normalize(throughCapture, loose).chars.length,
    };
  }
  if (captureNormalized !== null && captureNormalized.start >= captureNormalized.end) {
    return {
      ok: false,
      error: invalid("InvalidSnippet", "Capture boundaries collapse during text normalisation"),
    };
  }

  const spans: TextSpan[] = [];
  let lastEnd = 0;
  for (const matched of findMatches(haystack, needle.chars)) {
    const selected =
      captureNormalized !== null
        ? {
            start: matched.start + captureNormalized.start,
            end: matched.start + captureNormalized.end,
          }
        : matched;
    const spanStart = haystack.starts[selected.start];
    const spanEnd = haystack.ends[selected.end - 1];
    if (spanStart === undefined || spanEnd === undefined) {
      continue;
    }
    const span: TextSpan = { start: spanStart, end: spanEnd };
    if (span.start < lastEnd) {
      continue;
    }
    lastEnd = span.end;
    spans.push(span);
  }
  return { ok: true, spans };
}

function matchRange(
  start: string,
  end: string,
  visibleText: VisibleText,
  profile: Profile,
): MatchResult {
  const documentEnd = scalarCount(visibleText.text);

  if (start.length === 0 && end.length === 0) {
    return { ok: true, spans: [{ start: 0, end: documentEnd }] };
  }
  // Open ranges resolve like any other snippet: every candidate match of
  // the open endpoint is a candidate span, and the caller's cardinality
  // rules decide whether several candidates are ambiguous.
  if (start.length === 0) {
    const ends = matchPattern(end, null, visibleText, profile);
    if (!ends.ok) return ends;
    return {
      ok: true,
      spans: ends.spans.map((span) => ({ start: 0, end: span.end })),
    };
  }
  if (end.length === 0) {
    const starts = matchPattern(start, null, visibleText, profile);
    if (!starts.ok) return starts;
    return {
      ok: true,
      spans: starts.spans.map((span) => ({ start: span.start, end: documentEnd })),
    };
  }

  const starts = matchPattern(start, null, visibleText, profile);
  if (!starts.ok) return starts;
  const ends = matchPattern(end, null, visibleText, profile);
  if (!ends.ok) return ends;
  let lastEnd = 0;
  const ranges: TextSpan[] = [];
  for (const startSpan of starts.spans) {
    if (startSpan.start < lastEnd) {
      continue;
    }
    const endSpan = ends.spans.find((candidate) => candidate.start >= startSpan.end);
    if (endSpan !== undefined) {
      ranges.push({ start: startSpan.start, end: endSpan.end });
      lastEnd = endSpan.end;
    }
  }
  return { ok: true, spans: ranges };
}

export function matchSnippet(
  parts: SnippetPart[],
  visibleText: VisibleText,
  profile: Profile,
): MatchResult {
  const separators = parts.filter((part) => part.type === "rangeSeparator").length;
  if (separators > 1) {
    return {
      ok: false,
      error: invalid("InvalidSnippet", "A range snippet may contain only one range separator"),
    };
  }
  if (separators === 1) {
    if (parts.some((part) => part.type === "capture")) {
      return {
        ok: false,
        error: invalid("InvalidSnippet", "Captures are not allowed inside range snippets"),
      };
    }
    const split = parts.findIndex((part) => part.type === "rangeSeparator");
    const start = assemblePattern(parts.slice(0, split));
    if (!start.ok) return start;
    const end = assemblePattern(parts.slice(split + 1));
    if (!end.ok) return end;
    return matchRange(start.pattern, end.pattern, visibleText, profile);
  }

  const assembled = assemblePattern(parts);
  if (!assembled.ok) return assembled;
  return matchPattern(assembled.pattern, assembled.capture, visibleText, profile);
}

type AssembleResult =
  | { ok: true; pattern: string; capture: { start: number; end: number } | null }
  | { ok: false; error: Diagnostic };

/**
 * Spec ("Quoted Snippet Text"): quotes delimit only when they wrap an
 * entire snippet body or an entire range endpoint; anywhere else they
 * are literal target text.
 */
function assemblePattern(parts: SnippetPart[]): AssembleResult {
  if (parts.length === 1 && parts[0] !== undefined && parts[0].type === "quoted") {
    const quoted = parts[0];
    if (!quoted.terminated) {
      return { ok: false, error: invalid("InvalidSnippet", "Quoted snippet text is not terminated") };
    }
    return { ok: true, pattern: quoted.decoded, capture: null };
  }

  let pattern = "";
  let patternScalars = 0;
  let capture: { start: number; end: number } | null = null;
  for (const part of parts) {
    switch (part.type) {
      case "text":
        pattern += part.text;
        patternScalars += scalarCount(part.text);
        break;
      case "quoted":
        if (!part.terminated) {
          return {
            ok: false,
            error: invalid("InvalidSnippet", "Quoted snippet text is not terminated"),
          };
        }
        pattern += part.raw;
        patternScalars += scalarCount(part.raw);
        break;
      case "capture": {
        if (capture !== null) {
          return {
            ok: false,
            error: invalid("InvalidSnippet", "A snippet may contain at most one capture"),
          };
        }
        if (!part.terminated) {
          return { ok: false, error: invalid("InvalidSnippet", "Capture is not terminated") };
        }
        if (part.text.length === 0) {
          return { ok: false, error: invalid("InvalidSnippet", "Capture may not be empty") };
        }
        const start = patternScalars;
        pattern += part.text;
        patternScalars += scalarCount(part.text);
        capture = { start, end: patternScalars };
        break;
      }
      case "rangeSeparator":
        throw new Error("range separators are handled by the caller");
    }
  }
  return { ok: true, pattern, capture };
}

/** Non-overlapping needle occurrences in scalar indices. */
function findMatches(
  haystack: NormalizedText,
  needle: string[],
): { start: number; end: number }[] {
  const matches: { start: number; end: number }[] = [];
  if (needle.length === 0) return matches;
  let cursor = 0;
  while (cursor + needle.length <= haystack.chars.length) {
    let found = -1;
    for (let i = cursor; i + needle.length <= haystack.chars.length; i += 1) {
      let matched = true;
      for (let j = 0; j < needle.length; j += 1) {
        if (haystack.chars[i + j] !== needle[j]) {
          matched = false;
          break;
        }
      }
      if (matched) {
        found = i;
        break;
      }
    }
    if (found < 0) break;
    matches.push({ start: found, end: found + needle.length });
    cursor = found + needle.length;
  }
  return matches;
}

export function normalize(input: string, loose: boolean): NormalizedText {
  const nfc = input.normalize("NFC");
  const chars: string[] = [];
  const starts: number[] = [];
  const ends: number[] = [];
  let whitespaceStart: number | null = null;

  const nfcChars = [...nfc];
  for (let index = 0; index < nfcChars.length; index += 1) {
    const character = nfcChars[index];
    if (character === undefined) continue;
    if (loose && isRustWhitespace(character)) {
      if (whitespaceStart === null) {
        whitespaceStart = index;
      }
      continue;
    }

    if (whitespaceStart !== null) {
      chars.push(" ");
      starts.push(whitespaceStart);
      ends.push(index);
      whitespaceStart = null;
    }

    const replacement = loose ? (looseReplacement(character) ?? character) : character;
    for (const replacementCharacter of replacement) {
      chars.push(replacementCharacter);
      starts.push(index);
      ends.push(index + 1);
    }
  }

  if (whitespaceStart !== null) {
    chars.push(" ");
    starts.push(whitespaceStart);
    ends.push(nfcChars.length);
  }

  return { chars, starts, ends };
}

function looseReplacement(character: string): string | null {
  switch (character) {
    case "\u{2010}":
    case "\u{2011}":
    case "\u{2012}":
    case "\u{2013}":
    case "\u{2014}":
    case "\u{2212}":
      return "-";
    case "\u{2018}":
    case "\u{2019}":
    case "\u{201a}":
    case "\u{201b}":
      return "'";
    case "\u{201c}":
    case "\u{201d}":
    case "\u{201e}":
    case "\u{201f}":
      return '"';
    case "\u{fb00}":
      return "ff";
    case "\u{fb01}":
      return "fi";
    case "\u{fb02}":
      return "fl";
    case "\u{fb03}":
      return "ffi";
    case "\u{fb04}":
      return "ffl";
    case "\u{fb05}":
    case "\u{fb06}":
      return "st";
    default:
      return null;
  }
}

function invalid(code: DiagnosticCode, message: string): Diagnostic {
  return {
    code,
    severity: "error",
    message,
    span: null,
    related: [],
  };
}
