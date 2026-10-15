// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * 録画専用・後半（人間が後から動画で見る）。ST07「PC のアクティブウィンドウを集める」。
 * 前半（`st07-desktop.ps1`）は Windows Sandbox の中でメモ帳 → ページ A → ページ B → メモ帳と前景を切り替え、
 * 収集アプリが積んだ記録を `tools/record-env.sh after-windows` がサーバへ送った。ここではそれを今日の 1 日の画面で見せる:
 * 今日を開く → 前半の時間の滞在を開く → PC の記録（畳んだ行も開く）にメモ帳とページ A / B の題名が出る。
 *
 * 滞在は after-windows が前半の時間に置いた位置（端末 rec-phone）から作られる。1 日の画面は PC の記録を滞在の中に出すため。
 * 前半と後半はハーネスが 1 本の動画につなぐ。
 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const TITLES = ["メモ A", "ページ A", "ページ B"];

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

/** 滞在を開き、中の畳んだ行を全部開く。 */
async function openAll(stay: Locator): Promise<void> {
  await stay.getByRole("button", { expanded: false }).first().click();
  await expect(stay.getByTestId("stay-detail")).toBeVisible();
  await expect(stay.getByText("記録を読み込み中…")).toHaveCount(0);
  const folds = stay.getByTestId("records").getByRole("button", { expanded: false });
  for (let i = 0; i < 20 && (await folds.count()) > 0; i++) {
    await folds.first().click();
  }
}

test("ST07 録画: Sandbox で切り替えた窓が、1 日の画面に PC の記録として並ぶ", async ({ page }) => {
  test.setTimeout(120_000);

  await test.step("1. 今日の 1 日の画面を開く", async () => {
    await page.goto("/");
    await page.reload();
    await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
    await expect(page.getByTestId("day-error")).toHaveCount(0);
    await expect(page.getByTestId("day-view")).toBeVisible();
    await hold(page, 2);
  });

  // 今日の滞在は after-windows が前半の時間に置いた 1 つだけ（偽データの滞在は 2026-09-07）
  const stay = page.getByTestId("row-stay").last();

  await test.step("2. 前半の時間の滞在を開く（中に PC の記録が時刻順に並ぶ。同じアプリの連続は畳んである）", async () => {
    await expect(page.getByTestId("row-stay")).not.toHaveCount(0, { timeout: 15_000 });
    await stay.scrollIntoViewIfNeeded();
    await openAll(stay);
    await hold(page, 2);
  });

  await test.step("3. メモ帳・ページ A・ページ B の題名が、PC の記録として出ている", async () => {
    const records = stay.getByTestId("stay-detail");
    for (const title of TITLES) {
      const row = records.getByText(title).first();
      await expect(row, `「${title}」の記録が無い`).toBeVisible();
      await row.scrollIntoViewIfNeeded();
      await hold(page);
    }
    await expect(records.getByText(/^PC/).first()).toBeVisible();
    await hold(page, 2);
  });
});
