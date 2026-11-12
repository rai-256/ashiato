import { expect, test } from "@playwright/test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { rgbContrastRatio } from "../src/contrast";

declare const process: { env: Record<string, string | undefined> };
const dir = (): string => process.env.ASHIATO_UPDATER_DIR ?? (() => { throw new Error("ASHIATO_UPDATER_DIR が無い"); })();
const now = (): Date => new Date();
const ago = (days: number): string => new Date(now().getTime() - days * 86_400_000).toISOString();
const date = (value: string): string => { const parts = new Intl.DateTimeFormat("en-CA", { timeZone: "Asia/Tokyo", month: "2-digit", day: "2-digit" }).formatToParts(new Date(value)); return `${parts.find((part) => part.type === "month")?.value}/${parts.find((part) => part.type === "day")?.value}`; };
const dateTime = (value: string): string => `${date(value)} ${new Intl.DateTimeFormat("en-GB", { timeZone: "Asia/Tokyo", hour: "2-digit", minute: "2-digit", hourCycle: "h23" }).format(new Date(value))}`;
const settings = (enabled = true, since = ago(1)): void => {
  mkdirSync(`${dir()}/settings`, { recursive: true });
  writeFileSync(`${dir()}/settings/check.json`, JSON.stringify({ enabled, since }));
};
const result = (overrides: Record<string, unknown> = {}): void => {
  const checked = ago(0);
  writeFileSync(`${dir()}/status.json`, JSON.stringify({ latest: "0.5.0", found_at: ago(1), checked_at: checked, last_ok_at: checked, last_reason: null, notified_version: null, stale_notified: false, ...overrides }));
};
const clear = (): void => { rmSync(dir(), { recursive: true, force: true }); mkdirSync(`${dir()}/settings`, { recursive: true }); };

test.beforeEach(() => clear());
test.afterEach(() => clear());

