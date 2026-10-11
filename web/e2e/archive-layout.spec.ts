// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 「直近に置いた書庫」の箱と書庫のソースの格子の**実寸**（ST12 / code-verify R64・R65）。
 *
 * jsdom の試験（`archive-one-scroll.test.tsx` / `latest-archive.test.tsx`）は `style` に書いた値を
 * 足すだけで、文字の折り返しも `box-sizing` も知らない —— 箱の外寸が 178 px になり、
 * 「ほか N 件」と省かない行が箱の外に出ていても緑だった。ここは本人が開く幅（360 × 640）で
 * 本物の Chromium に描かせて測る。
 *
 * 状態は本物のサーバの `/api/archives/status` を受けてから**箱に関わる欄だけ**差し替える
 * （書庫のソースの一覧はサーバのまま）。
 */

const BOX_MAX_PX = 160; // design D12（仮）。リテラルで持つ（定数どうしを突き合わせない）
const PHONE = { width: 360, height: 640 };

type Status = Record<string, unknown>;

/** 箱を上限まで埋める状態。長いファイル名は 360 px で折り返す長さにする。 */
function crowded(base: Status, latestOutcome: "read" | "unreadable"): Status {
  const long = "takeout-20260913T041200Z-001-with-a-very-long-name-that-wraps.zip";
  return {
    ...base,
    reading: {
      file_name: long,
      inner_path: "Takeout/Location History (Timeline)/Records.json",
      items_read: 410_000,
      started_at: "2026-09-15T02:30:00Z",
    },
    inbox: {
      capturable: false,
      blockers: ["dedicated_inbox_unreadable", "downloads_unreadable"],
      emitted_at: "2026-09-01T01:00:00Z",
    },
    pending_shape: { archives: 2, files: 3 },
    latest_archive: {
      file_name: long,
      first_seen_at: "2026-09-13T04:12:00Z",
      outcome: latestOutcome,
      unreadable_kind: latestOutcome === "unreadable" ? "broken_zip" : null,
      inserted: 2,
      duplicate: 1,
      unreadable: 0,
      previously_read_at: null,
    },
  };
}

/** 箱が最も低くなる状態（直近の書庫の 1 行だけ）。 */
function quiet(base: Status): Status {
  return {
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
  };
}

async function openWith(page: Page, patch: (base: Status) => Status): Promise<void> {
  await page.setViewportSize(PHONE);
  await page.route("**/api/archives/status**", async (route) => {
    const res = await route.fetch();
    const base = (await res.json()) as Status;
    await route.fulfill({ response: res, json: patch(base) });
  });
  await page.goto("/");
  // 読み込み中の 1 行が消えるまで待つ（直近の書庫の行は、溢れると「ほか N 件」に畳まれることがある）
  const box = page.getByTestId("latest-archive");
  await expect(box).toBeVisible();
  await expect(box.getByTestId("archive-row-loading")).toHaveCount(0, { timeout: 15_000 });
  await expect(box.getByTestId("archive-row-failed")).toHaveCount(0);
  await expect(page.locator("[data-role=grid]").nth(4)).toBeVisible({ timeout: 15_000 });
}

/** 行の箱が、箱の枠の内側に収まって見えている。 */
async function insideBox(page: Page, testId: string): Promise<void> {
  const box = (await page.getByTestId("latest-archive").boundingBox())!;
  const row = (await page.getByTestId(testId).boundingBox())!;
  expect(row, `${testId} が描かれていない`).toBeTruthy();
  expect(row.y, `${testId} の上端が箱より上`).toBeGreaterThanOrEqual(box.y);
  expect(row.y + row.height, `${testId} の下端が箱の外（${row.y + row.height} > ${box.y + box.height}）`).toBeLessThanOrEqual(
    box.y + box.height,
  );
}

/** 画面の上端（ページの先頭）からの、n 本目（0 始まり）の格子の下端。 */
async function gridBottom(page: Page, n: number): Promise<number> {
  const g = (await page.locator("[data-role=grid]").nth(n).boundingBox())!;
  const scrollY = await page.evaluate(() => window.scrollY);
  return g.y + scrollY + g.height;
}

