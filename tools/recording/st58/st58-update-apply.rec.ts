// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { expect, test, type Page } from "@playwright/test";

/**
 * 録画用に独立して置く ST58 の画面（更新の帯と設定の「最後の更新」の行）。既存 e2e の相対 import は録画の写し先では使えない。
 * 戻した・入らなかったの行は Windows のサーバで更新が落ちたときにしか出ないので、`/api/update-status` の答えだけを
 * 差し替えて見せる（web/e2e/st58-update-apply.spec.ts と同じ形。ほかの API は本物のサーバが答える）。
 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);

const status = (overrides: Record<string, unknown> = {}): Record<string, unknown> => ({
  state: "available", installed: "0.4.0", latest: "0.5.0", found_at: "2026-11-14T00:00:00Z",
  checked_at: "2026-11-15T00:00:00Z", last_ok_at: "2026-11-15T00:00:00Z", reason: null, last_update: null, ...overrides,
});
const last = (result: string, reason: Record<string, unknown> | null): Record<string, unknown> =>
  ({ at: "2026-11-15T00:00:00Z", result, to_version: "0.5.0", from_version: "0.4.0", reason });

async function serve(page: Page, value: () => Record<string, unknown>): Promise<void> {
  await page.route("**/api/update-status", (route) =>
    route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value()) }));
}

test("ST58 録画: サーバの更新の手順と、戻した・入らなかった更新の知らせ", async ({ page }) => {
  let current = status();
  await serve(page, () => current);

  await test.step("1. 更新ありの帯を押すと、サーバの更新の手順が開く", async () => {
    await page.goto("/#/coverage");
    await expect(page.getByTestId("update-available")).toBeVisible();
    await page.waitForTimeout(HOLD);
    await page.getByTestId("update-available").click();
    const procedure = page.getByTestId("update-procedure");
    await expect(procedure).toContainText("サーバの更新の手順");
    await expect(procedure.locator("li")).toHaveCount(3);
    await expect(procedure).toContainText("移行が落ちたら自動で前の版に戻ります");
    await page.waitForTimeout(HOLD * 2);
  });

  await test.step("2. 移行が落ちて前の版に戻すと、帯に戻したことと版が出る", async () => {
    current = status({ last_update: last("rolled_back", { kind: "aborted", failed_version: "202611152200_x", sqlstate: "23505" }) });
    await page.reload();
    await expect(page.getByTestId("update-band")).toContainText("更新 0.5.0 は移行が落ちたので、0.4.0 に戻しました");
    await page.waitForTimeout(HOLD * 2);
  });

  await test.step("3. 設定の更新の節には、落ちた移行と理由まで出る", async () => {
    await page.goto("/#/settings");
    await expect(page.getByTestId("last-update")).toContainText("202611152200_x・23505（制約に反した）");
    await page.getByTestId("last-update").scrollIntoViewIfNeeded();
    await page.waitForTimeout(HOLD * 2);
  });

  await test.step("4. 入らなかった更新は、前の版のまま動いていることを出す", async () => {
    current = status({ state: "latest", latest: "0.4.0", last_update: last("not_applied", { kind: "db_ahead", failed_version: null, sqlstate: null }) });
    await page.goto("/#/coverage");
    await page.reload();
    await expect(page.getByTestId("update-band")).toContainText("更新 0.5.0 は入りませんでした（DB に新しい版の知らない版がある）。0.4.0 のまま動いています");
    await page.waitForTimeout(HOLD * 2);
  });

  await test.step("5. 次に更新が通ると、知らせは帯から消える", async () => {
    current = status({ state: "latest", latest: "0.5.0", installed: "0.5.0", last_update: last("applied", null) });
    await page.reload();
    await expect(page.getByTestId("update-band")).toHaveCount(0);
    await page.waitForTimeout(HOLD * 2);
  });
});
