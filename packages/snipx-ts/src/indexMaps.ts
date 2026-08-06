/**
 * Index maps between the three offset units that snipx cares about:
 *
 * - UTF-16 code units (native JS string indices),
 * - Unicode scalar values (what the canonical JSON contract uses for
 *   visible-text spans),
 * - UTF-8 bytes (what the canonical JSON contract uses for source spans,
 *   matching the Rust reference implementation's `&str` byte offsets).
 *
 * Every span that crosses the export boundary must be converted through
 * one of these maps; no raw `.length`/`.indexOf` offset may leak into
 * output.
 */

export interface IndexMap {
  /** Convert a UTF-16 code-unit index into a UTF-8 byte offset. */
  utf16ToUtf8(index: number): number;
  /** Convert a UTF-16 code-unit index into a Unicode scalar index. */
  utf16ToScalar(index: number): number;
  /** Convert a Unicode scalar index into a UTF-16 code-unit index. */
  scalarToUtf16(index: number): number;
  /** Total counts for the whole string. */
  readonly utf16Length: number;
  readonly utf8Length: number;
  readonly scalarLength: number;
}

export function utf8LengthOfCodePoint(codePoint: number): number {
  if (codePoint < 0x80) return 1;
  if (codePoint < 0x800) return 2;
  if (codePoint < 0x10000) return 3;
  return 4;
}

/**
 * Build an index map for `text`. Lookup tables are dense over UTF-16
 * indices (length + 1 entries), so conversions are O(1). Indices that
 * fall inside a surrogate pair map to the offsets of the code point
 * they interrupt; well-formed callers only query scalar boundaries.
 */
export function buildIndexMap(text: string): IndexMap {
  const utf16Length = text.length;
  const toUtf8 = new Array<number>(utf16Length + 1);
  const toScalar = new Array<number>(utf16Length + 1);
  const scalarToU16: number[] = [];

  let u8 = 0;
  let scalar = 0;
  let i = 0;
  while (i < utf16Length) {
    const codePoint = text.codePointAt(i);
    if (codePoint === undefined) break;
    const units = codePoint > 0xffff ? 2 : 1;
    for (let k = 0; k < units; k += 1) {
      toUtf8[i + k] = u8;
      toScalar[i + k] = scalar;
    }
    scalarToU16.push(i);
    u8 += utf8LengthOfCodePoint(codePoint);
    scalar += 1;
    i += units;
  }
  toUtf8[utf16Length] = u8;
  toScalar[utf16Length] = scalar;
  scalarToU16.push(utf16Length);

  return {
    utf16Length,
    utf8Length: u8,
    scalarLength: scalar,
    utf16ToUtf8(index: number): number {
      const clamped = Math.max(0, Math.min(index, utf16Length));
      const mapped = toUtf8[clamped];
      if (mapped === undefined) {
        throw new Error(`utf16ToUtf8: index ${index} out of range`);
      }
      return mapped;
    },
    utf16ToScalar(index: number): number {
      const clamped = Math.max(0, Math.min(index, utf16Length));
      const mapped = toScalar[clamped];
      if (mapped === undefined) {
        throw new Error(`utf16ToScalar: index ${index} out of range`);
      }
      return mapped;
    },
    scalarToUtf16(index: number): number {
      const clamped = Math.max(0, Math.min(index, scalarToU16.length - 1));
      const mapped = scalarToU16[clamped];
      if (mapped === undefined) {
        throw new Error(`scalarToUtf16: index ${index} out of range`);
      }
      return mapped;
    },
  };
}

/**
 * Unicode White_Space, mirroring Rust's `char::is_whitespace`. This is
 * deliberately not the JS `\s` class: `\s` includes U+FEFF (not
 * White_Space) and omits U+0085 (which is).
 */
export function isWhitespace(ch: string): boolean {
  if (ch.length === 0) return false;
  const cp = ch.codePointAt(0);
  if (cp === undefined) return false;
  return (
    (cp >= 0x09 && cp <= 0x0d) ||
    cp === 0x20 ||
    cp === 0x85 ||
    cp === 0xa0 ||
    cp === 0x1680 ||
    (cp >= 0x2000 && cp <= 0x200a) ||
    cp === 0x2028 ||
    cp === 0x2029 ||
    cp === 0x202f ||
    cp === 0x205f ||
    cp === 0x3000
  );
}
