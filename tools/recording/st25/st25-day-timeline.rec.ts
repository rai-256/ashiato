// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST25「1 日を時刻順に見る」を、1 日の画面で撮る:
 * 開いた最初の画面（今日）→ 偽データの日（2026-09-07）の並び → 滞在の行を開く → 畳んだ行を開く →
 * キーボード（Tab → Enter）で開く → 稼働状況・マスタ管理への行き先 → 幅 360 px → ライト → ダーク。
 *
 * 偽データは `tools/record-env.sh` が立てる `SEED=normal`（`e2e/day-timeline.spec.ts` と同じ日を使う）。
 * アサーションは `e2e/day-timeline.spec.ts` の主張の写し（滞在 9 件・閉じた行が 240 px 以下・横にはみ出さない・
 * 行き先の URL・背景の明暗）。足したのは見るための停止（HOLD）だけ。
 */
declare const process: { env: Record<string, string | undefined> };

const DAY = "2026-09-07";
const PHONE = { width: 360, height: 640 };
const DESKTOP = { width: 1280, height: 720 };
const ROW_MAX_PX = 240; // design D15（仮）。e2e と同じ値をリテラルで持つ
const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

async function openDay(page: Page, hash: string): Promise<void> {
  // ハッシュだけの遷移は読み直さない（e2e と同じ）
  await page.goto(`/${hash}`);
  await page.reload();
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
}

const rows = (page: Page): Locator => page.getByRole("list", { name: "1 日の並び" }).locator("> li");

/** 背景色（rgb）の輝度 0〜255 の目安（e2e と同じ式） */
async function luminance(page: Page, testId: string): Promise<number> {
  const bg = await page.getByTestId(testId).evaluate((el) => getComputedStyle(el).backgroundColor);
  const m = bg.match(/\d+(\.\d+)?/g)?.map(Number) ?? [];
  expect(m.length, `背景色を読めない: ${bg}`).toBeGreaterThanOrEqual(3);
  return 0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2];
}

/** 滞在の行を先頭から開き、畳んだ行（同じアプリの連続）を持つ最初の滞在を開いたまま返す */
async function openStayWithFold(page: Page): Promise<Locator> {
  const stays = page.getByTestId("row-stay");
  const n = await stays.count();
  for (let i = 0; i < n; i++) {
    const stay = stays.nth(i);
    await stay.getByRole("button", { expanded: false }).first().click();
    await expect(stay.getByTestId("stay-detail")).toBeVisible();
    await expect(stay.getByText("記録を読み込み中…")).toHaveCount(0);
    if ((await stay.getByTestId("records").getByRole("button").count()) > 0) return stay;
    await stay.getByRole("button", { expanded: true }).first().click();
  }
  throw new Error("畳んだ行のある滞在が無い");
}

