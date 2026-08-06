/**
 * Conformance corpus runner, mirroring
 * crates/snipx-core/tests/conformance.rs.
 *
 * Executes every case under conformance/cases/** against exportJson and
 * compares the result to expected.json under the contract declared in
 * conformance/MANIFEST.json: structural comparison, the implementation
 * block excluded, diagnostic codes normative but message strings
 * informative, facts/resolutions order-sensitive and diagnostics an
 * order-insensitive multiset, object keys canonically sorted.
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import {
  exportJson,
  profileFromName,
  SPEC_VERSION,
  type ExportRequest,
  type InputForm,
  type Value,
} from "../src/index.js";

const corpusRoot = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..", "..", "conformance");

/**
 * Cases whose behaviour genuinely diverges between the CommonMark
 * engines. Empty is the goal; any entry must reference a documented
 * divergence proposed as a spec ruling in the PR that added it.
 */
const SKIP_LIST: ReadonlyMap<string, string> = new Map<string, string>([]);

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

function readJson(path: string): Json {
  return JSON.parse(readFileSync(path, "utf8")) as Json;
}

/** Discover case directories (any directory containing request.json) in stable path order. */
function discoverCases(casesDir: string): string[] {
  const found: string[] = [];
  const stack = [casesDir];
  while (stack.length > 0) {
    const dir = stack.pop();
    if (dir === undefined) break;
    const entries = readdirSync(dir)
      .map((name) => join(dir, name))
      .filter((path) => statSync(path).isDirectory())
      .sort();
    for (const entry of entries) {
      let isCase = false;
      try {
        isCase = statSync(join(entry, "request.json")).isFile();
      } catch {
        isCase = false;
      }
      if (isCase) {
        found.push(entry);
      } else {
        stack.push(entry);
      }
    }
  }
  found.sort();
  return found;
}

function stringField(
  object: { [key: string]: Json },
  key: string,
  caseName: string,
): string | undefined {
  const value = object[key];
  if (value === undefined || value === null) return undefined;
  if (typeof value !== "string") {
    throw new Error(`${caseName}: field ${key} must be a string`);
  }
  return value;
}

function parseAmbientSubject(value: Json, caseName: string): Value {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${caseName}: ambientSubject must be an object`);
  }
  const kind = value["kind"];
  if (typeof kind !== "string") {
    throw new Error(`${caseName}: ambientSubject needs a string kind`);
  }
  const field = (key: string): Json => {
    const inner = value[key];
    if (inner === undefined) {
      throw new Error(`${caseName}: ambientSubject missing ${key}`);
    }
    return inner;
  };
  switch (kind) {
    case "name":
      return { type: "name", value: field("value") as string };
    case "string":
      return { type: "string", value: field("value") as string };
    case "uri":
      return { type: "uri", value: field("value") as string };
    case "number":
      return { type: "number", value: field("value") as number };
    case "boolean":
      return { type: "boolean", value: field("value") as boolean };
    default:
      throw new Error(`${caseName}: unsupported ambientSubject kind ${kind}`);
  }
}

const KNOWN_FIELDS = new Set([
  "source",
  "inputForm",
  "targetText",
  "targetFile",
  "profile",
  "path",
  "targetUri",
  "ambientSubject",
]);

function parseRequest(caseDir: string): ExportRequest {
  const path = join(caseDir, "request.json");
  const value = readJson(path);
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${path}: request must be an object`);
  }
  for (const key of Object.keys(value)) {
    if (!KNOWN_FIELDS.has(key)) {
      throw new Error(`${path}: unknown request field ${key}`);
    }
  }

  const source = stringField(value, "source", path);
  if (source === undefined) {
    throw new Error(`${path}: request needs a source`);
  }
  const inputFormRaw = stringField(value, "inputForm", path);
  if (
    inputFormRaw !== "commentaria" &&
    inputFormRaw !== "marginalia" &&
    inputFormRaw !== "intralinea"
  ) {
    throw new Error(`${path}: invalid inputForm ${inputFormRaw ?? "(missing)"}`);
  }
  const inputForm: InputForm = inputFormRaw;

  const targetTextField = stringField(value, "targetText", path);
  const targetFileField = stringField(value, "targetFile", path);
  if (targetTextField !== undefined && targetFileField !== undefined) {
    throw new Error(`${path}: targetText and targetFile are mutually exclusive`);
  }
  const targetText =
    targetTextField !== undefined
      ? targetTextField
      : targetFileField !== undefined
        ? readFileSync(join(caseDir, targetFileField), "utf8")
        : undefined;

  const profileName = stringField(value, "profile", path);
  let profile;
  if (profileName !== undefined) {
    const named = profileFromName(profileName);
    if (named === null) {
      throw new Error(`${path}: unknown profile ${profileName}`);
    }
    profile = named;
  }

  const ambient = value["ambientSubject"];

  const request: ExportRequest = { source, inputForm };
  if (targetText !== undefined) request.targetText = targetText;
  if (profile !== undefined) request.profile = profile;
  const pathField = stringField(value, "path", path);
  if (pathField !== undefined) request.path = pathField;
  const targetUri = stringField(value, "targetUri", path);
  if (targetUri !== undefined) request.targetUri = targetUri;
  if (ambient !== undefined && ambient !== null) {
    request.ambientSubject = parseAmbientSubject(ambient, path);
  }
  return request;
}

