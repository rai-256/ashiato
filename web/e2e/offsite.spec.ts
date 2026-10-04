// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 画面は外部の資源を読まず、端末に記録の写しを置かない（ST28 / design D10）。
 * ログイン済み（既定の `storageState`）で稼働状況・1 日を見る・マスタ管理を順に開く。
 */
const VIEWS = ["/", "/#/day/2026-09-07", "/#/master"] as const;
const READY = ['[data-testid^="grid-"]', '[data-testid="day-view"]', '[data-testid="master-view"]'] as const;

async function openAll(page: Page): Promise<void> {
  for (const [i, path] of VIEWS.entries()) {
    await page.goto(path);
    await page.reload(); // ハッシュだけの遷移は読み直さない
    await expect(page.locator(READY[i]!).first()).toBeVisible({ timeout: 15_000 });
  }
}

test("画面を開いても外部への要求は 0 件である", async ({ page, baseURL }) => {
  // Scenario: 画面を開いても外部への要求は 0 件である
  const origin = new URL(baseURL!).origin;
  const outside: string[] = [];
  page.on("request", (req) => {
    const url = new URL(req.url());
    if (["data:", "blob:", "about:"].includes(url.protocol)) return;
    if (url.origin !== origin) outside.push(`${url.protocol}//${url.host}`);
  });
  await openAll(page);
  expect(outside).toEqual([]);
});

test("画面は外部の資源の読み込みを禁じる指示を持つ", async ({ page }) => {
  // Scenario: 画面は外部の資源の読み込みを禁じる指示を持つ
  const res = await page.goto("/");
  const csp = res?.headers()["content-security-policy"] ?? "";
  const defaultSrc = csp.split(";").map((d) => d.trim()).find((d) => d.startsWith("default-src"));
  expect(defaultSrc).toBe("default-src 'self'");
});

test("画面を開いても読み込みの指示に反した報告は出ない", async ({ page }) => {
  // Scenario: 画面を開いても読み込みの指示に反した報告は出ない
  await page.addInitScript(() => {
    const w = window as unknown as { __violations: string[] };
    w.__violations = [];
    document.addEventListener("securitypolicyviolation", (e) => w.__violations.push(e.violatedDirective));
  });
  const violations: string[] = [];
  for (const [i, path] of VIEWS.entries()) {
    await page.goto(path);
    await page.reload();
    await expect(page.locator(READY[i]!).first()).toBeVisible({ timeout: 15_000 });
    violations.push(...(await page.evaluate(() => (window as unknown as { __violations: string[] }).__violations)));
  }
  expect(violations).toEqual([]);
});

test("画面の応答は写しを保存させない", async ({ page, baseURL }) => {
  // Scenario: 画面の応答は写しを保存させない
  const origin = new URL(baseURL!).origin;
  const seen: { url: string; cacheControl: string }[] = [];
  page.on("response", (res) => {
    if (new URL(res.url()).origin === origin) {
      seen.push({ url: res.url(), cacheControl: res.headers()["cache-control"] ?? "" });
    }
  });
  await openAll(page);
  // 画面の本体と、画面の経路を通した記録の読み出しの両方が入っている
  expect(seen.some((s) => new URL(s.url).pathname === "/")).toBe(true);
  expect(seen.some((s) => new URL(s.url).pathname.startsWith("/api/coverage"))).toBe(true);
  expect(seen.filter((s) => !s.cacheControl.includes("no-store"))).toEqual([]);
});

test("画面は端末に記録の写しを置かない", async ({ page }) => {
  // Scenario: 画面は端末に記録の写しを置かない
  await openAll(page);
  // 偽データの値: 画面が読んだ記録の論理ソース名（試験が seed で入れたもの）
  const sources = await page.evaluate(async () => {
    const r = await fetch("/api/coverage?from=2026-01-01&to=2026-12-31");
    return ((await r.json()) as { logical_source: string }[]).map((s) => s.logical_source);
  });
  expect(sources.length).toBeGreaterThan(0);

  const kept = await page.evaluate(async () => {
    const dbs = await indexedDB.databases();
    const idb: unknown[] = [];
    for (const d of dbs) idb.push(d.name);
    return {
      workers: (await navigator.serviceWorker.getRegistrations()).length,
      dump: JSON.stringify({ local: { ...localStorage }, session: { ...sessionStorage }, idb }),
      dbCount: dbs.length,
    };
  });
  expect(kept.workers).toBe(0);
  expect(kept.dbCount).toBe(0);
  for (const s of sources) expect(kept.dump).not.toContain(s);
});