async function coverage(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/#/coverage");
  await expect(page.locator("h1")).toBeVisible();
}
async function installedVersion(page: import("@playwright/test").Page): Promise<string> {
  const response = await page.request.get("/api/update-status");
  return ((await response.json()) as { installed: string }).installed;
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
test("6 日確かめられていないだけなら帯は出ない", async ({ page }) => { settings(); result({ latest: "0.1.0", checked_at: ago(1), last_ok_at: ago(6), last_reason: "unreachable" }); await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); });
// Scenario: 7 日確かめられなければ帯に出る
test("7 日確かめられなければ帯に出る", async ({ page }) => { settings(); const lastOk = ago(8); result({ checked_at: ago(1), last_ok_at: lastOk, last_reason: "unreachable" }); await coverage(page); await expect(page.getByTestId("update-band")).toContainText(`${date(lastOk)}・配布元に届かない`); });
// Scenario: 帯の文字と地のコントラストは 3:1 以上
test("帯の文字と地のコントラストは 3:1 以上", async ({ page }) => { settings(); result({ checked_at: ago(1), last_ok_at: ago(8), last_reason: "unreachable" }); await coverage(page); for (const scheme of ["light", "dark"] as const) { await page.emulateMedia({ colorScheme: scheme }); await page.reload(); const colors = await page.getByTestId("update-band").evaluate((el) => { const css = (value: string): [number, number, number] => { const match = value.match(/\d+(?:\.\d+)?/g); if (!match || match.length < 3) throw new Error(`色を読めない: ${value}`); return [Number(match[0]), Number(match[1]), Number(match[2])]; }; const s = getComputedStyle(el); return [css(s.color), css(s.backgroundColor)] as const; }); expect(rgbContrastRatio(colors[0], colors[1])).toBeGreaterThanOrEqual(3); expect(await page.getByTestId("update-band").locator("div")).toHaveCount(2); } });
// Scenario: 更新ありと確かめられていないが同時なら 2 行出る
test("更新ありと確かめられていないが同時なら 2 行出る", async ({ page }) => { settings(); result({ checked_at: ago(1), last_ok_at: ago(8), last_reason: "unreachable" }); await page.setViewportSize({ width: 360, height: 640 }); await coverage(page); await expect(page.getByTestId("update-band").locator("div")).toHaveCount(2); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); });
// Scenario: 新しい版が無ければ帯は出ない
test("新しい版が無ければ帯は出ない", async ({ page }) => { const installed = await installedVersion(page); settings(); result({ latest: installed }); await coverage(page); const withBand = await page.locator("h1").boundingBox(); await expect(page.getByTestId("update-band")).toHaveCount(0); rmSync(`${dir()}/status.json`, { force: true }); await page.reload(); const withoutBand = await page.locator("h1").boundingBox(); expect(withBand?.y).toBe(withoutBand?.y); });
// Scenario: 確認を切ってあれば帯は出ず、設定に切ってあると出る
test("確認を切ってあれば帯は出ず、設定に切ってあると出る", async ({ page }) => { settings(false); result(); await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); await page.goto("/#/settings"); await expect(page.getByTestId("update-section")).toContainText("切ってある"); });
// Scenario: 帯を押しても何も起きない
test("帯を押しても何も起きない", async ({ page }) => { settings(); result(); await coverage(page); const before = page.url(); const requests: string[] = []; page.on("request", (r) => requests.push(r.url())); await page.getByTestId("update-band").click(); expect(page.url()).toBe(before); expect(requests).toHaveLength(0); });
// Scenario: 設定に更新の確認の状態が出る
test("設定に更新の確認の状態が出る", async ({ page }) => { const installed = await installedVersion(page); settings(); const found = ago(1); const checked = ago(0); result({ found_at: found, checked_at: checked, last_ok_at: checked }); await page.goto("/#/settings"); const section = page.getByTestId("update-section"); for (const text of ["有効・新しい版あり", `入っている版 ${installed}`, "見つけた最新の版 0.5.0", `（${date(found)} に見つけた）`, `最後に確かめた ${dateTime(checked)}`]) await expect(section).toContainText(text); });
// Scenario: 更新係の無いサーバは設定に動いていないと出し、帯を出さない
test("更新係の無いサーバは設定に動いていないと出し、帯を出さない", async ({ page }) => { await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); await page.goto("/#/settings"); await expect(page.getByTestId("update-section")).toContainText("動いていない"); });
// Scenario: 結果を読めないときは帯を出さず、設定にだけ出す
test("結果を読めないときは帯を出さず、設定にだけ出す", async ({ page }) => { settings(); writeFileSync(`${dir()}/status.json`, "not-json"); await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); await page.goto("/#/settings"); await expect(page.getByTestId("update-section")).toContainText("結果を読めない"); });
// Scenario: まだ確かめていない更新係は、7 日たつまで帯を出さない
test("まだ確かめていない更新係は、7 日たつまで帯を出さない", async ({ page }) => { settings(true, ago(2)); await coverage(page); await expect(page.getByTestId("update-band")).toHaveCount(0); await page.goto("/#/settings"); await expect(page.getByTestId("update-section")).toContainText("まだ確かめていない"); });
// Scenario: 新しい版なしは設定にだけ出る
test("新しい版なしは設定にだけ出る", async ({ page }) => { settings(); result({ latest: "0.1.0" }); await page.goto("/#/settings"); await expect(page.getByTestId("update-band")).toHaveCount(0); await expect(page.getByTestId("update-section")).toContainText("新しい版なし"); });
// Scenario: 帯を出しても外部への要求は 0 件である
test("帯を出しても外部への要求は 0 件である", async ({ page, baseURL }) => { settings(); result(); const origin = new URL(baseURL!).origin; const offsite: string[] = []; page.on("request", (r) => { const url = new URL(r.url()); if (!["data:", "blob:", "about:"].includes(url.protocol) && url.origin !== origin) offsite.push(r.url()); }); for (const hash of ["#/coverage", "#/day/", "#/master", "#/settings"]) { await page.goto(`/${hash}`); await expect(page.locator("h1")).toBeVisible(); } expect(offsite).toHaveLength(0); });