/** Rebuild a value with object keys in sorted order, recursively. */
function sortKeys(value: Json): Json {
  if (Array.isArray(value)) {
    return value.map(sortKeys);
  }
  if (value !== null && typeof value === "object") {
    const sorted: { [key: string]: Json } = {};
    for (const key of Object.keys(value).sort()) {
      const inner = value[key];
      if (inner !== undefined) {
        sorted[key] = sortKeys(inner);
      }
    }
    return sorted;
  }
  return value;
}

/** Compact canonical serialisation of an already key-sorted value. */
function canonical(value: Json): string {
  return JSON.stringify(value);
}

/**
 * Reduce a document to its comparable form per the MANIFEST contract:
 * drop the implementation block, drop informative message fields, and
 * sort diagnostics into a canonical order.
 */
function comparable(document: Json): Json {
  const sorted = sortKeys(document);
  if (sorted === null || typeof sorted !== "object" || Array.isArray(sorted)) {
    throw new Error("export document must be an object");
  }
  delete sorted["implementation"];
  const diagnostics = sorted["diagnostics"];
  if (Array.isArray(diagnostics)) {
    for (const diagnostic of diagnostics) {
      if (diagnostic === null || typeof diagnostic !== "object" || Array.isArray(diagnostic)) {
        throw new Error("diagnostic must be an object");
      }
      delete diagnostic["message"];
      const related = diagnostic["related"];
      if (Array.isArray(related)) {
        for (const entry of related) {
          if (entry !== null && typeof entry === "object" && !Array.isArray(entry)) {
            delete entry["message"];
          }
        }
      }
    }
    diagnostics.sort((a, b) => {
      const left = canonical(a);
      const right = canonical(b);
      return left < right ? -1 : left > right ? 1 : 0;
    });
  }
  return sorted;
}

/** Serialise the actual export document, stripping the implementation block. */
function storedForm(request: ExportRequest): Json {
  const document = JSON.parse(JSON.stringify(exportJson(request))) as Json;
  if (document === null || typeof document !== "object" || Array.isArray(document)) {
    throw new Error("export document must be an object");
  }
  delete document["implementation"];
  return sortKeys(document);
}

describe("conformance corpus", () => {
  const manifest = readJson(join(corpusRoot, "MANIFEST.json"));
  if (manifest === null || typeof manifest !== "object" || Array.isArray(manifest)) {
    throw new Error("MANIFEST.json must be an object");
  }

  it("matches the implementation's SPEC_VERSION", () => {
    expect(manifest["specVersion"]).toBe(SPEC_VERSION);
  });

  const cases = discoverCases(join(corpusRoot, "cases"));

  it("discovers exactly the MANIFEST caseCount", () => {
    expect(cases.length).toBe(manifest["caseCount"]);
  });

  for (const caseDir of cases) {
    const name = relative(join(corpusRoot, "cases"), caseDir);
    const skipReason = SKIP_LIST.get(name);
    const definition = skipReason === undefined ? it : it.skip;
    definition(name, () => {
      const actual = storedForm(parseRequest(caseDir));
      const expected = readJson(join(caseDir, "expected.json"));
      expect(comparable(actual)).toEqual(comparable(expected));
    });
  }
});
