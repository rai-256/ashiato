// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST22「記録の削除」を 1 本の流れで撮る:
 * 稼働状況（消す前）→ 削除前の記録 → 確認で「やめる」と消えない → 削除操作 → 削除後の表示
 * → 再読み込み後も削除されている → 稼働状況は消す前と同じ → 後始末に戻す。
 *
 * 置き場は `tools/recording/st22/`（宣言は `tools/recording/recording.json`）。録画はハーネスの `scripts/record-run` が
 * 対象コミットの `web/e2e-recording/` へ写して走らせる。
 *
 * 手順とアサーションは `e2e/day-erase.spec.ts` の「確認して消すと…」と同じもの
 * （spec を import すると向こうのテストまで登録されるので、helper はここに写す）。
 * 足したのは「再読み込み後も消えている」「確認で やめる と消えない」「確認の文面に位置の件数が出る」
 * 「稼働状況は消す前と同じ」（spec の同名の Scenario）の確認と、見るための短い停止（HOLD）だけ。
 */
declare const process: { env: Record<string, string | undefined> };

const DAY = "2026-09-07";
const DAY_URL = `/#/day/${DAY}`;
const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);

async function hold(page: Page): Promise<void> {
  await page.waitForTimeout(HOLD);
}

/** 稼働状況の格子と達成日数（消しても変わらないことを見る。spec「記録を消しても達成日数は減らない」） */
async function coverageSnapshot(page: Page): Promise<string[]> {
  await page.goto("/");
  await expect(page.getByTestId("coverage-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("coverage-error")).toHaveCount(0);
  const grids = page.locator('[data-testid^="grid-"], [data-testid^="achieved-"]');
  await expect(grids.first()).toBeVisible();
  return grids.evaluateAll((els) => els.map((e) => `${e.getAttribute("data-testid")}=${e.innerHTML}`));
}

async function openDay(page: Page): Promise<void> {
  await page.goto(DAY_URL);
  await page.reload();
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
  await expect(page.getByTestId("day-view")).toBeVisible();
}

test("ST22 録画: 消す → 消えた表示 → 再読み込みしても消えている", async ({ page, browser, browserName }) => {
  // 実行環境の記録用（ハーネスの record_summarize.py が results.json から拾う）。アサーションではない
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });

  // 0. 稼働状況（消す前）。消した後と比べる
  const coverageBefore = await test.step("0. 稼働状況（消す前）", async () => {
    const snap = await coverageSnapshot(page);
    await hold(page);
    return snap;
  });

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

  // 2. 確認で「やめる」と消えない（spec「消す前に確認が出て、やめると消えない」）
  await test.step(`2. 確認で「やめる」: ${range} は消えない`, async () => {
    await row.getByRole("button").first().click();
    await expect(row.getByTestId("stay-detail")).toBeVisible();
    await expect(row.getByRole("button", { name: "この滞在を消す" })).toBeVisible();
    await hold(page);
    await row.getByRole("button", { name: "この滞在を消す" }).click();
    const confirm = row.getByTestId("erase-confirm");
    await expect(confirm).toBeVisible();
    // spec「確認の文面に一緒に消える位置の件数が出る」
    await expect(confirm).toContainText(/一緒に消える位置の記録 \d+ 件/);
    await hold(page);
    await hold(page);
    await confirm.getByRole("button", { name: "やめる" }).click();
    await expect(row.getByTestId("erase-confirm")).toHaveCount(0);
    await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(1);
    await expect(page.getByTestId("row-erased").filter({ hasText: range })).toHaveCount(0);
    await hold(page);
  });

  // 3. 削除操作（「この滞在を消す」→ 確認 → 「消す」）
  await test.step(`3. 削除操作: ${range} の「この滞在を消す」→「消す」`, async () => {
    await row.getByRole("button", { name: "この滞在を消す" }).click();
    await expect(row.getByTestId("erase-confirm")).toBeVisible();
    await hold(page);
    await row.getByRole("button", { name: "消す" }).click();
  });

  const erased = page.getByTestId("row-erased").filter({ hasText: range });
  let restored = false;
  try {
    // 4. 削除後の表示（day-erase.spec.ts と同じ主張）
    await test.step(`4. 削除後: ${range} が「消した」の行になった`, async () => {
      await expect(page).toHaveURL(/#\/day\/2026-09-07$/);
      await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(0);
      await expect(erased).toHaveCount(1);
      await expect(erased).toContainText("消した");
      await hold(page);
    });

    // 5. 再読み込みしても消えている（サーバに残っていること）
    await test.step("5. 再読み込み中…", async () => {
      await page.reload();
      await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
      await expect(page.getByTestId("day-view")).toBeVisible();
    });
    await test.step(`5. 再読み込み後: ${range} は「消した」のまま`, async () => {
      await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(0);
      await expect(page.getByTestId("row-erased").filter({ hasText: range })).toHaveCount(1);
      await expect(page.getByTestId("row-erased").filter({ hasText: range })).toContainText("消した");
      await hold(page);
    });

    // 6. 稼働状況は消す前と同じ（spec「1 日の記録をすべて消しても稼働状況は記録ありのまま」「記録を消しても達成日数は減らない」）
    await test.step("6. 稼働状況: 消す前と同じ（格子も達成日数も変わらない）", async () => {
      expect(await coverageSnapshot(page)).toEqual(coverageBefore);
      await hold(page);
    });

    // 後始末: 戻す（次の実行が同じ状態から始められるように。day-erase.spec.ts と同じ主張）
    await test.step(`後始末: 「戻す」で ${range} を元に戻す`, async () => {
      await openDay(page);
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
