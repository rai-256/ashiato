// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { expect, test } from "@playwright/test";

/**
 * 録画用に独立して置く ST59 の設定画面（画面の合言葉と収集アプリの値を変える 2 つの節）。既存 e2e の相対 import は
 * 録画の写し先では使えない。録画のサーバは tools/record-env.sh が `.env` の合言葉で立てる**開発の形**で、
 * 設定のファイルを指していないので、変える求めはどちらも 409 で断られる（design・deep.md Q3）。
 * アサーションは web/e2e/st59-change-secrets.spec.ts の「設定に 2 つの節が並び…」と「開発の形のサーバでは…」の写し。
 * 変えられる流れ（設定のファイル型のサーバ）は録画の環境で起こせないので、scenarios.d の waive に担保する試験を挙げた。
 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const REFUSED = "この形のサーバでは画面から変えられません。.env を書き換えて起動し直してください";

test("ST59 録画: 設定の 2 つの節と、開発の形のサーバでの断り", async ({ page }) => {
  const password = process.env.WEB_PASSWORD;
  if (!password) throw new Error("WEB_PASSWORD が無い");

  await test.step("1. 設定を開くと、画面の合言葉と収集アプリの値を変える 2 つの節が並ぶ", async () => {
    await page.goto("/#/settings");
    await expect(page.getByLabel("今の合言葉").first()).toBeVisible();
    await expect(page.getByText("携帯を落としたときは両方を変える")).toBeVisible();
    await expect(page.getByRole("heading", { name: "画面の合言葉を変える" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "収集アプリの値を変える" })).toBeVisible();
    await page.getByTestId("web-password-change-section").scrollIntoViewIfNeeded();
    await page.waitForTimeout(HOLD * 2);
  });

  for (const [id, path, title] of [
    ["web-password-change-section", "web-password", "2. 画面の合言葉を変えようとすると、開発の形のサーバでは断られる"],
    ["collector-token-change-section", "collector-token", "3. 収集アプリの値を変えようとしても、同じく断られる"],
  ] as const) {
    await test.step(title, async () => {
      const section = page.getByTestId(id);
      await section.scrollIntoViewIfNeeded();
      const response = page.waitForResponse((item) => item.url().includes(`/api/${path}`));
      await section.getByLabel("今の合言葉").fill(password);
      await section.getByRole("button", { name: "変える" }).click();
      expect((await response).status()).toBe(409);
      await expect(section.getByRole("alert")).toHaveText(REFUSED);
      await page.waitForTimeout(HOLD * 2);
    });
  }

  await test.step("4. 断られた後も、元の合言葉でログインでき、元の収集アプリの値で記録を送れる", async () => {
    const base = process.env.REC_BASE_URL ?? "";
    const token = process.env.API_TOKEN;
    if (!token) throw new Error("API_TOKEN が無い");
    const context = await page.context().browser()!.newContext({ baseURL: base, storageState: { cookies: [], origins: [] } });
    const fresh = await context.newPage();
    await fresh.goto("/#/coverage");
    await fresh.getByLabel("合言葉").fill(password);
    await fresh.getByRole("button", { name: "ログイン" }).click();
    await expect(fresh.getByRole("link", { name: "設定" })).toBeVisible();
    await expect(fresh.getByTestId("coverage-view")).toBeVisible();
    const item = { id: crypto.randomUUID(), user_id: "00000000-0000-0000-0000-000000000000", logical_source: "c01-location", external_id: null, device_id: "st59-rec", origin: "collected", event_time: "1900-01-01T00:00:00Z", tz_offset_min: 540, tz_id: "Asia/Tokyo", schema_version: 1, raw: JSON.stringify({ lat: 35.6812, lon: 139.7671, acc_m: 10 }), payload: { lat: 35.6812, lon: 139.7671, acc_m: 10 } };
    const ingest = await fresh.request.post("/api/ingest", { headers: { Authorization: `Bearer ${token}` }, data: [item] });
    expect(ingest.status()).toBe(200);
    expect((await ingest.json() as { accepted: boolean }[]).every((row) => row.accepted)).toBe(true);
    await fresh.waitForTimeout(HOLD);
    await context.close();
  });
});
