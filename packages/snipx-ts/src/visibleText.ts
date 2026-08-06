/**
 * Mirror of crates/snipx-core/src/visible_text.rs, which pins
 * pulldown-cmark 0.13 behaviour. The TypeScript implementation walks a
 * commonmark.js AST and applies the same enumerated rules:
 *
 * - Text and inline-code content is included (code without delimiters).
 * - Soft and hard breaks insert a newline.
 * - Entering or leaving a paragraph, heading, block quote, code block,
 *   or list item inserts a newline (deduplicated).
 * - Link URLs are excluded (only child text contributes; autolink text
 *   is a text child and therefore included, matching pulldown-cmark).
 * - Image alt text is included (children of the image node).
 * - Raw HTML is dropped entirely with RAW_HTML_OMITTED warnings; block
 *   HTML warns once per source line (matching pulldown-cmark's per-line
 *   Html events), inline HTML once per tag.
 *
 * Warning spans are UTF-16 code-unit offsets into the target text here;
 * the export boundary converts them to UTF-8 bytes with the target
 * text's index map.
 */

import { Parser } from "commonmark";

import type { Diagnostic } from "./diagnostic.js";

export type Profile = "plain" | "plain-loose" | "markdown" | "markdown-loose";

export function profileFromName(name: string): Profile | null {
  switch (name) {
    case "plain":
    case "plain-loose":
    case "markdown":
    case "markdown-loose":
      return name;
    default:
      return null;
  }
}

export interface VisibleText {
  /** NFC-normalised visible text. */
  text: string;
  normalisation: "NFC";
  profile: Profile;
  /** RAW_HTML_OMITTED warnings, spans in UTF-16 units of the target. */
  diagnostics: Diagnostic[];
}

export function extractVisibleText(source: string, profile: Profile): VisibleText {
  if (profile === "plain" || profile === "plain-loose") {
    return {
      text: source.normalize("NFC"),
      normalisation: "NFC",
      profile,
      diagnostics: [],
    };
  }
  return extractMarkdown(source, profile);
}

/** UTF-16 offsets of each line start, for sourcepos conversion. */
function lineStarts(source: string): number[] {
  const starts = [0];
  for (let i = 0; i < source.length; i += 1) {
    if (source[i] === "\n") {
      starts.push(i + 1);
    }
  }
  return starts;
}

function extractMarkdown(source: string, profile: Profile): VisibleText {
  const parser = new Parser();
  const document = parser.parse(source);
  const starts = lineStarts(source);
  const lineStart = (line: number): number => {
    const index = Math.max(0, Math.min(line - 1, starts.length - 1));
    const offset = starts[index];
    return offset ?? 0;
  };
  const nextLineStart = (line: number): number => {
    const index = line; // line is 1-based; starts[line] is the next line.
    const offset = starts[index];
    return offset ?? source.length;
  };

  let text = "";
  const diagnostics: Diagnostic[] = [];
  /**
   * commonmark.js records sourcepos for block nodes only. Inline HTML
   * tags are verbatim source text, so we locate each one by searching
   * forward from a cursor that resets to the enclosing block's start
   * and advances past every located tag.
   */
  let inlineCursor = 0;
  const pushNewline = (): void => {
    if (text.length > 0 && !text.endsWith("\n")) {
      text += "\n";
    }
  };
  const rawHtmlWarning = (start: number, end: number): void => {
    diagnostics.push({
      code: "RawHtmlOmitted",
      severity: "warning",
      message: "Raw HTML is omitted from Markdown visible text",
      span: { start, end },
      related: [],
    });
  };

  const walker = document.walker();
  let step = walker.next();
  while (step !== null) {
    const { node, entering } = step;
    switch (node.type) {
      case "text":
        if (entering) {
          text += node.literal ?? "";
        }
        break;
      case "code":
        if (entering) {
          text += node.literal ?? "";
        }
        break;
      case "softbreak":
      case "linebreak":
        if (entering) {
          pushNewline();
        }
        break;
      case "paragraph":
      case "heading":
        if (entering) {
          const sourcepos = node.sourcepos;
          if (sourcepos !== undefined) {
            const [[startLine, startCol]] = sourcepos;
            inlineCursor = lineStart(startLine) + (startCol - 1);
          }
        }
        pushNewline();
        break;
      case "block_quote":
      case "item":
        pushNewline();
        break;
      case "code_block":
        if (entering) {
          // pulldown-cmark: Start(CodeBlock) newline, Text(content),
          // End(CodeBlock) newline.
          pushNewline();
          text += node.literal ?? "";
          pushNewline();
        }
        break;
      case "html_block": {
        if (entering) {
          const sourcepos = node.sourcepos;
          if (sourcepos !== undefined) {
            const [[startLine], [endLine]] = sourcepos;
            // pulldown-cmark emits one Html event per source line, each
            // span running to the start of the next line.
            for (let line = startLine; line <= endLine; line += 1) {
              rawHtmlWarning(lineStart(line), nextLineStart(line));
            }
          }
        }
        break;
      }
      case "html_inline": {
        if (entering) {
          const literal = node.literal ?? "";
          if (literal.length > 0) {
            const index = source.indexOf(literal, inlineCursor);
            if (index >= 0) {
              rawHtmlWarning(index, index + literal.length);
              inlineCursor = index + literal.length;
            }
          }
        }
        break;
      }
      default:
        // document, list, emph, strong, link, image, thematic_break:
        // no direct text contribution (children are visited normally).
        break;
    }
    step = walker.next();
  }

  return {
    text: text.normalize("NFC"),
    normalisation: "NFC",
    profile,
    diagnostics,
  };
}
