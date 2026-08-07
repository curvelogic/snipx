/** Mirror of crates/snipx-core/src/snippet.rs. */

import type { SyntaxNode } from "./syntax.js";

export type SnippetPart =
  | { type: "text"; text: string }
  | { type: "quoted"; raw: string; decoded: string; terminated: boolean }
  | { type: "capture"; text: string; terminated: boolean }
  | { type: "rangeSeparator" };

export type Cardinality = "exactlyOne" | "oneOrMore" | "zeroOrMore" | "zeroOrOne";

export interface SnippetValue {
  /** Trimmed source syntax (`[Alice]+`), `~` already stripped by the caller. */
  source: string;
  parts: SnippetPart[];
  cardinality: Cardinality;
  /** False when the closing `]` is missing. */
  terminated: boolean;
}

/** `node` must be a Snippet or RangeSnippet node. */
export function snippetValueFromNode(node: SyntaxNode, source: string): SnippetValue {
  const parts: SnippetPart[] = [];
  let cardinality: Cardinality = "exactlyOne";
  let terminated = false;

  for (const element of node.children) {
    if (element.type === "token") {
      switch (element.kind) {
        case "LBrack":
          break;
        case "RBrack":
          terminated = true;
          break;
        case "Dot":
          parts.push({ type: "rangeSeparator" });
          break;
        default:
          parts.push({ type: "text", text: element.text });
          break;
      }
    } else {
      switch (element.kind) {
        case "QuotedSnippetPart":
          parts.push(quotedPart(element));
          break;
        case "Capture":
          parts.push(capturePart(element));
          break;
        case "Quantifier": {
          const text = element.toText();
          cardinality =
            text === "+"
              ? "oneOrMore"
              : text === "*"
                ? "zeroOrMore"
                : text === "?"
                  ? "zeroOrOne"
                  : "exactlyOne";
          break;
        }
        // Invalid captures (second capture, capture in a range) are
        // wrapped in an Error node by the parser; surface them so the
        // matcher reports the same InvalidSnippet errors.
        case "Error": {
          let capture: SyntaxNode | null = null;
          for (const inner of element.descendants()) {
            if (inner.kind === "Capture") {
              capture = inner;
              break;
            }
          }
          if (capture !== null) {
            parts.push(capturePart(capture));
          } else {
            parts.push({ type: "text", text: element.toText() });
          }
          break;
        }
        default:
          parts.push({ type: "text", text: element.toText() });
          break;
      }
    }
  }

  return { source, parts, cardinality, terminated };
}

/**
 * Snippet quoting is very literal: only the quote delimiter itself is
 * escaped, so decoding maps `\"` to `"` and nothing else.
 */
function quotedPart(node: SyntaxNode): SnippetPart {
  const raw = node.toText();
  let quotes = 0;
  let content = "";
  for (const element of node.descendantsWithTokens()) {
    if (element.type === "token") {
      if (element.kind === "Quote") {
        quotes += 1;
      } else if (element.kind === "Text") {
        content += element.text;
      }
    }
  }
  return {
    type: "quoted",
    raw,
    decoded: content.replaceAll('\\"', '"'),
    terminated: quotes >= 2,
  };
}

function capturePart(node: SyntaxNode): SnippetPart {
  let text = "";
  let terminated = false;
  for (const element of node.children) {
    if (element.type === "token") {
      if (element.kind === "Text") {
        text += element.text;
      } else if (element.kind === "RBrace") {
        terminated = true;
      }
    }
  }
  return { type: "capture", text, terminated };
}
