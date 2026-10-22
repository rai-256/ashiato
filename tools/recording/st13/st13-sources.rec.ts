// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST13 の稼働状況の画面の変わったところを撮る:
 * Must の 5 本の格子の見出しに取り方と間隔（60 秒ごと・30 分ごと・変わったとき・24 時間ごと）→ 集計は「始めたときに 1 回」
 * → 書庫のソースは「手作業・2 か月ごと」と通知までの残り日数（起点の日つき）・過ぎた・まだ書庫が無い
 * → 携帯の収集アプリが休止の対象のままなら、位置の見出しは間隔の代わりに「休止対象・アプリで外す」
 * → 幅 360 px でも Must の 5 本の見出しは 1 行で横に溢れない → 読み出せなかった書庫は形の問題と別の文字
 * → S-7 設定には取り方と間隔の節が無い。
 *
 * 偽データは `tools/record-env.sh` が立てる `SEED=normal`（`tools/seed.d/202610212200_st13-sources-grid.sh`）。
 * 書庫の経過は seed した時点からの相対なので、期待する日数と日付は本物の `/api/sources` の値から作る。
 * **応答の差し替え（route）を使うのは 2 か所だけ**: 休止の対象のまま（seed の生存信号は外れている `exempt`）の
 * `device_notice` と、直近の書庫を読み出せなかった版の `latest_archive`（seed の読めた書庫が直近を奪うため）。
 * どちらも `e2e/coverage-sources.spec.ts` と同じ差し替え。
 *
 * アサーションは `e2e/coverage-sources.spec.ts` の主張の写し（spec を import すると向こうのテストまで登録されるので、
 * helper はここに写す）。足したのは見るための停止（HOLD）と、Must の写真・ブラウザ履歴の文字（seed の登録簿の値）だけ。
 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const PHONE = { width: 360, height: 640 };
const DESKTOP = { width: 1280, height: 720 };
const MUST_NOTES: [string, string][] = [
  ["c01-location", "60 秒ごと"],
  ["c01-app-usage", "30 分ごと"],
  ["c01-photo", "変わったとき"],
  ["c02-window", "変わったとき"],
  ["c02-browser-history", "24 時間ごと"],
];

type SourceApiRow = { logical_source: string; last_archive_on: string | null; alert_days_left: number | null; alert_passed: boolean };

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

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

async function showNote(page: Page, source: string): Promise<string> {
  const text = await noteOf(page, source);
  await page.locator(`section[data-source="${source}"]`).scrollIntoViewIfNeeded();
  return text;
}

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

