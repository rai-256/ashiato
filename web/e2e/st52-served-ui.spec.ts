// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { expect, test } from "@playwright/test";

declare const process: { env: Record<string, string | undefined> };

test.use({ storageState: { cookies: [], origins: [] } });

test("組み込んだ画面をサーバから開ける", async ({ page }) => {
  // Scenario: 配ったサーバだけで、Node を入れずに画面が開く
  // サーバの待ち受けは playwright.config.ts が tools/ports.sh と同じ規則で組んで渡す（BIND の env に頼らない。R2）
  const bind = process.env.ASHIATO_E2E_SERVER_BIND;
  const password = process.env.WEB_PASSWORD;
  if (!bind || !password) throw new Error("ASHIATO_E2E_SERVER_BIND（playwright.config.ts が組む）と WEB_PASSWORD が必要");
  const origin = `http://${bind}`;
  const requested: string[] = [];
  page.on("request", (request) => requested.push(request.url()));

  await page.goto(`${origin}/#/coverage`);
  await expect(page.getByLabel("合言葉")).toBeVisible();
  await page.getByLabel("合言葉").fill(password);
  await page.getByRole("button", { name: "ログイン" }).click();
  await expect(page.getByTestId("coverage-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.locator('[data-testid^="grid-"]').first()).toBeVisible();

  expect(requested.length).toBeGreaterThan(0);
  expect(requested.every((url) => new URL(url).origin === origin)).toBe(true);
  expect(requested.some((url) => new URL(url).pathname === "/api/coverage")).toBe(true);
});
