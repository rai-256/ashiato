// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST12「書庫を置くだけで過去のデータが入る」を、稼働状況の画面で撮る:
 * 「直近に置いた書庫」の箱（形の確認待ちの書庫がある・直近の書庫は確認待ち）→ 取り込んだ書庫のソースの格子
 * （見出しに「〜まで（N 日前）」）→ その週を開く。
 *
 * 書庫は `tools/record-env.sh` の `archive_prepare` が録画の前に置く（この実行の一時の置き場）:
 *   rec-youtube.zip（YouTube の視聴履歴 2 件。2026-09-20 / 09-21（seed の書庫の最終日 09-16 より後））—— 形を本人の代わりに確認して取り込ませた
 *   rec-pending.zip（マイアクティビティの検索）—— 形の確認待ちのまま
 * 形の確認（`tools/archive-shape.sh`）はターミナルの操作で画面からは押せないので、この動画には入らない。
 * **本物の Google の書き出しは使わない**（合成の書庫は Google の実物の形の揺れを再現できない。それは人間の確認 13.1）。
 *
 * アサーションは `e2e/latest-archive.spec.ts` の箱の主張の写し（直近の書庫の行が失敗にならない）に、
 * 置いた 2 冊の結果と最終日の注記を足したもの。見るための停止（HOLD）だけを足した。
 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const YOUTUBE = "c03-youtube-watch";

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

test("ST12 録画: 直近に置いた書庫の箱 → 書庫のソースの格子と最終日", async ({ page, browser, browserName }) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  // 見るための停止と操作の間隔（slowMo）で、既定の 30 秒を使い切る（実測 2026-10-05: 手順 4 で時間切れ）
  test.setTimeout(120_000);

  const box = page.getByTestId("latest-archive");
  await test.step("1. 稼働状況: 達成の下に「直近に置いた書庫」の箱", async () => {
    await page.goto("/");
    await expect(box).toBeVisible({ timeout: 15_000 });
    await expect(box.getByTestId("archive-row-loading")).toHaveCount(0, { timeout: 15_000 });
    await expect(box.getByTestId("archive-row-failed")).toHaveCount(0);
    await box.scrollIntoViewIfNeeded();
    await hold(page);
  });

  await test.step("2. 形の確認を待っている書庫が 1 冊（rec-pending.zip）", async () => {
    await expect(box.getByTestId("archive-row-pending-shape")).toContainText("形の確認を待っている書庫が 1 冊");
    await expect(box.getByTestId("archive-row-latest")).toContainText("rec-pending.zip");
    await expect(box.getByTestId("archive-row-latest")).toContainText("形の確認を待っています");
    await hold(page, 3);
  });

  const section = page.locator(`[data-source="${YOUTUBE}"]`);
  await test.step("3. 取り込んだ書庫（rec-youtube.zip）のソースの格子: 見出しに最終日", async () => {
    await expect(section).toBeVisible({ timeout: 15_000 });
    await section.scrollIntoViewIfNeeded();
    await expect(page.getByTestId(`archive-note-${YOUTUBE}`)).toContainText("2026-09-21 まで");
    await hold(page, 3);
  });

  await test.step("4. 最終日（2026-09-21）の週を開く（その週の 7 日の状態）", async () => {
    const grid = page.getByTestId(`grid-${YOUTUBE}`);
    const day = grid.locator('[data-day="2026-09-21"]');
    // 直近の週だけが出ていてその日が無ければ、1 年ぶんを出す（e2e/coverage-year.spec.ts と同じ操作）
    if ((await day.count()) === 0) {
      await section.getByRole("button", { name: "1 年ぶんを見る" }).click();
    }
    await expect(day).toHaveCount(1);
    await grid.locator('button[data-week]:has([data-day="2026-09-21"])').click();
    await expect(page.getByTestId("week-detail")).toBeVisible();
    await page.getByTestId("week-detail").scrollIntoViewIfNeeded();
    await hold(page, 3);
  });
});
