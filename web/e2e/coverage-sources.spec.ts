// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 稼働状況の「直近に置いた書庫」に、読み出せなかったファイルが形の問題と別の文字で出ること（ST13 / design D8）。
 *
 * 状態は本物のサーバの `/api/archives/status` を受けてから `latest_archive` だけ、seed が台帳に置く
 * `file_unreadable` の行（`tools/seed.d/202610211200_st13-sources.sh`）と同じ値に差し替える
 * —— 直近の書庫は finished_at の新しい順で、seed の読めた書庫（latest-archive.spec が見る）を奪えないため。
 */

// Scenario: 読み出せなかったファイルは稼働状況に形の問題と別の文字で出る
test("読み出せなかったファイルは「読み出せません」と出て、形の対応の文言は出ない", async ({ page }) => {
  await page.route("**/api/archives/status**", async (route) => {
    const res = await route.fetch();
    const base = (await res.json()) as Record<string, unknown>;
    await route.fulfill({
      response: res,
      json: {
        ...base,
        latest_archive: {
          file_name: "Timeline.json",
          first_seen_at: "2026-09-01T00:00:00Z",
          outcome: "unreadable",
          unreadable_kind: "file_unreadable",
          inserted: 0,
          duplicate: 0,
          unreadable: 0,
          previously_read_at: null,
        },
      },
    });
  });
  await page.goto("/#/coverage");
  const latest = page.getByTestId("latest-archive").getByTestId("archive-row-latest");
  await expect(latest).toContainText("Timeline.json", { timeout: 15_000 });
  await expect(latest).toContainText("読み出せません");
  await expect(latest).toContainText("権限を確かめてください");
  await expect(latest).not.toContainText("扱えない形式");
  await expect(latest).not.toContainText("形が");
});

/**
 * 稼働状況の各ソースの格子の見出しに、取り方と取得の間隔が読むだけで添えられること（ST13 / design D10・D11・D18・D19）。
 *
 * 状態は seed（`tools/seed.d/202610212200_st13-sources-grid.sh`）が置く。書庫の経過はいまからの相対なので、
 * 期待する日付もいまから数える。「外れていない」版は本物の応答の `device_notice` だけ差し替える。
 */
const PHONE = { width: 360, height: 640 };

type SourceApiRow = { logical_source: string; last_archive_on: string | null; alert_days_left: number | null; alert_passed: boolean };

/**
 * 本物の `/api/sources` の 1 行。書庫の経過は seed した時点の now() から置いてあるので、
 * 走らせた時刻から期待値を数えると seed から時間が経つほどずれる —— 期待する日付と日数は応答の値から作る。
 */
async function sourceRow(page: Page, source: string): Promise<SourceApiRow> {
  // 画面と同じ認証（cookie）で読むため、開いた後のページの中から読む
  const body = await page.evaluate(async () => {
    const res = await fetch("/api/sources");
    if (!res.ok) throw new Error(`/api/sources ${res.status}`);
    return (await res.json()) as { sources: SourceApiRow[] };
  });
  const row = body.sources.find((r) => r.logical_source === source);
  expect(row, `${source} の行が /api/sources に無い`).toBeDefined();
  return row!;
}

async function openCoverage(page: Page): Promise<void> {
  // 同じ URL への goto は読み直さない（hash の移動だけ）。route を差し替えた後は必ず読み直させる
  await page.goto("about:blank");
  await page.goto("/#/coverage");
  await expect(page.locator("[data-role=grid]").nth(4)).toBeVisible({ timeout: 15_000 });
}

/** `/api/sources` の応答の `device_notice` だけ差し替える（行は本物のまま）。 */
async function withNotice(page: Page, state: string): Promise<void> {
  await page.route("**/api/sources", async (route) => {
    const res = await route.fetch();
    const base = (await res.json()) as Record<string, unknown>;
    await route.fulfill({ response: res, json: { ...base, device_notice: { unused_app_restrictions: state } } });
  });
}

async function noteOf(page: Page, source: string): Promise<string> {
  const note = page.getByTestId(`source-note-${source}`);
  await expect(note).toBeVisible({ timeout: 15_000 });
  return (await note.textContent()) ?? "";
}

