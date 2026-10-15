// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST14 の稼働状況の画面の変わったところを撮る:
 * 格子の並び（Must の 5 本 → アプリ利用の集計 → 書庫）→ 集計の格子は開いた直後から直近 4 週 →
 * アプリ利用の 2026-08-30 の週を開く → 取りこぼしの期間が理由ごとに別の文字で出る → OS をダークにすると稼働状況もダーク。
 *
 * 偽データは `tools/record-env.sh` が立てる `SEED=normal`（`tools/seed.sh` の ST14 の段）: アプリ利用に
 * 取得元に残っていない期間（2026-09-01 08:00〜09-03 08:00）と時計のずれで諦めた期間（2026-09-04 09:00〜15:00）、
 * 集計 3 件。**応答の差し替え（route）は使わない**（同じ日に 2 理由・知らない理由は `e2e/coverage-gaps.spec.ts` が担保する）。
 *
 * アサーションは `e2e/coverage-rollup.spec.ts`（並び・直近 4 週）・`e2e/coverage-gaps.spec.ts`（取りこぼしの文字）・
 * `e2e/coverage-scheme.spec.ts`（明暗の追従）の主張の写し。見るための停止（HOLD）だけを足した。
 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const MUST = ["c01-location", "c01-app-usage", "c01-photo", "c02-window", "c02-browser-history"];
const ROLLUP = "c01-app-usage-rollup";
const WEEK = "2026-08-30 の週";

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

const sourcesInOrder = (page: Page): Promise<string[]> =>
  page.locator("section[data-source]").evaluateAll((els) => els.map((e) => e.getAttribute("data-source") ?? ""));

test("ST14 録画: 格子の並び → 集計の直近 4 週 → 取りこぼしの期間の文字 → ダーク", async ({ page, browser, browserName }) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  // 見るための停止と操作の間隔（slowMo）で、既定の 30 秒を使い切る
  test.setTimeout(120_000);

  await test.step("1. 稼働状況: 格子は上から Must の 5 ソース・アプリ利用の集計・書庫のソースの順", async () => {
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/#/coverage");
    await page.reload();
    await expect(page.getByTestId("coverage-loading")).toBeHidden({ timeout: 15_000 });
    await expect(page.locator("[data-role=grid]").nth(5)).toBeVisible({ timeout: 15_000 });
    const order = await sourcesInOrder(page);
    expect(order.slice(0, 5)).toEqual(MUST);
    expect(order[5]).toBe(ROLLUP);
    const archives = order.slice(6);
    expect(archives.length, "書庫のソースが 1 本も無い").toBeGreaterThan(0);
    expect(archives.every((s) => s.startsWith("c03-")), `集計の後ろが書庫のソースだけではない: ${archives}`).toBe(true);
    await hold(page, 2);
  });

  await test.step("2. アプリ利用の集計の格子: 押さなくても直近 4 週の行が出ている", async () => {
    const section = page.locator(`section[data-source='${ROLLUP}']`);
    await section.scrollIntoViewIfNeeded();
    const grid = section.locator("[data-role=grid]");
    await expect(grid).toHaveAttribute("data-weeks", "4");
    await expect(grid.locator("[data-week]")).toHaveCount(4);
    for (const row of await grid.locator("[data-week]").all()) {
      const box = (await row.boundingBox())!;
      expect(box.height, "行が描かれていない").toBeGreaterThan(0);
    }
    await hold(page, 3);
  });

  const section = page.locator("section[data-source='c01-app-usage']");
  const detail = section.getByTestId("week-detail");
  const rowOf = (day: string) => detail.locator("> div").filter({ has: page.getByText(day, { exact: true }) });

  await test.step(`3. アプリ利用: 1 年ぶんを出して「${WEEK}」を開く`, async () => {
    await section.scrollIntoViewIfNeeded();
    await hold(page);
    await section.getByRole("button", { name: "1 年ぶんを見る" }).click();
    await section.getByRole("button", { name: WEEK }).click();
    await expect(detail).toBeVisible();
    await detail.scrollIntoViewIfNeeded();
    await hold(page, 2);
  });

  await test.step("4. 2026-09-02: 取得元に残っていない期間が、状態の名前に添えて期間つきで出る", async () => {
    const row = rowOf("2026-09-02");
    await expect(row).toContainText("動いていたが取れない状態だった");
    await expect(row).toContainText("取得元に残っていない（2026-09-01 08:00〜2026-09-03 08:00）");
    await row.scrollIntoViewIfNeeded();
    await hold(page, 3);
  });

  await test.step("5. 2026-09-04: 時計のずれで取れていない期間は別の文字で出て、取得元の文字は出ない", async () => {
    const row = rowOf("2026-09-04");
    await expect(row).toContainText("時計のずれで取れていない（2026-09-04 09:00〜2026-09-04 15:00）");
    await expect(row).not.toContainText("取得元に残っていない");
    await row.scrollIntoViewIfNeeded();
    await hold(page, 3);
  });

  await test.step("6. OS をダークにすると、稼働状況もダークで出る", async () => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.reload();
    await expect(page.getByTestId("coverage-loading")).toBeHidden({ timeout: 15_000 });
    await expect(page.getByTestId("coverage-view")).toHaveAttribute("data-scheme", "dark");
    await expect(page.locator("[data-role=grid]").first()).toBeVisible({ timeout: 15_000 });
    await hold(page, 3);
  });
});
