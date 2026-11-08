import { readFileSync } from "node:fs";
import { describe, expect, test } from "vitest";
import { buildSetupString, parseSetupString } from "../collectorSetup";

type Vector = { name: string; input: string; output: { url: string; token: string } | null };
const vectors = JSON.parse(readFileSync("../docs/contracts/collector-setup-vectors.json", "utf8")) as {
  valid: Vector[];
  invalid: Vector[];
};

describe("collector setup string", () => {
  test("builds the shared valid setup string", () => {
    const expected = vectors.valid[0];
    expect(buildSetupString(expected.output!.url, expected.output!.token)).toBe(expected.input);
  });

  test("parses every shared vector", () => {
    for (const vector of [...vectors.valid, ...vectors.invalid]) {
      expect(parseSetupString(vector.input), vector.name).toEqual(vector.output);
    }
  });
});