/** 画面の上端（ページの先頭）からの、n 本目（0 始まり）の格子の下端。 */
async function gridBottom(page: Page, n: number): Promise<number> {
  const g = (await page.locator("[data-role=grid]").nth(n).boundingBox())!;
  return g.y + (await page.evaluate(() => window.scrollY)) + g.height;
}

// Scenario: 間隔ごとのソースは格子の見出しに取得の間隔が出る
test("位置は「60 秒ごと」、アプリ利用は「30 分ごと」が見出しに出る", async ({ page }) => {
  await openCoverage(page);
  expect(await noteOf(page, "c01-location")).toContain("60 秒ごと");
  expect(await noteOf(page, "c01-app-usage")).toContain("30 分ごと");
});

// Scenario: 変わったときに取るソースは格子の見出しに契機が出る
test("PC のウィンドウは「変わったとき」、集計は「始めたときに 1 回」で、間隔は出ない", async ({ page }) => {
  await openCoverage(page);
  const win = await noteOf(page, "c02-window");
  const rollup = await noteOf(page, "c01-app-usage-rollup");
  expect(win).toContain("変わったとき");
  expect(rollup).toContain("始めたときに 1 回");
  for (const t of [win, rollup]) expect(t).not.toMatch(/ごと/);
});

// Scenario: 書庫のソースは手作業と起点つきの通知までの残り日数が出る
test("視聴履歴は手作業・2 か月ごとと、通知まで 通知までの残り日数（最後の書庫の日つき）が出て、取り込み済みの最終日も残る", async ({ page }) => {
  await openCoverage(page);
  const note = await noteOf(page, "c03-youtube-watch");
  expect(note).toContain("手作業・2 か月ごと");
  const row = await sourceRow(page, "c03-youtube-watch");
  // 通知の日は seed の書庫の日で決まり、過ぎれば応答が過ぎたと言う。画面は応答に従っているかを見る
  if (row.alert_passed) expect(note).toContain(`通知の日を過ぎた（最後の書庫 ${row.last_archive_on} から）`);
  else {
    expect(row.alert_days_left).toBeGreaterThan(0);
    expect(note).toContain(`通知まで あと ${row.alert_days_left} 日（最後の書庫 ${row.last_archive_on} から）`);
  }
  await expect(page.getByTestId("archive-note-c03-youtube-watch")).toContainText(/\d{4}-\d{2}-\d{2} まで（\d+ 日前）/);
});

// Scenario: 通知の日を過ぎた書庫のソースはそのことが出る
test("80 日前の書庫は通知の日を過ぎたと最後の書庫の日つきで出て、負の日数は出ない", async ({ page }) => {
  await openCoverage(page);
  const note = await noteOf(page, "c03-youtube-search");
  const row = await sourceRow(page, "c03-youtube-search");
  expect(row.alert_passed).toBe(true);
  expect(note).toContain(`通知の日を過ぎた（最後の書庫 ${row.last_archive_on} から）`);
  expect(note).not.toContain("あと");
  expect(note).not.toMatch(/-\d+ ?日/);
});

// Scenario: 通知が鳴る 1 時間前の書庫のソースは残り 1 日と出る
test("74 日 23 時間前の書庫は あと 1 日 で、過ぎたとは出ない", async ({ page }) => {
  await openCoverage(page);
  const note = await noteOf(page, "c03-timeline-visit");
  // seed から 1 時間以内なら あと 1 日。過ぎた後は応答が過ぎたと言うので、画面は応答に従っているかを見る
  const row = await sourceRow(page, "c03-timeline-visit");
  if (row.alert_passed) expect(note).toContain("通知の日を過ぎた");
  else {
    expect(row.alert_days_left).toBe(1);
    expect(note).toContain("通知まで あと 1 日");
    expect(note).not.toContain("過ぎた");
  }
});

// Scenario: 通知が鳴った 1 時間後の書庫のソースは過ぎたと出る
test("75 日 1 時間前の書庫は通知の日を過ぎたと出る", async ({ page }) => {
  await openCoverage(page);
  const note = await noteOf(page, "c03-timeline-move");
  expect(note).toContain("通知の日を過ぎた");
  expect(note).not.toContain("あと");
});

