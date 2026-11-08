// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { expect, test } from "@playwright/test";

/** 録画用に独立して置く ST56 の Web 設定画面。既存 e2e の相対 import は録画の写し先では使えない。 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);

async function openSettings(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/#/settings");
  await expect(page.getByTestId("collector-setup-section")).toBeVisible();
}

test("ST56 録画: 合言葉を入れ直して収集アプリの設定文字列を出す", async ({ page }) => {
  await test.step("1. 設定を開く（設定文字列はまだ出ていない）", async () => {
    await openSettings(page);
    const section = page.getByTestId("collector-setup-section");
    await expect(section.getByLabel("合言葉")).toBeVisible();
    await expect(section.getByTestId("collector-setup-value")).toHaveCount(0);
    await page.waitForTimeout(HOLD);
  });

  await test.step("2. 合言葉を入れ直すと、貼る設定文字列とコピーのボタンが出る", async () => {
    const section = page.getByTestId("collector-setup-section");
    const password = process.env.WEB_PASSWORD;
    if (!password) throw new Error("WEB_PASSWORD が無い");
    await section.getByLabel("合言葉").fill(password);
    await section.getByRole("button", { name: "表示" }).click();
    await expect(section.getByTestId("collector-setup-value")).toBeVisible();
    await expect(section.getByRole("button", { name: "値をコピー" })).toBeVisible();
    await expect(section.getByLabel("利用者")).toHaveCount(0);
    await page.waitForTimeout(HOLD * 2);
  });

  await test.step("3. 画面を開き直すと設定文字列は消える", async () => {
    await page.reload();
    const section = page.getByTestId("collector-setup-section");
    await expect(section.getByLabel("合言葉")).toBeVisible();
    await expect(section.getByTestId("collector-setup-value")).toHaveCount(0);
    await page.waitForTimeout(HOLD * 2);
  });
});
