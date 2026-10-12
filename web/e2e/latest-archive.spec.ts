// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test } from "@playwright/test";

/**
 * 「直近に置いた書庫」の箱が**本物のサーバから状態を受けて**出ること（ST12 / final review R47）。
 *
 * jsdom の試験（`web/src/__tests__`）は fetch を通らないので、画面が `/api/archives/status` を
 * `user_id` 無しで呼び、サーバがそれを 400 で断っていても緑だった —— 本人が見る箱は
 * いつも「読み出せませんでした」になっていた。ここは `tools/stack.sh` が立てた縦串
 * （`SEED=normal` が台帳に 1 冊入れる）を本物のブラウザで開いて確かめる。
 */

// seed（tools/seed.sh の「書庫（ST12）」）が台帳に置く 1 冊。値をリテラルで持つ
const SEEDED_ARCHIVE = "takeout-20260919T000000Z-001.zip";

// Scenario: 直近に置いた書庫の結果が箱に出る
test("直近に置いた書庫の名前と件数が箱に出る（読み出しに失敗していない）", async ({ page }) => {
  await page.goto("/");
  const box = page.getByTestId("latest-archive");
  await expect(box).toBeVisible();
  // 読み込み中のまま止まらず、失敗の文にもならない
  await expect(box.getByTestId("archive-row-loading")).toHaveCount(0, { timeout: 15_000 });
  await expect(box.getByTestId("archive-row-failed")).toHaveCount(0);
  const latest = box.getByTestId("archive-row-latest");
  await expect(latest).toContainText(SEEDED_ARCHIVE);
  await expect(latest).toContainText("入った 1");
  await expect(latest).toContainText("既にあった 0");
  await expect(latest).toContainText("読めなかった 0");
});

// Scenario: 直近に置いた書庫の箱は Must の前にある
test("箱は成功条件 1 の達成の下、最初の格子の上に描かれている", async ({ page }) => {
  await page.goto("/");
  const box = page.getByTestId("latest-archive");
  const achievement = page.getByTestId("achievement");
  const firstGrid = page.locator("[data-role=grid]").first();
  await expect(box).toBeVisible();
  await expect(achievement).toBeVisible({ timeout: 15_000 });
  await expect(firstGrid).toBeVisible({ timeout: 15_000 });
  const a = await achievement.boundingBox();
  const b = await box.boundingBox();
  const g = await firstGrid.boundingBox();
  expect(a && b && g, "どれかが描かれていない").toBeTruthy();
  expect(b!.y, "箱が達成より上にある").toBeGreaterThanOrEqual(a!.y + a!.height);
  expect(g!.y, "箱が最初の格子より下にある").toBeGreaterThanOrEqual(b!.y + b!.height);
});
