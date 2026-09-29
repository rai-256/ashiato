// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page } from "@playwright/test";

const DAY = "2026-09-07";
const DAY_URL = `/#/day/${DAY}`;

async function openDay(page: Page): Promise<void> {
  await page.goto(DAY_URL);
  await page.reload();
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
  await expect(page.getByTestId("day-view")).toBeVisible();
}

async function openFirstStay(page: Page): Promise<{ row: Locator; range: string }> {
  const row = page.getByTestId("row-stay").first();
  await expect(row).toBeVisible();
  const range = (await row.locator("h2").innerText()).trim();
  await row.getByRole("button").click();
  await expect(row.getByTestId("stay-detail")).toBeVisible();
  await expect(row.getByRole("button", { name: "この滞在を消す" })).toBeVisible();
  return { row, range };
}

// Scenario: キーボードで詳細を開ける
test("キーボードで滞在の行を開ける", async ({ page }) => {
  await openDay(page);
  const row = page.getByTestId("row-stay").first();
  await expect(row).toBeVisible();
  const toggle = row.getByRole("button");
  await toggle.focus();
  await page.keyboard.press("Enter");
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(row.getByTestId("stay-detail")).toBeVisible();
});

// Scenario: 詳細の末尾の消す操作は 44 px を下回らない
test("詳細の末尾の「この滞在を消す」は幅と高さが 44 CSS px 以上ある", async ({ page }) => {
  await openDay(page);
  const { row } = await openFirstStay(page);

  const box = await row.getByRole("button", { name: "この滞在を消す" }).boundingBox();
  expect(box?.width ?? 0, "消去操作の幅が 44 CSS px 未満").toBeGreaterThanOrEqual(44);
  expect(box?.height ?? 0, "消去操作の高さが 44 CSS px 未満").toBeGreaterThanOrEqual(44);
});

// Scenario: 確認して消すとその滞在の行が一覧から消える
test("確認して消すと同じ時刻の消した行になり、戻すと滞在行に戻る", async ({ page }) => {
  await openDay(page);
  const { row, range } = await openFirstStay(page);
  await row.getByRole("button", { name: "この滞在を消す" }).click();
  await expect(row.getByTestId("erase-confirm")).toBeVisible();
  await row.getByRole("button", { name: "消す" }).click();

  const erased = page.getByTestId("row-erased").filter({ hasText: range });
  let restored = false;
  try {
    await expect(page).toHaveURL(/#\/day\/2026-09-07$/);
    await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(0);
    await expect(erased).toHaveCount(1);
    await expect(erased).toContainText("消した");

    await erased.getByRole("button", { name: "戻す" }).click();
    restored = true;
    await expect(page.getByTestId("row-erased").filter({ hasText: range })).toHaveCount(0);
    await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(1);
    await expect(page).toHaveURL(/#\/day\/2026-09-07$/);
  } finally {
    if (!restored) {
      await erased.getByRole("button", { name: "戻す" }).click({ timeout: 5_000 }).catch(() => undefined);
    }
  }
});
