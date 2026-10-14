// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * 取りこぼしの期間が週の文字に出る（ST14 / tasks 10.2 / design D23（仮））。
 *
 * 見ているのは `tools/stack.sh` が立てた偽データ（`SEED=normal`。design D18）の稼働状況 —— アプリ利用に
 * 取得元に残っていない期間（2026-09-01 08:00〜2026-09-03 08:00）と、同じ週（2026-08-30〜）の
 * 時計のずれで諦めた期間（2026-09-04 09:00〜15:00）がある。偽データに無い重なり方
 * （同じ日に 2 理由・知らない理由）は応答を差し替えて出す（文字の組み立ては本物の画面）。
 */

const WEEK = "2026-08-30 の週";

type Cell = { day: string; gaps?: unknown[] } & Record<string, unknown>;
type Source = { logical_source: string; days: Cell[] } & Record<string, unknown>;

/** 稼働状況の応答のうち、アプリ利用の `2026-09-02` の日の取りこぼしだけを差し替える。 */
async function openWith(page: Page, gaps?: unknown[]): Promise<Locator> {
  if (gaps !== undefined) {
    await page.route("**/api/coverage?from=*", async (route) => {
      const res = await route.fetch();
      const body = (await res.json()) as Source[];
      for (const s of body) {
        if (s.logical_source !== "c01-app-usage") continue;
        const cell = s.days.find((d) => d.day === "2026-09-02");
        if (cell) cell.gaps = gaps;
      }
      await route.fulfill({ response: res, json: body });
    });
  }
  await page.goto("/#/coverage");
  const section = page.locator("section[data-source='c01-app-usage']");
  await expect(section).toBeVisible({ timeout: 15_000 });
  await section.getByRole("button", { name: "1 年ぶんを見る" }).click();
  await section.getByRole("button", { name: WEEK }).click();
  return section.getByTestId("week-detail");
}

const rowOf = (detail: Locator, day: string): Locator =>
  detail.locator("> div").filter({ has: detail.page().getByText(day, { exact: true }) });

// Scenario: 取得元に残っていない期間が週の文字に出る
test("取得元に残っていない期間は、状態の名前に添えて期間つきで出る", async ({ page }) => {
  const row = rowOf(await openWith(page), "2026-09-02");
  await expect(row).toContainText("動いていたが取れない状態だった");
  await expect(row).toContainText("取得元に残っていない（2026-09-01 08:00〜2026-09-03 08:00）");
});

// Scenario: 時計のずれで取れていない期間は別の文字で出る
test("時計のずれで取れていない期間は別の文字で出て、取得元の文字は出ない", async ({ page }) => {
  const row = rowOf(await openWith(page), "2026-09-04");
  await expect(row).toContainText("時計のずれで取れていない（2026-09-04 09:00〜2026-09-04 15:00）");
  await expect(row).not.toContainText("取得元に残っていない");
});

// Scenario: 2 つの理由が同じ日に重なると 2 つとも出る
test("理由の違う 2 つが同じ日に重なると、2 つとも 1 つずつ出る", async ({ page }) => {
  const detail = await openWith(page, [
    { reason: "retention", begin: "2026-08-31T23:00:00Z", end: "2026-09-02T23:00:00Z" },
    { reason: "clock_skew_abandoned", begin: "2026-09-02T01:00:00Z", end: "2026-09-02T05:00:00Z" },
  ]);
  const row = rowOf(detail, "2026-09-02");
  await expect(row.locator("[data-gap-note]")).toHaveCount(2);
  await expect(row.locator("[data-gap-note]").filter({ hasText: "取得元に残っていない（" })).toHaveCount(1);
  await expect(row.locator("[data-gap-note]").filter({ hasText: "時計のずれで取れていない（" })).toHaveCount(1);
});

// Scenario: 知らない理由の取りこぼしは理由の文字を付けずに出る
test("知らない理由は「取りこぼし（期間）」で、理由の文字を付けない", async ({ page }) => {
  const detail = await openWith(page, [
    { reason: "unknown_reason", begin: "2026-08-31T23:00:00Z", end: "2026-09-02T23:00:00Z" },
  ]);
  const row = rowOf(detail, "2026-09-02");
  await expect(row.locator("[data-gap-note]")).toHaveText([" 取りこぼし（2026-09-01 08:00〜2026-09-03 08:00）"]);
  await expect(row).not.toContainText("取得元に残っていない");
  await expect(row).not.toContainText("時計のずれで取れていない");
});
