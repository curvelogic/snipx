/** Mirror of the AST accessors in crates/snipx-core/src/ast.rs. */

import type { SourceSpan } from "./diagnostic.js";
import type { SyntaxNode, SyntaxToken } from "./syntax.js";

export interface DirectiveValue {
  value: string;
  /** UTF-16 code-unit span into the source (converted at export). */
  span: SourceSpan;
}

export interface HeaderDirectives {
  profile: DirectiveValue | null;
  target: DirectiveValue | null;
  duplicates: [string, SourceSpan][];
}

export function nodeSpan(node: SyntaxNode): SourceSpan {
  return { start: node.start, end: node.end };
}

export function statementSubject(statement: SyntaxNode): SyntaxNode | null {
  return statement.childNodes().find((child) => child.kind === "Subject") ?? null;
}

export function statementPredicates(statement: SyntaxNode): SyntaxNode[] {
  return statement.childNodes().filter((child) => child.kind === "Predicate");
}

export function statementObjectLists(statement: SyntaxNode): SyntaxNode[] {
  return statement.childNodes().filter((child) => child.kind === "ObjectList");
}

export function statementDecorations(statement: SyntaxNode): SyntaxNode[] {
  return statement.childNodes().filter((child) => child.kind === "Decoration");
}

export function objectListObjects(list: SyntaxNode): SyntaxNode[] {
  return list.childNodes().filter((child) => child.kind === "Object");
}

/**
 * Decorations attached to an object: the Decoration siblings after it in
 * the ObjectList, up to the next Object.
 */
export function objectDecorations(object: SyntaxNode): SyntaxNode[] {
  const parent = object.parent;
  if (parent === null || parent.kind !== "ObjectList") {
    return [];
  }
  let seenObject = false;
  const decorations: SyntaxNode[] = [];
  for (const child of parent.childNodes()) {
    if (!seenObject) {
      seenObject = child === object;
      continue;
    }
    if (child.kind === "Object") {
      break;
    }
    if (child.kind === "Decoration") {
      decorations.push(child);
    }
  }
  return decorations;
}

export function isDirectiveKind(node: SyntaxNode): boolean {
  return (
    node.kind === "Directive" || node.kind === "TargetDirective" || node.kind === "ProfileDirective"
  );
}

/**
 * The directive's value: the URI body for `@target`, or the first value
 * identifier for `@profile`. The directive name itself is an identifier
 * token, not a node, so it never matches here.
 */
export function directiveValue(directive: SyntaxNode): DirectiveValue | null {
  for (const child of directive.childNodes()) {
    if (child.kind === "Uri") {
      const text = child.toText();
      const stripped =
        text.startsWith("<") && text.endsWith(">") && text.length >= 2
          ? text.slice(1, -1)
          : text;
      return { value: stripped, span: nodeSpan(child) };
    }
    if (child.kind === "Identifier") {
      return { value: child.toText(), span: nodeSpan(child) };
    }
  }
  return null;
}

export function headerDirectives(root: SyntaxNode): HeaderDirectives {
  const header: HeaderDirectives = { profile: null, target: null, duplicates: [] };
  for (const node of root.descendants()) {
    if (!isDirectiveKind(node)) continue;
    let name: "profile" | "target";
    if (node.kind === "ProfileDirective") {
      name = "profile";
    } else if (node.kind === "TargetDirective") {
      name = "target";
    } else {
      continue;
    }
    const value = directiveValue(node);
    if (value === null) {
      continue;
    }
    if (header[name] === null) {
      header[name] = value;
    } else {
      header.duplicates.push([name, value.span]);
    }
  }
  return header;
}

export function tokensOf(node: SyntaxNode): SyntaxToken[] {
  const tokens: SyntaxToken[] = [];
  for (const element of node.descendantsWithTokens()) {
    if (element.type === "token") {
      tokens.push(element);
    }
  }
  return tokens;
}
