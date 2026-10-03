// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST22「記録の削除」を**スマホの幅・ダーク表示**で撮る:
 * 1 日の並び → 行を開く → 「この滞在を消す」→ 確認 → 「消す」→ 消した行 → 「戻す」で元に戻る。
 *
 * 画面はスマホから見ることが多い（`tailscale serve` 越し）。PC の幅・ライトの録画（`st22-erase-reload.rec.ts`）と
 * 同じ操作を、押す場所の大きさ・文字の見え方が分かる形でもう 1 本撮る。
 * アサーションは `e2e/day-erase.spec.ts` の「確認して消すと…」と同じもの。足したのは見るための短い停止（HOLD）だけ。
 */
declare const process: { env: Record<string, string | undefined> };

const DAY = "2026-09-07";
const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const PHONE = { width: 390, height: 844 };

test.use({
  viewport: PHONE,
  colorScheme: "dark",
  hasTouch: true,
  isMobile: true,
  deviceScaleFactor: 2,
  video: {
    mode: "on",
    size: PHONE,
    show: {
      actions: { position: "top-right", cursor: "pointer", fontSize: 14, duration: 1500 },
      test: { level: "step", position: "bottom-left", fontSize: 14 },
    },
  },
});

async function hold(page: Page): Promise<void> {
  await page.waitForTimeout(HOLD);
}

test("ST22 録画（スマホ幅・ダーク）: 消す → 消した行 → 戻す", async ({ page, browser, browserName }) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });

  const row = page.getByTestId("row-stay").first();
  const range = await test.step(`1. ${DAY} の 1 日の並び（390 px・ダーク）`, async () => {
    await page.goto(`/#/day/${DAY}`);
    await page.reload();
    await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
    await expect(page.getByTestId("day-error")).toHaveCount(0);
    await expect(row).toBeVisible();
    // 横にはみ出していない（スマホで横スクロールが出ない）
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(PHONE.width);
    await hold(page);
    return (await row.locator("h2").innerText()).trim();
  });

  await test.step(`2. ${range} を開いて「この滞在を消す」→ 確認 →「消す」`, async () => {
    await row.getByRole("button").first().tap();
    await expect(row.getByTestId("stay-detail")).toBeVisible();
    await row.getByRole("button", { name: "この滞在を消す" }).scrollIntoViewIfNeeded();
    await hold(page);
    await row.getByRole("button", { name: "この滞在を消す" }).tap();
    await expect(row.getByTestId("erase-confirm")).toBeVisible();
    await hold(page);
    await row.getByRole("button", { name: "消す" }).tap();
  });

  const erased = page.getByTestId("row-erased").filter({ hasText: range });
  let restored = false;
  try {
    await test.step(`3. ${range} が「消した」の行になった`, async () => {
      await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(0);
      await expect(erased).toHaveCount(1);
      await expect(erased).toContainText("消した");
      await erased.scrollIntoViewIfNeeded();
      await hold(page);
    });

    await test.step(`4. 「戻す」で ${range} が滞在の行に戻る`, async () => {
      await erased.getByRole("button", { name: "戻す" }).tap();
      restored = true;
      await expect(erased).toHaveCount(0);
      await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(1);
      await hold(page);
    });
  } finally {
    if (!restored) {
      await erased.getByRole("button", { name: "戻す" }).click({ timeout: 5_000 }).catch(() => undefined);
    }
  }
});
