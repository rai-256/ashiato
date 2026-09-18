// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test } from "@playwright/test";

/**
 * **人間の目から機械へ移した 1 件**（2026-09-18）。
 *
 * この Scenario は ST02 の `tasks.md`「人間の確認待ち」に残り、確認バッチ
 * `verify-20260913-2255` で人間に「一目で読めるか」を聞いていた。
 * 畳んだ週の**本数**は jsdom（`web/src/__tests__/expand-to-year.test.tsx`）が固定しているが、
 * **実際に描かれているか・横に溢れないか**は jsdom には測れない。ここが測る。
 *
 * 見ているのは人間が見ていたのと同じもの —— `run.sh` と同じ `tools/stack.sh` が立てた
 * 偽データ（`SEED=normal`）の稼働状況。
 */

// **spec の「1 年ぶん（53 週）」をリテラルで持つ**（jsdom 側 R5 / I4 と同じ理由。
// 実装の定数と突き合わせると、定数を 20 週に変えても緑のままになる）
const WEEKS_IN_A_YEAR = 53;

// Scenario: 1 年ぶんが一目で読める
test("伸ばすと 1 年ぶんの日が週に畳まれた格子として、実寸で読める", async ({ page }) => {
  await page.goto("/");

  // 格子を 1 つ選ぶ（ソースは複数ある。最初の 1 本で足りる）
  const section = page.locator("section", { has: page.locator("[data-role=grid]") }).first();
  const grid = section.locator("[data-role=grid]");
  await expect(grid).toBeVisible();
  const initial = Number(await grid.getAttribute("data-weeks"));
  expect(initial, "開いた直後は直近だけ").toBeLessThanOrEqual(6);

  await section.getByRole("button", { name: "1 年ぶんを見る" }).click();

  // 1. 1 年ぶんの日が、週に畳まれた行として出ている
  await expect(grid).toHaveAttribute("data-weeks", String(WEEKS_IN_A_YEAR));
  await expect(grid.locator("[data-week]")).toHaveCount(WEEKS_IN_A_YEAR);

  // 2. **本当に描かれている**（jsdom はここを見られない）。
  //    先頭・中ほど・末尾の週が、どれも面積を持っている
  const rows = grid.locator("[data-week]");
  for (const i of [0, Math.floor(WEEKS_IN_A_YEAR / 2), WEEKS_IN_A_YEAR - 1]) {
    const box = await rows.nth(i).boundingBox();
    expect(box?.width ?? 0, `${i} 番目の週に幅が無い`).toBeGreaterThan(0);
    expect(box?.height ?? 0, `${i} 番目の週に高さが無い`).toBeGreaterThan(0);
  }

  // 3. 横に溢れていない（縦 1 本で読む。横スクロールが要るなら「一目」ではない）
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow, "横スクロールが出ている").toBeLessThanOrEqual(0);

  // 4. 末尾の週までスクロールで届き、届いた先が画面の中にある
  await rows.nth(WEEKS_IN_A_YEAR - 1).scrollIntoViewIfNeeded();
  await expect(rows.nth(WEEKS_IN_A_YEAR - 1)).toBeInViewport();

  // 5. 畳み戻せる（伸ばしたままにならない）
  await section.getByRole("button", { name: "直近だけにする" }).click();
  expect(Number(await grid.getAttribute("data-weeks"))).toBeLessThanOrEqual(6);
});
