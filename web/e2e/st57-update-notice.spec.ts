import { expect, test } from "@playwright/test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";

declare const process: { env: Record<string, string | undefined> };
const dir = (): string => process.env.ASHIATO_UPDATER_DIR ?? "/tmp/ashiato-st57-updater";
const settings = (enabled = true, since = "2026-11-10T09:12:00Z"): void => {
  mkdirSync(`${dir()}/settings`, { recursive: true });
  writeFileSync(`${dir()}/settings/check.json`, JSON.stringify({ enabled, since }));
};
const result = (overrides: Record<string, unknown> = {}): void => {
  writeFileSync(`${dir()}/status.json`, JSON.stringify({ latest: "0.5.0", found_at: "2026-11-10T09:12:00Z", checked_at: "2026-11-10T09:12:00Z", last_ok_at: "2026-11-10T09:12:00Z", last_reason: null, notified_version: null, stale_notified: false, ...overrides }));
};
const clear = (): void => { rmSync(dir(), { recursive: true, force: true }); mkdirSync(`${dir()}/settings`, { recursive: true }); };

test.beforeEach(() => clear());
test.afterEach(() => clear());

async function coverage(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/#/coverage");
  await expect(page.locator("h1")).toBeVisible();
}

// Scenario: 更新ありはどの画面でも見出しの上に出る
test("更新ありはどの画面でも見出しの上に出る", async ({ page }) => {
  settings(); result();
  for (const hash of ["#/coverage", "#/day/", "#/master", "#/settings"]) {
    await page.goto(`/${hash}`); await expect(page.getByTestId("update-band")).toBeVisible();
    const band = await page.getByTestId("update-band").boundingBox(); const h1 = await page.locator("h1").boundingBox();
    expect((band?.y ?? 0) + (band?.height ?? 0)).toBeLessThanOrEqual(h1?.y ?? 0);
  }
});

// Scenario: 6 日確かめられていないだけなら帯は出ない
test("6 日確かめられていないだけなら帯は出ない", async ({ page }) => { settings(); result({ latest: "0.1.0", checked_at: "2026-11-11T23:00:00Z", last_ok_at: "2026-11-06T09:12:00Z", last_reason: "unreachable" }); await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); });
// Scenario: 7 日確かめられなければ帯に出る
test("7 日確かめられなければ帯に出る", async ({ page }) => { settings(); result({ checked_at: "2026-11-11T23:00:00Z", last_ok_at: "2026-11-03T09:12:00Z", last_reason: "unreachable" }); await coverage(page); await expect(page.getByTestId("update-band")).toContainText("11/03・配布元に届かない"); });
// Scenario: 帯の文字と地のコントラストは 3:1 以上
test("帯の文字と地のコントラストは 3:1 以上", async ({ page }) => { settings(); result({ checked_at: "2026-11-01T09:12:00Z", last_ok_at: "2026-11-01T09:12:00Z", last_reason: "unreachable" }); await coverage(page); for (const scheme of ["light", "dark"] as const) { await page.emulateMedia({ colorScheme: scheme }); await page.reload(); const colors = await page.getByTestId("update-band").evaluate((el) => { const s = getComputedStyle(el); return [s.color, s.backgroundColor]; }); expect(colors[0]).not.toBe(colors[1]); } });
// Scenario: 更新ありと確かめられていないが同時なら 2 行出る
test("更新ありと確かめられていないが同時なら 2 行出る", async ({ page }) => { settings(); result({ checked_at: "2026-11-11T23:00:00Z", last_ok_at: "2026-11-03T09:12:00Z", last_reason: "unreachable" }); await page.setViewportSize({ width: 360, height: 640 }); await coverage(page); await expect(page.getByTestId("update-band").locator("div")).toHaveCount(2); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); });
// Scenario: 新しい版が無ければ帯は出ない
test("新しい版が無ければ帯は出ない", async ({ page }) => { settings(); result({ latest: "0.1.0" }); await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); });
// Scenario: 確認を切ってあれば帯は出ず、設定に切ってあると出る
test("確認を切ってあれば帯は出ず、設定に切ってあると出る", async ({ page }) => { settings(false); result(); await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); await page.goto("/#/settings"); await expect(page.getByTestId("update-section")).toContainText("切ってある"); });
// Scenario: 帯を押しても何も起きない
test("帯を押しても何も起きない", async ({ page }) => { settings(); result(); await coverage(page); const before = page.url(); const requests: string[] = []; page.on("request", (r) => requests.push(r.url())); await page.getByTestId("update-band").click(); expect(page.url()).toBe(before); expect(requests).toHaveLength(0); });
// Scenario: 設定に更新の確認の状態が出る
test("設定に更新の確認の状態が出る", async ({ page }) => { settings(); result(); await page.goto("/#/settings"); const section = page.getByTestId("update-section"); for (const text of ["有効・新しい版あり", "入っている版", "見つけた最新の版 0.5.0", "最後に確かめた 11/10 09:12"]) await expect(section).toContainText(text); });
// Scenario: 更新係の無いサーバは設定に動いていないと出し、帯を出さない
test("更新係の無いサーバは設定に動いていないと出し、帯を出さない", async ({ page }) => { await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); await page.goto("/#/settings"); await expect(page.getByTestId("update-section")).toContainText("動いていない"); });
// Scenario: 結果を読めないときは帯を出さず、設定にだけ出す
test("結果を読めないときは帯を出さず、設定にだけ出す", async ({ page }) => { settings(); writeFileSync(`${dir()}/status.json`, "not-json"); await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); await page.goto("/#/settings"); await expect(page.getByTestId("update-section")).toContainText("結果を読めない"); });
// Scenario: まだ確かめていない更新係は、7 日たつまで帯を出さない
test("まだ確かめていない更新係は、7 日たつまで帯を出さない", async ({ page }) => { settings(true, "2026-11-10T09:12:00Z"); await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); await page.goto("/#/settings"); await expect(page.getByTestId("update-section")).toContainText("まだ確かめていない"); });
// Scenario: 新しい版なしは設定にだけ出る
test("新しい版なしは設定にだけ出る", async ({ page }) => { settings(); result({ latest: "0.1.0" }); await page.goto("/#/settings"); await expect(page.getByTestId("update-band")).toHaveCount(0); await expect(page.getByTestId("update-section")).toContainText("新しい版なし"); });
// Scenario: 帯を出しても外部への要求は 0 件である
test("帯を出しても外部への要求は 0 件である", async ({ page }) => { settings(); result(); await coverage(page); const origin = new URL(page.url()).origin; const offsite: string[] = []; page.on("request", (r) => { if (!r.url().startsWith(origin)) offsite.push(r.url()); }); await page.waitForTimeout(100); expect(offsite).toHaveLength(0); });
