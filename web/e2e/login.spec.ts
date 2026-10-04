// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

declare const process: { env: Record<string, string | undefined> };

/**
 * 画面のログイン（ST28 / design D10）。**未ログインの `storageState` から始める**
 * （既定はログイン済み。`global-setup.ts`）。
 *
 * 記録の行の要素 —— 稼働状況の格子・1 日の滞在の行・マスタの札。ログインしていない画面には 1 つも出ない。
 */
test.use({ storageState: { cookies: [], origins: [] } });

const RECORD_ROWS =
  '[data-testid^="grid-"], [data-testid="row-stay"], [data-testid="claim-row"], [data-testid="kind-card"], [data-testid^="achieved-"]';

/** 画面が出した `/api/` の読み出し（GET）の応答の状態を集める。 */
function collectReads(page: Page): number[] {
  const statuses: number[] = [];
  page.on("response", (res) => {
    const url = new URL(res.url());
    if (url.pathname.startsWith("/api/") && res.request().method() === "GET") statuses.push(res.status());
  });
  return statuses;
}

async function loginByForm(page: Page, password: string | undefined): Promise<void> {
  await page.getByLabel("合言葉").fill(password ?? "");
  await page.getByRole("button", { name: "ログイン" }).click();
}

test("ログインしていないブラウザには記録が 1 件も返らない", async ({ page }) => {
  // Scenario: ログインしていないブラウザには記録が 1 件も返らない
  const reads = collectReads(page);
  await page.goto("/");
  await expect(page.getByLabel("合言葉")).toBeVisible();
  expect(reads.length).toBeGreaterThan(0);
  expect(reads.every((s) => s === 401)).toBe(true);
  await expect(page.locator(RECORD_ROWS)).toHaveCount(0);
});

test("ログインしていないブラウザには合言葉の入力欄が出る", async ({ page }) => {
  // Scenario: ログインしていないブラウザには合言葉の入力欄が出る
  await page.goto("/");
  await expect(page.getByLabel("合言葉")).toBeVisible();
  await expect(page.getByRole("button", { name: "ログイン" })).toBeVisible();
});

test("合言葉でログインすると画面が記録を読める", async ({ page }) => {
  // Scenario: 合言葉でログインすると画面が記録を読める
  await page.goto("/");
  await loginByForm(page, process.env.WEB_PASSWORD);
  await expect(page.getByTestId("coverage-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.locator('[data-testid^="grid-"]').first()).toBeVisible();
  await expect(page.getByTestId("coverage-error")).toHaveCount(0);
  await expect(page.getByLabel("合言葉")).toHaveCount(0);
});

test("違う合言葉を入れても画面は記録を出さない", async ({ page }) => {
  // Scenario: 違う合言葉を入れても画面は記録を出さない
  await page.goto("/");
  await loginByForm(page, `違う-${Math.random().toString(36).slice(2)}`);
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(page.getByLabel("合言葉")).toBeVisible();
  await expect(page.locator(RECORD_ROWS)).toHaveCount(0);
});

test("ログアウトすると記録が読めなくなる", async ({ page }) => {
  // Scenario: ログアウトすると記録が読めなくなる
  // 共有のログイン（storageState）を失効させないよう、この試験は自分でログインする
  await page.goto("/");
  await loginByForm(page, process.env.WEB_PASSWORD);
  await expect(page.locator('[data-testid^="grid-"]').first()).toBeVisible();
  await page.getByRole("button", { name: "ログアウト" }).click();
  await expect(page.getByLabel("合言葉")).toBeVisible();

  const reads = collectReads(page);
  await page.goto("/");
  await page.reload();
  await expect(page.getByLabel("合言葉")).toBeVisible();
  expect(reads.length).toBeGreaterThan(0);
  expect(reads.every((s) => s === 401)).toBe(true);
  await expect(page.locator(RECORD_ROWS)).toHaveCount(0);
});

test("画面を配る側は合言葉を付け足さない", async ({ request }) => {
  // Scenario: 画面を配る側は合言葉を付け足さない
  // cookie も Authorization も持たない求めを、画面を配る側（preview の /api）で記録の読み出しへ送る
  const res = await request.get("/api/events");
  expect(res.status()).toBe(401);
});

test("ログインの印はスクリプトから読めない", async ({ page }) => {
  // Scenario: ログインの印はスクリプトから読めない
  await page.goto("/");
  await loginByForm(page, process.env.WEB_PASSWORD);
  await expect(page.locator('[data-testid^="grid-"]').first()).toBeVisible();
  // 印は本当にブラウザにある（HttpOnly で、スクリプトから見えないだけ）
  expect((await page.context().cookies()).some((c) => c.name === "ashiato_session")).toBe(true);
  const seen = await page.evaluate(() => document.cookie);
  expect(seen).not.toContain("ashiato_session");
});
