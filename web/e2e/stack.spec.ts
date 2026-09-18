// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test } from "@playwright/test";

/**
 * **器が通ることだけを見る 1 本**（製造準備 B）。画面の振る舞いは Story ごとの change が足す。
 *
 * ここが落ちたら、壊れているのは画面ではなく縦串（DB → サーバ → 偽データ → 画面）。
 */
test("縦串が立って、S-1 稼働状況が本物のブラウザで開く", async ({ page }) => {
  const res = await page.goto("/");
  expect(res?.ok()).toBe(true);
  await expect(page).toHaveTitle("あしあと。");

  // 見出しが**実際に描かれている**（jsdom では測れない実寸を 1 つだけ確かめる）
  const h1 = page.locator("h1").first();
  await expect(h1).toBeVisible();
  const box = await h1.boundingBox();
  expect(box?.height ?? 0).toBeGreaterThan(0);

  // 画面はサーバと話せている（偽データが入っているので、読み込み中のまま止まらない）
  await expect(page.getByTestId("coverage-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("coverage-error")).toHaveCount(0);

  // 横スクロールが出ていない（縦 1 本で読む。実寸は本物のブラウザにしか分からない）
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
});