// Scenario: 書庫も記録も無い書庫のソースはまだ書庫が無いと出る
test("書庫も記録も無い Chrome の履歴は まだ書庫が無い と出て、残り日数は出ない", async ({ page }) => {
  await openCoverage(page);
  const note = await noteOf(page, "c03-chrome-history");
  expect(note).toContain("まだ書庫が無い");
  expect(note).not.toContain("あと");
  expect(note).not.toContain("通知の日を過ぎた");
});

// Scenario: 退役したソースは通知までの残り日数を出さない
test("退役した移行前のロケーション履歴には取り方も残り日数も出ず、退役の文字が出る", async ({ page }) => {
  // 退役した格子は畳まれて週の帯が 0 本になり、書庫の格子の実寸を測る既存の e2e を落とすので、seed には置かない。
  // 本物の応答の同じ 1 行だけ退役済みに差し替える（/coverage と /sources の両方）
  const retire = (rows: Record<string, unknown>[]): Record<string, unknown>[] =>
    rows.map((r) => (r.logical_source === "c03-legacy-location" ? { ...r, retired_on: "2026-09-01" } : r));
  await page.route("**/api/coverage?**", async (route) => {
    const res = await route.fetch();
    await route.fulfill({ response: res, json: retire((await res.json()) as Record<string, unknown>[]) });
  });
  await page.route("**/api/sources", async (route) => {
    const res = await route.fetch();
    const base = (await res.json()) as { sources: Record<string, unknown>[] };
    await route.fulfill({ response: res, json: { ...base, sources: retire(base.sources) } });
  });
  await openCoverage(page);
  const section = page.locator('section[data-source="c03-legacy-location"]');
  await expect(section.getByTestId("retired-c03-legacy-location")).toBeVisible();
  await expect(page.getByTestId("source-note-c03-legacy-location")).toHaveCount(0);
  await expect(section).not.toContainText("手作業・2 か月ごと");
  await expect(section).not.toContainText("通知まで");
});

// Scenario: 休止の対象から外れていないことが位置の格子の見出しに出る
test("休止の対象から外れていなければ、位置の見出しは間隔の代わりに休止の対象と出る", async ({ page }) => {
  await withNotice(page, "not_exempt");
  await openCoverage(page);
  const note = await noteOf(page, "c01-location");
  expect(note).toContain("休止の対象（アプリで外す）");
  expect(note).not.toContain("60 秒ごと");
});

/** Must の 5 本の見出しが 1 行（高さが行の高さの 1.5 倍未満）。 */
async function expectOneLineHeadings(page: Page): Promise<void> {
  const h2s = page.locator("section[data-source] h2");
  for (let i = 0; i < 5; i++) {
    const { height, lineHeight } = await h2s.nth(i).evaluate((el) => ({
      height: el.getBoundingClientRect().height,
      lineHeight: parseFloat(getComputedStyle(el).lineHeight),
    }));
    expect(height, `${i + 1} 本目の見出しの高さ ${height} px（行の高さ ${lineHeight} px）`).toBeLessThan(lineHeight * 1.5);
  }
}

async function bottomsWithoutNotes(page: Page): Promise<number[]> {
  await page.route("**/api/sources", (route) => route.fulfill({ json: { sources: [], device_notice: null } }));
  await openCoverage(page);
  await expect(page.locator('[data-testid^="source-note-"]')).toHaveCount(0);
  const out = [await gridBottom(page, 1), await gridBottom(page, 4)];
  await page.unroute("**/api/sources");
  return out;
}

// Scenario: 添えた文字で Must の 5 ソースの見出しは折り返さない
test("360 × 640 で、除外の状態が外れていてもいなくても Must の 5 本の見出しは 1 行で、格子の下端は動かない", async ({ page }) => {
  await page.setViewportSize(PHONE);
  const base = await bottomsWithoutNotes(page);
  for (const state of ["exempt", "not_exempt"]) {
    await withNotice(page, state);
    await openCoverage(page);
    await expect(page.getByTestId("source-note-c01-location")).toBeVisible({ timeout: 15_000 });
    await expectOneLineHeadings(page);
    expect(Math.abs((await gridBottom(page, 1)) - base[0]!), `${state}: 2 本目の下端`).toBeLessThanOrEqual(1);
    expect(Math.abs((await gridBottom(page, 4)) - base[1]!), `${state}: 5 本目の下端`).toBeLessThanOrEqual(1);
    await page.unroute("**/api/sources");
  }
});

