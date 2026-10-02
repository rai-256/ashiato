// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST22「記録の削除」を 1 本の流れで撮る:
 * 削除前の記録 → 削除操作 → 削除後の表示 → 再読み込み後も削除されている → 後始末に戻す。
 *
 * 置き場は `tools/recording/st22/`。`record-st22.sh` が対象コミットの `web/e2e-recording/` へ写して走らせる。
 *
 * 手順とアサーションは `e2e/day-erase.spec.ts` の「確認して消すと…」と同じもの
 * （spec を import すると向こうのテストまで登録されるので、helper はここに写す）。
 * 足したのは「再読み込み後も消えている」の確認と、見るための短い停止（HOLD）だけ。
 */
declare const process: { env: Record<string, string | undefined> };

const DAY = "2026-09-07";
const DAY_URL = `/#/day/${DAY}`;
const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);

async function hold(page: Page): Promise<void> {
  await page.waitForTimeout(HOLD);
}

async function openDay(page: Page): Promise<void> {
  await page.goto(DAY_URL);
  await page.reload();
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
  await expect(page.getByTestId("day-view")).toBeVisible();
}

test("ST22 録画: 消す → 消えた表示 → 再読み込みしても消えている", async ({ page, browser, browserName }) => {
  // 実行環境の記録用（record-st22.sh が results.json から拾う）。アサーションではない
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });

  // 1. 削除前の記録（test.step の名前は動画に焼き込まれる。見るための見出し）
  const row = page.getByTestId("row-stay").first();
  const range = await test.step("1. 削除前: 2026-09-07 の滞在が並んでいる", async () => {
    await openDay(page);
    await expect(row).toBeVisible();
    const r = (await row.locator("h2").innerText()).trim();
    await expect(page.getByTestId("row-stay").filter({ hasText: r })).toHaveCount(1);
    await expect(page.getByTestId("row-erased").filter({ hasText: r })).toHaveCount(0);
    await hold(page);
    return r;
  });

  // 2. 削除操作（詳細を開く → 「この滞在を消す」→ 確認 → 「消す」）
  await test.step(`2. 削除操作: ${range} を開いて「この滞在を消す」→「消す」`, async () => {
    await row.getByRole("button").click();
    await expect(row.getByTestId("stay-detail")).toBeVisible();
    await expect(row.getByRole("button", { name: "この滞在を消す" })).toBeVisible();
    await hold(page);
    await row.getByRole("button", { name: "この滞在を消す" }).click();
    await expect(row.getByTestId("erase-confirm")).toBeVisible();
    await hold(page);
    await row.getByRole("button", { name: "消す" }).click();
  });

  const erased = page.getByTestId("row-erased").filter({ hasText: range });
  let restored = false;
  try {
    // 3. 削除後の表示（day-erase.spec.ts と同じ主張）
    await test.step(`3. 削除後: ${range} が「消した」の行になった`, async () => {
      await expect(page).toHaveURL(/#\/day\/2026-09-07$/);
      await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(0);
      await expect(erased).toHaveCount(1);
      await expect(erased).toContainText("消した");
      await hold(page);
    });

    // 4. 再読み込みしても消えている（サーバに残っていること）
    await test.step("4. 再読み込み中…", async () => {
      await page.reload();
      await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
      await expect(page.getByTestId("day-view")).toBeVisible();
    });
    await test.step(`4. 再読み込み後: ${range} は「消した」のまま`, async () => {
      await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(0);
      await expect(page.getByTestId("row-erased").filter({ hasText: range })).toHaveCount(1);
      await expect(page.getByTestId("row-erased").filter({ hasText: range })).toContainText("消した");
      await hold(page);
    });

    // 後始末: 戻す（次の実行が同じ状態から始められるように。day-erase.spec.ts と同じ主張）
    await test.step(`後始末: 「戻す」で ${range} を元に戻す`, async () => {
      await page.getByTestId("row-erased").filter({ hasText: range }).getByRole("button", { name: "戻す" }).click();
      restored = true;
      await expect(page.getByTestId("row-erased").filter({ hasText: range })).toHaveCount(0);
      await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(1);
      await hold(page);
    });
  } finally {
    if (!restored) {
      await page
        .getByTestId("row-erased")
        .filter({ hasText: range })
        .getByRole("button", { name: "戻す" })
        .click({ timeout: 5_000 })
        .catch(() => undefined);
    }
  }
});
