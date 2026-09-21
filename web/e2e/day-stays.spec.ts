// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * **人間の目が見ていた 2 件を、本物のブラウザへ移す**（2026-09-22）。
 *
 * ST16 の `tasks.md`「人間の確認待ち」に 5 本あり、確認バッチ `verify-20260916-1442` で
 * 人間に聞いていた。**5 本とも機械の印は既に持っている** —— だからここは「担保の無い
 * Scenario を埋める」作業ではない。埋めるのは**層の穴**で、既存の担保がどちらも
 * 本物のブラウザを通っていない 2 本だけを持つ:
 *
 *   - 記録が欠けた時間は記録なしとして出る
 *       既存: `stay_tests.rs` が **API の応答**を見る。画面に描かれるかは見ていない
 *   - キーボードで移るとフォーカスの位置が見える
 *       既存: `day-view-limits.test.tsx`（jsdom）が**規則の文字列**を照合する。
 *       jsdom は Tab を押さず、`:focus-visible` が実際に掛かるかも測れない。
 *       実測 2026-09-22: ボタンに `tabIndex={-1}` を入れると **jsdom は 4 本とも緑のまま**、
 *       ここだけが「Tab で届かなかった: 前の日、次の日」で赤になった
 *
 * 残る 3 本はここに**置かない** —— `半径を変えて作り直すと区切りが変わる` と
 * `作り直しで位置の記録は変わらない` は `stay_tests.rs` の `stay_rebuild_keeps_locations` が
 * 正典どおりの数値（70 m 離れた 2 点・半径 50 m）で持っている。偽データを使うと
 * 「半径 30 m では件数が変わらない」のような、データ次第で緑にも赤にもなる検査になる
 * （実測 2026-09-22: 手元では通って CI で落ちた）。
 * `1 日歩き回った後、その日の滞在が一覧で出る` は**人間に残る 1 本**（`> 物理: gps`）。
 *
 * 見ているのは人間が見ていたのと同じもの —— `run.sh` と同じ `tools/stack.sh` が立てた縦串。
 */
// 偽データ（`SEED=normal`）が滞在を作る日。手順書が人間に開かせていたのと同じ日。
const DAY = "2026-09-07";

async function openDay(page: Page) {
  // **ハッシュだけの遷移では読み直さない。** 同じ URL への goto は何も起こさないので、
  // 作り直しの後に開き直しても古い応答のままになる（2026-09-22 に踏んだ）。
  await page.goto(`/#/day/${DAY}`);
  await page.reload();
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
  await expect(page.getByTestId("day-view")).toBeVisible();
}

// Scenario: 記録が欠けた時間は記録なしとして出る
test("記録が欠けた時間は「記録なし」と文字で出て、「移動」と見分けられる", async ({ page }) => {
  await openDay(page);

  const gaps = page.getByTestId("row-no-record");
  await expect(gaps.first()).toBeVisible();

  // 1. **文字で**区別している（色や線だけではない。design D8）
  const gapText = await gaps.first().innerText();
  expect(gapText).toContain("記録なし");
  expect(gapText).not.toContain("移動");

  // 2. 「移動」の行とは別の語で出ている
  const moves = page.getByTestId("row-move");
  if ((await moves.count()) > 0) {
    const moveText = await moves.first().innerText();
    expect(moveText).toContain("移動");
    expect(moveText).not.toContain("記録なし");
  }

  // 3. 実際に描かれている（jsdom には測れない）
  const box = await gaps.first().boundingBox();
  expect(box?.height ?? 0, "「記録なし」の行に高さが無い").toBeGreaterThan(0);
});

// Scenario: キーボードで移るとフォーカスの位置が見える
test("Tab で日の移動を辿ると、3 つの操作それぞれに輪郭が出る", async ({ page }) => {
  await openDay(page);

  // **名前で数える。** 同じ要素を数え直さない —— Tab は一周して戻ってくるので、
  // 重複を許すと「稼働状況へ」を 3 回数えて緑になる（2026-09-22 に踏んだ）。
  const want = ["前の日", "日付を指定", "次の日"];
  const seen = new Map<string, number>();

  for (let i = 0; i < 20 && seen.size < want.length; i++) {
    await page.keyboard.press("Tab");
    const at = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      if (!el || el === document.body) return null;
      return {
        name: el.getAttribute("aria-label") ?? el.innerText?.trim() ?? el.tagName,
        ring: el.hasAttribute("data-focus-ring"),
        // `:focus-visible` の輪郭が**実際に掛かっている**か。jsdom は規則の文字列しか見られない
        outline: Number.parseFloat(getComputedStyle(el).outlineWidth || "0"),
        height: el.getBoundingClientRect().height,
      };
    });
    if (!at?.ring || !want.includes(at.name)) continue;
    expect(at.height, `${at.name} に面積が無い`).toBeGreaterThan(0);
    expect(at.outline, `${at.name} に輪郭が出ていない`).toBeGreaterThan(0);
    seen.set(at.name, at.outline);
  }

  expect(
    [...seen.keys()].sort(),
    `Tab で届かなかった: ${want.filter((w) => !seen.has(w)).join("、")}`,
  ).toEqual([...want].sort());
});