// Scenario: 取り方と間隔を読めなくても格子は出る
test("取り方と間隔の読み出しが 503 でも格子は同じ数だけ出て、添え書きは 1 つも出ない", async ({ page }) => {
  await openCoverage(page);
  const real = await page.locator("section[data-source]").count();
  expect(real).toBeGreaterThan(5);
  await page.route("**/api/sources", (route) => route.fulfill({ status: 503, body: "unavailable" }));
  await openCoverage(page);
  await expect(page.locator("section[data-source]")).toHaveCount(real);
  await expect(page.locator('[data-testid^="source-note-"]')).toHaveCount(0);
});

// Scenario: 格子を持たないソースの取り方と間隔は画面に出ない
test("携帯端末の時計と取り込み器の見回りは、稼働状況にも S-7 設定にも添え書きの場所が無い", async ({ page }) => {
  await openCoverage(page);
  // 本文の字では見ない —— 「直近に置いた書庫」の箱の文言が「1 時間ごと」を含む（spec-review R24）
  for (const name of ["c01-clock", "s01-archive-inbox"]) {
    await expect(page.getByTestId(`source-note-${name}`)).toHaveCount(0);
    await expect(page.locator(`section[data-source="${name}"]`)).toHaveCount(0);
  }
  await page.goto("/#/settings");
  await expect(page.getByTestId("settings-view")).toBeVisible();
  for (const name of ["c01-clock", "s01-archive-inbox"]) await expect(page.getByTestId(`source-note-${name}`)).toHaveCount(0);
});

// Scenario: S-7 設定にソースの取り方と間隔が出ない
test("S-7 設定には取り方と間隔の節が無く、全記録の書き出しの節がある", async ({ page }) => {
  await page.goto("/#/settings");
  await expect(page.getByTestId("export-section")).toBeVisible();
  await expect(page.locator('[data-testid^="source-note-"]')).toHaveCount(0);
  const titles = await page.getByTestId("settings-view").locator("h2").allTextContents();
  expect(titles).toContain("全記録の書き出し");
  for (const word of ["端末", "PC", "書庫", "サーバ", "定期取得"]) {
    expect(titles.filter((t) => t.includes(word)), `見出し「${word}」を持つ節`).toEqual([]);
  }
});

// Scenario: 格子に添えた取り方と間隔に変える操作は無い
test("各格子の見出しの行の中に入力欄・選択・ボタンが 1 つも無い", async ({ page }) => {
  await openCoverage(page);
  await expect(page.getByTestId("source-note-c01-location")).toBeVisible({ timeout: 15_000 });
  await expect(page.locator("section[data-source] h2 :is(input, select, button, textarea)")).toHaveCount(0);
});

// Scenario: 360 px の幅で添えた文字が横に溢れない
test("360 px で画面は横に溢れず、各見出しの行も溢れない", async ({ page }) => {
  await page.setViewportSize(PHONE);
  await withNotice(page, "not_exempt");
  await openCoverage(page);
  await expect(page.getByTestId("source-note-c03-youtube-watch")).toBeVisible({ timeout: 15_000 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(360);
  const h2s = page.locator("section[data-source] h2");
  for (let i = 0; i < (await h2s.count()); i++) {
    const { sw, cw } = await h2s.nth(i).evaluate((el) => ({ sw: el.scrollWidth, cw: el.clientWidth }));
    expect(sw, `${i + 1} 本目の見出しが横に溢れる`).toBeLessThanOrEqual(cw);
  }
});

// Scenario: ソースの一覧は認証の無い要求には返らない
test("認証の無い要求には /api/sources は 401 を返す", async ({ playwright, baseURL }) => {
  const anon = await playwright.request.newContext({ baseURL, storageState: { cookies: [], origins: [] } });
  const res = await anon.get("/api/sources");
  expect(res.status()).toBe(401);
  await anon.dispose();
});
