// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * 録画専用・後半（人間が後から動画で見る）。ST50「送信契約の版」。
 * 前半（`st50-desktop.ps1`）は Windows Sandbox の中で、ST50 より古いサーバの間は送らずに PC に残し（「サーバを更新してください」の通知）、
 * 更新した後に通知が消えるところまでを撮った。残していた記録は `tools/record-env.sh after-windows` が録画用のサーバ（ST50 の版）へ送った。
 * ここではそれを今日の 1 日の画面で見せる: 今日を開く → 前半の時間の滞在を開く → PC の記録としてフォルダ「メモ ST50」が出る
 * （古いサーバの間に集めた記録が、捨てられずに更新後のサーバへ届いた）。
 *
 * 滞在は after-windows が前半の時間に置いた位置（端末 rec-phone）から作られる。1 日の画面は PC の記録を滞在の中に出すため。
 * 前半と後半はハーネスが 1 本の動画につなぐ。足場は `tools/recording/st07/st07-result.rec.ts` の写し。
 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const TITLES = ["メモ ST50"];

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

/** 滞在を開く（中に PC の記録が時刻順に並ぶ。同じアプリの連続は畳んである）。 */
async function openStay(stay: Locator): Promise<void> {
  await stay.getByRole("button", { expanded: false }).first().click();
  await expect(stay.getByTestId("stay-detail")).toBeVisible();
  await expect(stay.getByText("記録を読み込み中…")).toHaveCount(0);
}

/** 題名の行を出す。畳んだ束は 1 度に 1 つしか開かない作りなので、見えなければ束を 1 つずつ開いて探す。 */
async function reveal(stay: Locator, title: string): Promise<Locator> {
  const row = stay.getByTestId("stay-detail").getByText(title).first();
  if (await row.isVisible()) return row;
  const folds = stay.getByTestId("records").getByRole("button");
  const n = await folds.count();
  for (let i = 0; i < n; i++) {
    if ((await folds.nth(i).getAttribute("aria-expanded")) !== "true") await folds.nth(i).click();
    if (await row.isVisible()) return row;
  }
  return row;
}

test("ST50 録画: 古いサーバの間 PC に残した記録が、更新後のサーバへ届いて 1 日の画面に並ぶ", async ({ page }) => {
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
    await openStay(stay);
    await hold(page, 2);
  });

  await test.step("3. 古いサーバの間に集めたフォルダ「メモ ST50」の記録が、PC の記録として出ている（捨てられずに届いた）", async () => {
    const records = stay.getByTestId("stay-detail");
    for (const title of TITLES) {
      const row = await reveal(stay, title);
      await expect(row, `「${title}」の記録が無い`).toBeVisible();
      await row.scrollIntoViewIfNeeded();
      await hold(page);
    }
    await expect(records.getByText(/^PC/).first()).toBeVisible();
    await hold(page, 2);
  });
});
