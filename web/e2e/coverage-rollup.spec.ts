// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * アプリ利用の集計の格子（ST14 / tasks 10.3 / design D10（仮））。
 * 見ているのは `tools/stack.sh` が立てた偽データ（`SEED=normal`）。測り方は `archive-layout.spec.ts` と同じ。
 */

const BOX_MAX_PX = 160; // design D12（仮）。箱の高さの上限をリテラルで持つ
const PHONE = { width: 360, height: 640 };
const MUST = ["c01-location", "c01-app-usage", "c01-photo", "c02-window", "c02-browser-history"];

type Status = Record<string, unknown>;

async function open(page: Page, patch?: (base: Status) => Status): Promise<void> {
  await page.setViewportSize(PHONE);
  if (patch) {
    await page.route("**/api/archives/status**", async (route) => {
      const res = await route.fetch();
      await route.fulfill({ response: res, json: patch((await res.json()) as Status) });
    });
  }
  await page.goto("/#/coverage");
  const box = page.getByTestId("latest-archive");
  await expect(box).toBeVisible();
  await expect(box.getByTestId("archive-row-loading")).toHaveCount(0, { timeout: 15_000 });
  await expect(page.locator("[data-role=grid]").nth(5)).toBeVisible({ timeout: 15_000 });
}

const sourcesInOrder = (page: Page): Promise<string[]> =>
  page.locator("section[data-source]").evaluateAll((els) => els.map((e) => e.getAttribute("data-source") ?? ""));

async function gridBottom(page: Page, n: number): Promise<number> {
  const g = (await page.locator("[data-role=grid]").nth(n).boundingBox())!;
  return g.y + (await page.evaluate(() => window.scrollY)) + g.height;
}

const quiet = (base: Status): Status => ({
  ...base,
  reading: null,
  inbox: { capturable: true, blockers: [], emitted_at: new Date().toISOString() },
  pending_shape: null,
  latest_archive: {
    file_name: "takeout-20260919T000000Z-001.zip",
    first_seen_at: "2026-09-19T00:00:00Z",
    outcome: "read",
    unreadable_kind: null,
    inserted: 1,
    duplicate: 0,
    unreadable: 0,
    previously_read_at: null,
  },
});

const crowded = (base: Status): Status => ({
  ...base,
  reading: {
    file_name: "takeout-20260913T041200Z-001-with-a-very-long-name-that-wraps.zip",
    inner_path: "Takeout/Location History (Timeline)/Records.json",
    items_read: 410_000,
    started_at: "2026-09-15T02:30:00Z",
  },
  inbox: { capturable: false, blockers: ["dedicated_inbox_unreadable", "downloads_unreadable"], emitted_at: "2026-09-01T01:00:00Z" },
  pending_shape: { archives: 2, files: 3 },
  latest_archive: {
    file_name: "takeout-20260913T041200Z-001-with-a-very-long-name-that-wraps.zip",
    first_seen_at: "2026-09-13T04:12:00Z",
    outcome: "unreadable",
    unreadable_kind: "broken_zip",
    inserted: 2,
    duplicate: 1,
    unreadable: 0,
    previously_read_at: null,
  },
});

// Scenario: 集計の格子が Must の後ろで書庫の前に出る
test("格子は上から Must の 5 ソース・集計・書庫のソースの順に並ぶ", async ({ page }) => {
  await open(page, quiet);
  const order = await sourcesInOrder(page);
  expect(order.slice(0, 5)).toEqual(MUST);
  expect(order[5]).toBe("c01-app-usage-rollup");
  const archives = order.slice(6);
  expect(archives.length, "書庫のソースが 1 本も無い").toBeGreaterThan(0);
  expect(archives.every((s) => s.startsWith("c03-")), `集計の後ろが書庫のソースだけではない: ${archives}`).toBe(true);
});

// Scenario: 集計の格子を足しても Must の 5 本はひとスクロール以内
test("集計の記録がある 360 × 640 で、Must の最後の格子の下端は箱の上限ぶんしか下がらない", async ({ page }) => {
  await open(page, quiet);
  const order = await sourcesInOrder(page);
  expect(order.indexOf("c01-app-usage-rollup"), "集計が Must の後ろにある").toBe(5);
  const low = await gridBottom(page, 4);
  const rollupTop = (await page.locator("section[data-source='c01-app-usage-rollup']").boundingBox())!.y + (await page.evaluate(() => window.scrollY));
  expect(rollupTop, "集計は Must の最後の格子より下").toBeGreaterThanOrEqual(low);

  await open(page, crowded);
  expect(await gridBottom(page, 4), "箱が最も低いときの下端 + 160 px").toBeLessThanOrEqual(low + BOX_MAX_PX);
});

// Scenario: 集計の格子は開いた直後から直近 4 週を出す
test("集計の格子は押さなくても直近 4 週の行を出している", async ({ page }) => {
  await open(page, quiet);
  const grid = page.locator("section[data-source='c01-app-usage-rollup'] [data-role=grid]");
  await expect(grid).toHaveAttribute("data-weeks", "4");
  await expect(grid.locator("[data-week]")).toHaveCount(4);
  for (const row of await grid.locator("[data-week]").all()) {
    const box = (await row.boundingBox())!;
    expect(box.height, "行が描かれていない").toBeGreaterThan(0);
  }
});
