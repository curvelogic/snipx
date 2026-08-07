import { describe, expect, it } from "vitest";

import { buildIndexMap, isWhitespace, utf8LengthOfCodePoint } from "../src/indexMaps.js";

describe("utf8LengthOfCodePoint", () => {
  it("covers all four widths", () => {
    expect(utf8LengthOfCodePoint(0x41)).toBe(1); // A
    expect(utf8LengthOfCodePoint(0xe9)).toBe(2); // é
    expect(utf8LengthOfCodePoint(0x20ac)).toBe(3); // €
    expect(utf8LengthOfCodePoint(0x1f600)).toBe(4); // 😀
  });
});

describe("buildIndexMap", () => {
  it("is the identity on ASCII", () => {
    const map = buildIndexMap("hello");
    expect(map.utf16Length).toBe(5);
    expect(map.utf8Length).toBe(5);
    expect(map.scalarLength).toBe(5);
    for (let i = 0; i <= 5; i += 1) {
      expect(map.utf16ToUtf8(i)).toBe(i);
      expect(map.utf16ToScalar(i)).toBe(i);
      expect(map.scalarToUtf16(i)).toBe(i);
    }
  });

  it("maps astral-plane characters (surrogate pairs, 4-byte UTF-8)", () => {
    // "a😀b": UTF-16 [a, hi, lo, b]; UTF-8 [a, 4 bytes, b]; scalars [a, 😀, b].
    const map = buildIndexMap("a\u{1f600}b");
    expect(map.utf16Length).toBe(4);
    expect(map.utf8Length).toBe(6);
    expect(map.scalarLength).toBe(3);
    expect(map.utf16ToUtf8(0)).toBe(0);
    expect(map.utf16ToUtf8(1)).toBe(1);
    expect(map.utf16ToUtf8(3)).toBe(5);
    expect(map.utf16ToUtf8(4)).toBe(6);
    expect(map.utf16ToScalar(1)).toBe(1);
    expect(map.utf16ToScalar(3)).toBe(2);
    expect(map.scalarToUtf16(1)).toBe(1);
    expect(map.scalarToUtf16(2)).toBe(3);
    expect(map.scalarToUtf16(3)).toBe(4);
  });

  it("maps combining sequences scalar by scalar", () => {
    // "e" + COMBINING ACUTE (U+0301, 2 UTF-8 bytes): two scalars.
    const map = buildIndexMap("éx");
    expect(map.utf16Length).toBe(3);
    expect(map.utf8Length).toBe(4);
    expect(map.scalarLength).toBe(3);
    expect(map.utf16ToUtf8(1)).toBe(1);
    expect(map.utf16ToUtf8(2)).toBe(3);
    expect(map.utf16ToUtf8(3)).toBe(4);
  });

  it("maps three-byte BMP characters", () => {
    const map = buildIndexMap("€x");
    expect(map.utf16Length).toBe(2);
    expect(map.utf8Length).toBe(4);
    expect(map.utf16ToUtf8(1)).toBe(3);
    expect(map.utf16ToUtf8(2)).toBe(4);
  });

  it("clamps out-of-range queries to the ends", () => {
    const map = buildIndexMap("ab");
    expect(map.utf16ToUtf8(99)).toBe(2);
    expect(map.utf16ToUtf8(-1)).toBe(0);
  });
});

describe("isWhitespace", () => {
  it("mirrors Rust char::is_whitespace, not JS \\s", () => {
    expect(isWhitespace(" ")).toBe(true);
    expect(isWhitespace("\t")).toBe(true);
    expect(isWhitespace("\n")).toBe(true);
    expect(isWhitespace("")).toBe(true); // NEL: White_Space, not in \s
    expect(isWhitespace(" ")).toBe(true); // NBSP
    expect(isWhitespace(" ")).toBe(true);
    expect(isWhitespace("　")).toBe(true);
    expect(isWhitespace("﻿")).toBe(false); // BOM: in \s, not White_Space
    expect(isWhitespace("​")).toBe(false); // zero-width space
    expect(isWhitespace("a")).toBe(false);
  });
});
