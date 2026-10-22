import { expect, test } from "@playwright/test";

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
