// SPDX-License-Identifier: AGPL-3.0-only
import { getConfig } from "@testing-library/dom";
import { describe, expect, test } from "vitest";
import vitestConfigSource from "../../vitest.config.ts?raw";

describe("画面の単体試験の上限", () => {
  // Scenario: 画面の単体試験の待ちの上限は 10 秒、試験の上限は 30 秒
  test("待ちと試験の上限を設定している", () => {
    expect(getConfig().asyncUtilTimeout).toBe(10_000);
    expect(vitestConfigSource).toMatch(/testTimeout:\s*30_000/);
  });
});