// Scenario: 箱は 160 px を超えない
test("箱を上限まで埋めても、外寸は 160 px 以下で「ほか N 件」が箱の中に見える", async ({ page }) => {
  await openWith(page, (b) => crowded(b, "read"));
  const box = (await page.getByTestId("latest-archive").boundingBox())!;
  expect(box.height, "箱の外寸（枠と内側の余白を含む）").toBeLessThanOrEqual(BOX_MAX_PX);
  await insideBox(page, "archive-row-more");
  // 長い名前は折り返さず、1 行で「…」に省かれる（省いたことが示される）
  const reading = page.getByTestId("archive-row-reading");
  const ellipsized = await reading.evaluate((el) => el.scrollWidth > el.clientWidth);
  expect(ellipsized, "長い行が省かれずに折り返している").toBe(true);
  expect(await reading.evaluate((el) => getComputedStyle(el).textOverflow)).toBe("ellipsis");
});

// Scenario: 箱が溢れても読めなかった書庫は省かれない
test("箱が溢れても、読めなかった直近の書庫の行は箱の中に見える", async ({ page }) => {
  await openWith(page, (b) => crowded(b, "unreadable"));
  const box = (await page.getByTestId("latest-archive").boundingBox())!;
  expect(box.height).toBeLessThanOrEqual(BOX_MAX_PX);
  await expect(page.getByTestId("archive-row-latest")).toContainText("読めなかった書庫です");
  await insideBox(page, "archive-row-latest");
  await insideBox(page, "archive-row-more");
});

// Scenario: 箱の高さを除く量は 160 px を超えない
test("箱が低いときと上限まで埋めたときで、Must の格子が押し下げられる量は 160 px 以下", async ({ page }) => {
  await openWith(page, quiet);
  const low = await gridBottom(page, 4);
  await openWith(page, (b) => crowded(b, "unreadable"));
  const high = await gridBottom(page, 4);
  expect(high - low, "箱の伸びで押し下げられた量").toBeLessThanOrEqual(BOX_MAX_PX);
  const box = (await page.getByTestId("latest-archive").boundingBox())!;
  expect(box.height, "除く量（箱の外寸）").toBeLessThanOrEqual(BOX_MAX_PX);
});

// Scenario: 書庫のソースを足しても Must の 5 本はひとスクロール以内
// Scenario: 箱が上限の高さのとき 5 ソースが 1,440 px に収まる
test("箱を上限まで埋めても、Must の 5 本目の格子の下端は 1,440 px 以内", async ({ page }) => {
  await openWith(page, (b) => crowded(b, "unreadable"));
  // 書庫のソースは Must の後ろに積まれる。最初の 5 本の格子が Must
  const sources = await page.locator("[data-role=grid]").evaluateAll((els) =>
    els.slice(0, 5).map((e) => e.closest("section")?.getAttribute("data-source") ?? ""),
  );
  expect(sources.some((s) => s.startsWith("c03-")), `Must の前に書庫のソースがある: ${sources}`).toBe(false);
  expect(await gridBottom(page, 4)).toBeLessThanOrEqual(1_440);
});

// Scenario: 箱が上限の高さのとき 2 ソースが 800 px に収まる
test("箱を上限まで埋めたとき、2 本目の Must の直近 4 週の下端は 800 px 以内", async ({ page }) => {
  // **いまは成り立たない**（design D20（仮））。箱を除いた土台（ST02 の 640 px）が、達成の欄の実寸
  // （約 253 px）で既に約 63 px 超えている。直ったら test.fail を外す —— 外さないと緑に変わったところで落ちる。
  test.fail(true, "D20（仮）: ST02 の 640 px の土台が実寸で超えている（followup ST02）");
  await openWith(page, (b) => crowded(b, "unreadable"));
  expect(await gridBottom(page, 1)).toBeLessThanOrEqual(800);
});

// Scenario: 書庫のソースの格子は 360 px に収まる
// Scenario: 書庫のソースの週の帯は 24 px 以上
test("書庫のソースの格子は 360 px の幅に収まり、週の帯は 24 px 以上の高さがある", async ({ page }) => {
  await openWith(page, quiet);
  const archive = page.locator("section[data-source^='c03-'] [data-role=grid]");
  expect(await archive.count(), "書庫のソースの格子が 1 本も無い").toBeGreaterThan(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(PHONE.width);
  for (const grid of await archive.all()) {
    const g = (await grid.boundingBox())!;
    expect(g.x + g.width).toBeLessThanOrEqual(PHONE.width);
    const heights = await grid.evaluate((el) =>
      [...el.children].map((c) => (c as HTMLElement).getBoundingClientRect().height),
    );
    expect(heights.length).toBeGreaterThan(0);
    for (const h of heights) expect(h).toBeGreaterThanOrEqual(24);
  }
});