test("ST13 録画: 格子の見出しの取り方と間隔 → 書庫の残り日数 → 休止の対象 → 360 px → 設定", async ({ page, browser, browserName }) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  // 見るための停止と操作の間隔（slowMo）で、既定の 30 秒を使い切る
  test.setTimeout(180_000);

  await test.step("1. 稼働状況: Must の 5 本の見出しに、取り方と間隔が読むだけで添えられている", async () => {
    await page.setViewportSize(DESKTOP);
    await page.emulateMedia({ colorScheme: "light" });
    await openCoverage(page);
    for (const [source, text] of MUST_NOTES) expect(await noteOf(page, source), source).toContain(text);
    // 見出しの行の中に変える操作は無い
    await expect(page.locator("section[data-source] h2 :is(input, select, button, textarea)")).toHaveCount(0);
    await hold(page, 3);
  });

  await test.step("2. アプリ利用の集計: 「始めたときに 1 回」で、間隔は出ない", async () => {
    const rollup = await showNote(page, "c01-app-usage-rollup");
    expect(rollup).toContain("始めたときに 1 回");
    expect(rollup).not.toMatch(/ごと/);
    await hold(page, 2);
  });

  await test.step("3. 書庫のソース: 手作業・2 か月ごとと、通知までの残り日数（最後の書庫の日つき）", async () => {
    const watch = await showNote(page, "c03-youtube-watch");
    const row = await sourceRow(page, "c03-youtube-watch");
    expect(watch).toContain("手作業・2 か月ごと");
    if (row.alert_passed) {
      expect(watch).toContain(`通知の日を過ぎた（最後の書庫 ${row.last_archive_on} から）`);
    } else {
      expect(watch).toContain(`通知まで あと ${row.alert_days_left} 日（最後の書庫 ${row.last_archive_on} から）`);
    }
    await expect(page.getByTestId("archive-note-c03-youtube-watch")).toContainText(/\d{4}-\d{2}-\d{2} まで（\d+ 日前）/);
    await hold(page, 3);
  });

  await test.step("4. 80 日前の書庫は通知の日を過ぎたと出る（負の日数は出ない）", async () => {
    const search = await showNote(page, "c03-youtube-search");
    const row = await sourceRow(page, "c03-youtube-search");
    expect(row.alert_passed).toBe(true);
    expect(search).toContain(`通知の日を過ぎた（最後の書庫 ${row.last_archive_on} から）`);
    expect(search).not.toContain("あと");
    expect(search).not.toMatch(/-\d+ ?日/);
    await hold(page, 2);
  });

  await test.step("5. 書庫も記録も無い Chrome の履歴は「まだ書庫が無い」", async () => {
    const chrome = await showNote(page, "c03-chrome-history");
    expect(chrome).toContain("まだ書庫が無い");
    expect(chrome).not.toContain("あと");
    expect(chrome).not.toContain("通知の日を過ぎた");
    await hold(page, 2);
  });

  await test.step("6. 携帯の収集アプリが休止の対象のまま（応答の差し替え）: 位置の見出しは間隔の代わりに「休止対象・アプリで外す」", async () => {
    await withNotice(page, "not_exempt");
    await openCoverage(page);
    const note = await showNote(page, "c01-location");
    expect(note).toContain("休止対象・アプリで外す");
    expect(note).not.toContain("60 秒ごと");
    await hold(page, 3);
    await page.unroute("**/api/sources");
  });

  await test.step("7. 幅 360 px: 外れている・外れていないのどちらでも Must の 5 本の見出しは 1 行で、横に溢れない", async () => {
    await page.setViewportSize(PHONE);
    for (const state of ["exempt", "not_exempt"]) {
      await withNotice(page, state);
      await openCoverage(page);
      await expect(page.getByTestId("source-note-c01-location")).toBeVisible({ timeout: 15_000 });
      await page.evaluate(() => window.scrollTo(0, 0));
      await expectOneLineHeadings(page);
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(PHONE.width);
      await hold(page, 3);
      await page.unroute("**/api/sources");
    }
  });

  await test.step("8. 直近の書庫を読み出せなかった（応答の差し替え）: 「読み出せません」と権限の案内で、形の対応の文言は出ない", async () => {
    await page.setViewportSize(DESKTOP);
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
    await page.goto("about:blank");
    await page.goto("/#/coverage");
    const latest = page.getByTestId("latest-archive").getByTestId("archive-row-latest");
    await expect(latest).toContainText("Timeline.json", { timeout: 15_000 });
    await expect(latest).toContainText("読み出せません");
    await expect(latest).toContainText("権限を確かめてください");
    await expect(latest).not.toContainText("扱えない形式");
    await expect(latest).not.toContainText("形が");
    await latest.scrollIntoViewIfNeeded();
    await hold(page, 3);
    await page.unroute("**/api/archives/status**");
  });

  await test.step("9. S-7 設定: 取り方と間隔の節は無く、全記録の書き出しの節がある", async () => {
    await page.goto("/#/settings");
    await expect(page.getByTestId("export-section")).toBeVisible();
    await expect(page.locator('[data-testid^="source-note-"]')).toHaveCount(0);
    const titles = await page.getByTestId("settings-view").locator("h2").allTextContents();
    expect(titles).toContain("全記録の書き出し");
    for (const word of ["端末", "PC", "書庫", "サーバ", "定期取得"]) {
      expect(titles.filter((t) => t.includes(word)), `見出し「${word}」を持つ節`).toEqual([]);
    }
    await hold(page, 2);
  });
});
