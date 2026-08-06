import { describe, expect, it } from "vitest";

import { normalize } from "../src/match.js";

function text(result: ReturnType<typeof normalize>): string {
  return result.chars.join("");
}

describe("normalize (strict)", () => {
  it("applies NFC and maps scalar offsets one-to-one", () => {
    // e + combining acute composes to é under NFC.
    const result = normalize("éf", false);
    expect(text(result)).toBe("éf");
    expect(result.starts).toEqual([0, 1]);
    expect(result.ends).toEqual([1, 2]);
  });

  it("keeps whitespace verbatim", () => {
    const result = normalize("a  b", false);
    expect(text(result)).toBe("a  b");
    expect(result.starts).toEqual([0, 1, 2, 3]);
  });
});

describe("normalize (loose folding table)", () => {
  it("collapses whitespace runs to a single space with run-wide offsets", () => {
    const result = normalize("a \t\n b", true);
    expect(text(result)).toBe("a b");
    expect(result.starts).toEqual([0, 1, 5]);
    expect(result.ends).toEqual([1, 5, 6]);
  });

  it("records trailing whitespace runs", () => {
    const result = normalize("a  ", true);
    expect(text(result)).toBe("a ");
    expect(result.starts).toEqual([0, 1]);
    expect(result.ends).toEqual([1, 3]);
  });

  it("folds dashes to hyphen-minus", () => {
    for (const dash of ["‐", "‑", "‒", "–", "—", "−"]) {
      expect(text(normalize(dash, true))).toBe("-");
    }
  });

  it("folds curly single quotes to apostrophe", () => {
    for (const quote of ["‘", "’", "‚", "‛"]) {
      expect(text(normalize(quote, true))).toBe("'");
    }
  });

  it("folds curly double quotes to straight quote", () => {
    for (const quote of ["“", "”", "„", "‟"]) {
      expect(text(normalize(quote, true))).toBe('"');
    }
  });

  it("expands ligatures with every output scalar mapping to the ligature", () => {
    const cases: [string, string][] = [
      ["ﬀ", "ff"],
      ["ﬁ", "fi"],
      ["ﬂ", "fl"],
      ["ﬃ", "ffi"],
      ["ﬄ", "ffl"],
      ["ﬅ", "st"],
      ["ﬆ", "st"],
    ];
    for (const [input, expected] of cases) {
      const result = normalize(input, true);
      expect(text(result)).toBe(expected);
      expect(result.starts).toEqual(Array(expected.length).fill(0));
      expect(result.ends).toEqual(Array(expected.length).fill(1));
    }
  });

  it("does not fold in strict mode", () => {
    expect(text(normalize("—ﬁ", false))).toBe("—ﬁ");
  });
});
