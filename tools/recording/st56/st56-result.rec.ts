// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { expect, test, type Locator, type Page } from "@playwright/test";

/** 前半で設定後に積んだ PC の記録が、録画用サーバへ届いたことを 1 日の画面で見せる。 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const TITLE = "ST56 の設定";

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

async function reveal(stay: Locator): Promise<Locator> {
  const row = stay.getByTestId("stay-detail").getByText(TITLE).first();
  if (await row.isVisible()) return row;
  const folds = stay.getByTestId("records").getByRole("button");
  for (let i = 0; i < await folds.count(); i++) {
    if ((await folds.nth(i).getAttribute("aria-expanded")) !== "true") await folds.nth(i).click();
    if (await row.isVisible()) return row;
  }
  return row;
}

test("ST56 録画: PC で設定した後の記録が 1 日の画面に届く", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/");
  await page.reload();
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
  const stay = page.getByTestId("row-stay").last();
  await expect(stay).toBeVisible({ timeout: 15_000 });
  await stay.getByRole("button", { expanded: false }).first().click();
  await expect(stay.getByTestId("stay-detail")).toBeVisible();
  const row = await reveal(stay);
  await expect(row, `「${TITLE}」の PC の記録が無い`).toBeVisible();
  await row.scrollIntoViewIfNeeded();
  await expect(stay.getByTestId("stay-detail").getByText(/^PC/).first()).toBeVisible();
  await hold(page, 3);
});
