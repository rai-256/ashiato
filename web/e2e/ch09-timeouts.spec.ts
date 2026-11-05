// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test } from "@playwright/test";
import playwrightConfig from "../playwright.config";

// Scenario: 画面の e2e の待ちの上限は 30 秒、試験の上限は 120 秒
test("画面の待ちと試験の上限が設定されている", () => {
  expect(playwrightConfig.expect?.timeout).toBe(30_000);
  expect(test.info().timeout).toBeGreaterThanOrEqual(120_000);
});