test("ST25 録画: 1 日の画面を時刻順に見る（開く・畳んだ行・行き先・電話の幅・明暗）", async ({ page, browser, browserName }) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  // 見るための停止と操作の間隔（slowMo）で、既定の 30 秒を使い切る
  test.setTimeout(120_000);
  await page.emulateMedia({ colorScheme: "dark" });

  await test.step("1. 画面を開いた最初の状態（今日の 1 日）", async () => {
    await openDay(page, "");
    await expect(page.getByTestId("day-view")).toBeVisible();
    await hold(page, 2);
  });

  await test.step(`2. ${DAY} の 1 日: 滞在 9 件が時刻順に並ぶ（記録 1 件ごとの行は出ない）`, async () => {
    await openDay(page, `#/day/${DAY}`);
    await expect(page.getByTestId("day-view")).toBeVisible();
    await expect(page.getByRole("list", { name: "1 日の並び" }).getByTestId("row-stay")).toHaveCount(9);
    const n = await rows(page).count();
    expect(n).toBeGreaterThan(0);
    expect(n).toBeLessThan(100);
    await hold(page, 2);
    await page.mouse.wheel(0, 600);
    await hold(page);
    await page.mouse.wheel(0, -600);
  });

  await test.step("3. 滞在の行を開くと、その中の記録が時刻順に出る（同じアプリの連続は畳んである）", async () => {
    const stay = await openStayWithFold(page);
    await stay.scrollIntoViewIfNeeded();
    await hold(page, 2);
    const fold = stay.getByTestId("records").getByRole("button").first();
    await fold.scrollIntoViewIfNeeded();
    await fold.click();
    await expect(stay.getByTestId("fold-records")).toBeVisible();
    // 畳んだ行の文字の末尾は「<件数> 件」で、開くと同じ数の記録が出る
    const label = await stay.getByTestId("records").getByRole("button", { expanded: true }).first().innerText();
    const folded = Number(/(\d+) 件\s*$/.exec(label)?.[1] ?? NaN);
    expect(folded, `畳んだ行の件数を読めない: ${label}`).toBeGreaterThan(1);
    expect(await stay.getByTestId("fold-records").locator("li").count()).toBe(folded);
    await hold(page, 2);
    // 閉じて元の並びに戻す
    await stay.getByRole("button", { expanded: true }).first().click();
    await expect(stay.getByTestId("records")).toHaveCount(0);
    await page.evaluate(() => window.scrollTo(0, 0));
    await hold(page);
  });

  await test.step("4. キーボードだけで開く（Tab で滞在の行へ → Enter。フォーカスの輪郭が見える）", async () => {
    const first = page.getByTestId("row-stay").first();
    const button = first.getByRole("button").first();
    await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    let reached = false;
    for (let i = 0; i < 80 && !reached; i++) {
      await page.keyboard.press("Tab");
      reached = await button.evaluate((el) => el === document.activeElement);
    }
    expect(reached, "Tab で滞在の行へ届かなかった").toBe(true);
    const outline = await page.evaluate(() => getComputedStyle(document.activeElement as HTMLElement).outlineStyle);
    expect(outline, "滞在の行に輪郭が無い").not.toBe("none");
    await hold(page);
    await page.keyboard.press("Enter");
    await expect(first.getByTestId("stay-detail")).toBeVisible();
    await hold(page, 2);
    await page.keyboard.press("Enter");
    await expect(first.getByTestId("stay-detail")).toHaveCount(0);
  });

  await test.step("5. 行き先「稼働状況」で稼働状況の画面へ移り、戻る", async () => {
    await page.getByRole("link", { name: "稼働状況" }).click();
    await expect(page).toHaveURL(/#\/coverage$/);
    await expect(page.locator("[data-role=grid]").first()).toBeVisible({ timeout: 15_000 });
    await hold(page, 2);
    await page.goBack();
    await expect(page.getByTestId("day-view")).toBeVisible({ timeout: 15_000 });
  });

  await test.step("6. 行き先「マスタ管理」でマスタ管理の画面へ移り、戻る", async () => {
    await page.getByRole("link", { name: "マスタ管理" }).click();
    await expect(page).toHaveURL(/#\/master$/);
    await expect(page.getByTestId("master-view")).toBeVisible({ timeout: 15_000 });
    await hold(page, 2);
    await page.goBack();
    await expect(page.getByTestId("day-view")).toBeVisible({ timeout: 15_000 });
  });

  await test.step("7. 幅 360 px（電話）: 横にはみ出さず、閉じた行はどれも 240 px 以下", async () => {
    await page.setViewportSize(PHONE);
    await openDay(page, `#/day/${DAY}`);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    const n = await rows(page).count();
    for (let i = 0; i < n; i++) {
      const box = await rows(page).nth(i).boundingBox();
      expect(box?.height ?? 0, `${i} 番目の行`).toBeLessThanOrEqual(ROW_MAX_PX);
    }
    await hold(page, 3);
    await page.mouse.wheel(0, 640);
    await hold(page, 2);
    await page.evaluate(() => window.scrollTo(0, 0));
  });

  await test.step("8. OS をライトに切り替えると明るくなり、ダークに戻すと暗くなる", async () => {
    const dark = await luminance(page, "day-view");
    await page.emulateMedia({ colorScheme: "light" });
    await expect.poll(() => luminance(page, "day-view")).toBeGreaterThan(dark + 100);
    await hold(page, 2);
    await page.emulateMedia({ colorScheme: "dark" });
    await expect.poll(() => luminance(page, "day-view")).toBeLessThan(60);
    await hold(page, 2);
    await page.setViewportSize(DESKTOP);
  });
});
