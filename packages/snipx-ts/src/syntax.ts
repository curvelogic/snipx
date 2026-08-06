/**
 * Lightweight syntax tree mirroring the rowan green/red tree used by the
 * Rust implementation. Offsets are UTF-16 code units into the source
 * string; conversion to UTF-8 bytes happens at the export boundary via
 * the index maps.
 */

export type SyntaxKind =
  | "Root"
  | "Directive"
  | "TargetDirective"
  | "ProfileDirective"
  | "Statement"
  | "Subject"
  | "Predicate"
  | "Object"
  | "ObjectList"
  | "Decoration"
  | "Snippet"
  | "RangeSnippet"
  | "QuotedSnippetPart"
  | "Capture"
  | "Quantifier"
  | "Uri"
  | "String"
  | "TripleString"
  | "Number"
  | "Boolean"
  | "BacktickPredicate"
  | "LineComment"
  | "BlockComment"
  | "MarginaliaText"
  | "Fence"
  | "FenceInfo"
  | "FenceBody"
  | "IntralineaText"
  | "IntralineaBlock"
  | "LocalSubjectMarker"
  | "Identifier"
  | "Whitespace"
  | "Error"
  | "Text"
  | "LBrack"
  | "RBrack"
  | "LBrace"
  | "RBrace"
  | "LAngle"
  | "RAngle"
  | "ColonColon"
  | "At"
  | "Tilde"
  | "Quote"
  | "Backtick"
  | "SlashSlashSlash"
  | "Semicolon"
  | "Comma"
  | "Dot";

export interface SyntaxToken {
  readonly type: "token";
  readonly kind: SyntaxKind;
  readonly text: string;
  /** UTF-16 code-unit offsets into the full source. */
  readonly start: number;
  readonly end: number;
}

export class SyntaxNode {
  readonly type = "node" as const;
  readonly kind: SyntaxKind;
  readonly children: (SyntaxNode | SyntaxToken)[] = [];
  parent: SyntaxNode | null = null;
  start = 0;
  end = 0;

  constructor(kind: SyntaxKind) {
    this.kind = kind;
  }

  /** Direct child nodes (no tokens). */
  childNodes(): SyntaxNode[] {
    return this.children.filter((c): c is SyntaxNode => c.type === "node");
  }

  /** Pre-order descendants including this node, matching rowan. */
  *descendants(): Generator<SyntaxNode> {
    yield this;
    for (const child of this.children) {
      if (child.type === "node") {
        yield* child.descendants();
      }
    }
  }

  *descendantsWithTokens(): Generator<SyntaxNode | SyntaxToken> {
    yield this;
    for (const child of this.children) {
      if (child.type === "node") {
        yield* child.descendantsWithTokens();
      } else {
        yield child;
      }
    }
  }

  *ancestors(): Generator<SyntaxNode> {
    let current: SyntaxNode | null = this;
    while (current !== null) {
      yield current;
      current = current.parent;
    }
  }

  /** Concatenated token text, matching rowan's node-to-string. */
  toText(): string {
    let out = "";
    for (const child of this.children) {
      out += child.type === "token" ? child.text : child.toText();
    }
    return out;
  }
}

export type Event =
  | { type: "start"; kind: SyntaxKind }
  | { type: "token"; kind: SyntaxKind; text: string }
  | { type: "finish" };

/** Build a red tree from a flat event stream. */
export function buildTree(events: Event[]): SyntaxNode {
  const stack: SyntaxNode[] = [];
  let root: SyntaxNode | null = null;
  let offset = 0;

  for (const event of events) {
    switch (event.type) {
      case "start": {
        const node = new SyntaxNode(event.kind);
        node.start = offset;
        const top = stack[stack.length - 1];
        if (top !== undefined) {
          node.parent = top;
          top.children.push(node);
        }
        stack.push(node);
        break;
      }
      case "token": {
        const top = stack[stack.length - 1];
        if (top === undefined) {
          throw new Error("token outside of any node");
        }
        top.children.push({
          type: "token",
          kind: event.kind,
          text: event.text,
          start: offset,
          end: offset + event.text.length,
        });
        offset += event.text.length;
        break;
      }
      case "finish": {
        const node = stack.pop();
        if (node === undefined) {
          throw new Error("finish without matching start");
        }
        node.end = offset;
        if (stack.length === 0) {
          root = node;
        }
        break;
      }
    }
  }

  if (root === null) {
    throw new Error("event stream produced no root node");
  }
  return root;
}
