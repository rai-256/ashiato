import { expect, test, type Locator, type Page, type Response } from "@playwright/test";

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

// ---------------------------------------------------------------- 開いたまま日の終わりをまたぐ（書く。実行ごとに乱数で選んだ日）

declare const process: { env: Record<string, string | undefined> };
const BEARER = { Authorization: `Bearer ${process.env.API_TOKEN ?? "dev-token-0123456789abcdef"}` };
const DAY_MS = 86_400_000;
const isIngest = (r: Response): boolean => r.url().includes("/api/ingest") && r.request().method() === "POST";
const shift = (day: string, n: number): string => new Date(Date.parse(`${day}T12:00:00Z`) + n * DAY_MS).toISOString().slice(0, 10);
const at = (day: string, hm: string): Date => new Date(`${day}T${hm}:00+09:00`);

/** 2100-01-01 から約 8 年の中の、実行ごとに違う日（`feel-parts.spec.ts` の `freshDay()` と同じ形）。 */
function freshDay(): string {
  return shift("2100-01-01", Math.floor(Math.random() * 3000));
}

interface Stored {
  scales: { id: string; value: number }[];
  phase: "before" | "after";
}
async function stored(page: Page, day: string): Promise<Stored[]> {
  const res = await page.request.get(`/api/feelings?from=${day}&to=${day}`, { headers: BEARER });
  expect(res.status()).toBe(200);
  const body = (await res.json()) as { days: { day: { records: Stored[] } }[] };
  return body.days[0]?.day.records ?? [];
}
const valenceOf = (r: Stored): number | undefined => r.scales.find((s) => s.id === "valence")?.value;

/** 23:58 に対象の日の画面を開く。時計はここから進み、`setSystemTime` で飛ばせる。 */
async function openAtNight(page: Page, day: string): Promise<void> {
  await page.clock.install({ time: at(day, "23:58") });
  await page.goto(`/#/feel/${day}`);
  await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
}

async function pressSaved(page: Page, testid: string): Promise<void> {
  const [res] = await Promise.all([page.waitForResponse(isIngest), page.getByTestId(testid).click()]);
  expect(res.status()).toBe(200);
}

// Scenario: 開いたまま日の終わりをまたいで押した記録は後として格納される
test("23:58 に開いたまま翌日 00:01 に「不快」を押すと、その記録は対象の日の「後」として格納される", async ({ page }) => {
  const day = freshDay();
  const before = (await stored(page, day)).length;
  await openAtNight(page, day);
  await page.clock.setSystemTime(at(shift(day, 1), "00:01"));
  await pressSaved(page, "valence--1");
  const after = await stored(page, day);
  expect(after).toHaveLength(before + 1);
  expect(after[0].phase).toBe("after");
  expect(valenceOf(after[0])).toBe(-1);
});

// Scenario: 開いたまま日の終わりをまたいで押すと振り返りとして残したと出る
test("「前」に +1 がある日を 23:58 に開き 00:01 に「不快」を押すと、振り返りとして残したと出て、-1 が押され、「前に書いた」と +1 が見える", async ({ page }) => {
  const day = freshDay();
  await openAtNight(page, day);
  // 押した後の読み直しが済んでから時計を進める（読み直しは画面の「いま」を取り直すので、進めた後に済むと「後」を出してしまう）
  const reread = page.waitForResponse((r) => r.url().includes("/api/feelings?") && r.request().method() === "GET");
  await pressSaved(page, "valence-1");
  await reread;
  await expect(page.getByTestId("valence-1")).toHaveAttribute("aria-pressed", "true");
  await page.clock.setSystemTime(at(shift(day, 1), "00:01"));
  await pressSaved(page, "valence--1");
  const ok = page.getByTestId("feel-notice-ok");
  await expect(ok).toContainText("保存しました");
  await expect(ok).toContainText("振り返りとして残しました");
  await expect(page.getByTestId("valence--1")).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("valence-1")).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByTestId("feel-other")).toContainText("前に書いた");
  await expect(page.getByTestId("feel-other")).toContainText("+1");
  const [latest, earlier] = await stored(page, day);
  expect([latest.phase, valenceOf(latest)]).toEqual(["after", -1]);
  expect([earlier.phase, valenceOf(earlier)]).toEqual(["before", 1]);
});

// Scenario: 日の終わりをまたがずに押すと振り返りの知らせは出ない
test("23:58 に開いて 23:59 に「快」を押すと、記録は「前」として格納され、保存したことが出て、振り返りの知らせは出ない", async ({ page }) => {
  const day = freshDay();
  const before = (await stored(page, day)).length;
  await openAtNight(page, day);
  await page.clock.setSystemTime(at(day, "23:59"));
  await pressSaved(page, "valence-1");
  await expect(page.getByTestId("feel-notice-ok")).toContainText("保存しました");
  await expect(page.getByTestId("feel-notice-ok")).not.toContainText("振り返りとして残しました");
  const after = await stored(page, day);
  expect(after).toHaveLength(before + 1);
  expect(after[0].phase).toBe("before");
});

// Scenario: 日をまたいでから届いた送り直しは振り返りの知らせを出さない
test("23:59 に届かない状態で「快」を押し、00:01 に届く状態に戻して送り直すと、記録は「前」として格納され、振り返りの知らせは出ない", async ({ page }) => {
  const day = freshDay();
  const before = (await stored(page, day)).length;
  await openAtNight(page, day);
  await page.clock.setSystemTime(at(day, "23:59"));
  await page.route("**/api/ingest", (route) => route.abort("failed"));
  await page.getByTestId("valence-1").click();
  await expect(page.getByTestId("feel-notice-bad")).toContainText("サーバに届きませんでした");
  await page.unroute("**/api/ingest");
  await page.clock.setSystemTime(at(shift(day, 1), "00:01"));
  const [res] = await Promise.all([page.waitForResponse(isIngest), page.getByTestId("feel-resend").click()]);
  expect(res.status()).toBe(200);
  await expect(page.getByTestId("feel-notice-ok")).toContainText("保存しました");
  await expect(page.getByTestId("feel-notice-ok")).not.toContainText("振り返りとして残しました");
  const after = await stored(page, day);
  expect(after).toHaveLength(before + 1);
  expect(after[0].phase).toBe("before");
});
