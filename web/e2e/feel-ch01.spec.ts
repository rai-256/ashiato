import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * CH01（ST17 の /code-review の 9 件）の画面の Scenario。本物のブラウザ・本物のサーバ。ST17 の `feel.spec.ts` には書き足さない。
 * 主観の読み出しを落とすだけで書かないので、DB には何も足さない。
 */

test.use({ viewport: { width: 360, height: 740 }, colorScheme: "dark" });

// Scenario: 読み出しの失敗は日の欄にも出る
test("主観の読み出しが 500 のとき、日の欄と暦の両方に失敗が出て、快–不快のボタンはどれも押されていない", async ({ page }) => {
  await page.route("**/api/feelings?*", (route) => route.fulfill({ status: 500, body: "" }));
  await page.goto("/#/feel/2026-10-03");
  await expect(page.getByTestId("feel-notice")).toContainText("気分を読み出せませんでした");
  await expect(page.getByTestId("feel-calendar-failed")).toContainText("暦を読み出せませんでした");
  for (const v of [-2, -1, 0, 1, 2]) {
    await expect(page.getByTestId(`valence-${v}`)).toHaveAttribute("aria-pressed", "false");
  }
});

/** 断られる応答（1 件ごとの結果つき）。サーバへは何も書かない。 */
const rejectIngest = async (page: Page): Promise<void> => {
  await page.route("**/api/ingest", (route) =>
    route.fulfill({
      status: 400,
      contentType: "application/json",
      body: JSON.stringify([{ id: null, duplicate: false, accepted: false, error: "invalid_feeling_target" }]),
    }),
  );
};

const stayRow = (page: Page): Locator => page.getByTestId("feel-stay-row").first();
const stayBtn = (page: Page, name: string): Locator => stayRow(page).getByRole("button", { name, exact: true });

async function openStays(page: Page, hash: string): Promise<void> {
  await page.goto(`/${hash}`);
  await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("feel-stays-loading")).toHaveCount(0);
  await expect(page.getByTestId("feel-stay-row").first()).toBeVisible();
}

// Scenario: 滞在の行で断られた値は押された状態に残らない
test("+1 の記録がある滞在の行の「とても不快」が断られると、断られた文が出て、+1 が押された状態に戻り -2 は押されていない", async ({ page }) => {
  await rejectIngest(page);
  await openStays(page, "#/feel/2026-09-11");
  await expect(stayBtn(page, "+1 快")).toHaveAttribute("aria-pressed", "true");
  await stayBtn(page, "-2 とても不快").click();
  await expect(page.getByTestId("feel-notice-bad")).toContainText("書けませんでした");
  await expect(stayBtn(page, "+1 快")).toHaveAttribute("aria-pressed", "true");
  await expect(stayBtn(page, "-2 とても不快")).toHaveAttribute("aria-pressed", "false");
});

// Scenario: 記録の無い滞在の行で断られるとどれも押されていない
test("記録の無い滞在の行の「快」が断られると、断られた文が出て、その行のどのボタンも押されていない", async ({ page }) => {
  await rejectIngest(page);
  // 滞在に付く記録を外した応答にして、記録の無い行にする
  await page.route("**/api/feelings?*", async (route) => {
    const res = await route.fetch();
    const body = (await res.json()) as { days: { stays: unknown[]; stays_with_records: number; hidden_stay_records: number }[] };
    for (const d of body.days) Object.assign(d, { stays: [], stays_with_records: 0, hidden_stay_records: 0 });
    await route.fulfill({ response: res, json: body });
  });
  await openStays(page, "#/feel/2026-09-11");
  await stayBtn(page, "+1 快").click();
  await expect(page.getByTestId("feel-notice-bad")).toContainText("書けませんでした");
  for (const name of ["-2 とても不快", "-1 不快", "0 どちらでもない", "+1 快", "+2 とても快"]) {
    await expect(stayBtn(page, name)).toHaveAttribute("aria-pressed", "false");
  }
});

// Scenario: 滞在の行で届かなかった値は押された状態に残る
test("要求が届かない状態で滞在の行の「快」を押すと、届かなかった文が出て、+1 が押された状態で残る", async ({ page }) => {
  await page.route("**/api/ingest", (route) => route.abort("failed"));
  await page.route("**/api/feelings?*", async (route) => {
    const res = await route.fetch();
    const body = (await res.json()) as { days: { stays: unknown[]; stays_with_records: number; hidden_stay_records: number }[] };
    for (const d of body.days) Object.assign(d, { stays: [], stays_with_records: 0, hidden_stay_records: 0 });
    await route.fulfill({ response: res, json: body });
  });
  await openStays(page, "#/feel/2026-09-11");
  await stayBtn(page, "+1 快").click();
  const bad = page.getByTestId("feel-notice-bad");
  await expect(bad).toContainText("サーバに届きませんでした");
  await expect(bad).not.toContainText("書けませんでした");
  await expect(stayBtn(page, "+1 快")).toHaveAttribute("aria-pressed", "true");
});
