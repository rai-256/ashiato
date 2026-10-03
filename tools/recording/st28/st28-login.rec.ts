// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST28 の画面のログインを 1 本の流れで撮る:
 * ログインしていない → 違う合言葉は断られる → 正しい合言葉で記録が出る → 1 日の画面も読める
 * → ログアウト → 再読み込みしても記録は出ない。
 *
 * **未ログインから始める**（`storageState` を空にする。既定はログイン済み —— `e2e/global-setup.ts`）。
 * アサーションは `e2e/login.spec.ts` の同名の Scenario の写し。足したのは見るための短い停止（HOLD）だけ。
 * 「スマホから私設網のホスト名で届く／網の外からは届かない」は本物の網が要るので、ここでは撮らない（人間の確認に残る）。
 */
declare const process: { env: Record<string, string | undefined> };

test.use({ storageState: { cookies: [], origins: [] } });

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const RECORD_ROWS =
  '[data-testid^="grid-"], [data-testid="row-stay"], [data-testid="claim-row"], [data-testid="kind-card"], [data-testid^="achieved-"]';

async function hold(page: Page): Promise<void> {
  await page.waitForTimeout(HOLD);
}

/** 画面が出した `/api/` の読み出し（GET）の応答の状態を集める。 */
function collectReads(page: Page): number[] {
  const statuses: number[] = [];
  page.on("response", (res) => {
    const url = new URL(res.url());
    if (url.pathname.startsWith("/api/") && res.request().method() === "GET") statuses.push(res.status());
  });
  return statuses;
}

async function loginByForm(page: Page, password: string): Promise<void> {
  await page.getByLabel("合言葉").fill(password);
  await page.getByRole("button", { name: "ログイン" }).click();
}

test("ST28 録画: 未ログイン → 違う合言葉 → ログイン → ログアウト", async ({ page, browser, browserName }) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  const password = process.env.WEB_PASSWORD;
  expect(password, "WEB_PASSWORD が無い（tools/record-env.sh が渡す）").toBeTruthy();

  await test.step("1. ログインしていない: 合言葉の入力欄だけが出て、記録は 1 件も返らない", async () => {
    const reads = collectReads(page);
    await page.goto("/");
    await expect(page.getByLabel("合言葉")).toBeVisible();
    await expect(page.getByRole("button", { name: "ログイン" })).toBeVisible();
    expect(reads.length).toBeGreaterThan(0);
    expect(reads.every((s) => s === 401)).toBe(true);
    await expect(page.locator(RECORD_ROWS)).toHaveCount(0);
    await hold(page);
  });

  await test.step("2. 違う合言葉: 断られて、記録は出ない", async () => {
    await loginByForm(page, "まちがった合言葉");
    await expect(page.getByRole("alert")).toBeVisible();
    await expect(page.getByLabel("合言葉")).toBeVisible();
    await expect(page.locator(RECORD_ROWS)).toHaveCount(0);
    await hold(page);
    await hold(page);
  });

  await test.step("3. 正しい合言葉: 稼働状況が出る", async () => {
    await loginByForm(page, password ?? "");
    await expect(page.getByTestId("coverage-loading")).toBeHidden({ timeout: 15_000 });
    await expect(page.locator('[data-testid^="grid-"]').first()).toBeVisible();
    await expect(page.getByTestId("coverage-error")).toHaveCount(0);
    await expect(page.getByLabel("合言葉")).toHaveCount(0);
    await hold(page);
  });

  await test.step("4. 1 日の画面も読める（2026-09-07）", async () => {
    await page.goto("/#/day/2026-09-07");
    await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
    await expect(page.getByTestId("row-stay").first()).toBeVisible();
    await hold(page);
  });

  await test.step("5. ログアウト: 合言葉の入力欄に戻る", async () => {
    await page.getByRole("button", { name: "ログアウト" }).click();
    await expect(page.getByLabel("合言葉")).toBeVisible();
    await hold(page);
  });

  await test.step("6. 再読み込みしても記録は出ない（印はもう使えない）", async () => {
    const reads = collectReads(page);
    await page.goto("/");
    await page.reload();
    await expect(page.getByLabel("合言葉")).toBeVisible();
    expect(reads.length).toBeGreaterThan(0);
    expect(reads.every((s) => s === 401)).toBe(true);
    await expect(page.locator(RECORD_ROWS)).toHaveCount(0);
    await hold(page);
  });
});
