// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * 録画専用・後半（人間が後から動画で見る）。ST08「PC のブラウザ履歴を集める」。
 * 前半（`st08-desktop.ps1`）は Windows Sandbox の中でページ A → B を見て、ページ C を新しいタブで開いてすぐ閉じ、
 * その後に起こした収集アプリが初回の取得で訪問を 1 件ずつ積んだ。`tools/record-env.sh after-windows` がそれをサーバへ送った。
 * ここでは今日の 1 日の画面で、前半の時間の滞在の中に**ブラウザの訪問**として A・B・C が出ることを見せる
 * （C は一瞬しか開いていないが、履歴には 1 件として残る）。
 *
 * 滞在は after-windows が前半の時間に置いた位置（端末 rec-phone）から作られる。前半と後半はハーネスが 1 本の動画につなぐ。
 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const VISITS = ["ページ A", "ページ B", "ページ C"];

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

async function openAll(stay: Locator): Promise<void> {
  await stay.getByRole("button", { expanded: false }).first().click();
  await expect(stay.getByTestId("stay-detail")).toBeVisible();
  await expect(stay.getByText("記録を読み込み中…")).toHaveCount(0);
  const folds = stay.getByTestId("records").getByRole("button", { expanded: false });
  for (let i = 0; i < 20 && (await folds.count()) > 0; i++) {
    await folds.first().click();
  }
}

test("ST08 録画: Sandbox で見たページが、1 日の画面にブラウザの訪問として 1 件ずつ並ぶ", async ({ page }) => {
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

  await test.step("2. 前半の時間の滞在: 要約にブラウザの訪問の件数が出る", async () => {
    await expect(page.getByTestId("row-stay")).not.toHaveCount(0, { timeout: 15_000 });
    await stay.scrollIntoViewIfNeeded();
    await expect(stay.getByText(/ブラウザ/).first()).toBeVisible();
    await hold(page, 2);
  });

  await test.step("3. 滞在を開くと、ページ A・B・C の訪問が 1 件ずつ出る（C は一瞬開いただけ）", async () => {
    await openAll(stay);
    const records = stay.getByTestId("stay-detail");
    for (const title of VISITS) {
      const row = records.getByText(new RegExp(`ブラウザ.*${title}`)).first();
      await expect(row, `「${title}」の訪問が無い`).toBeVisible();
      await row.scrollIntoViewIfNeeded();
      await hold(page);
    }
    await hold(page, 2);
  });
});
