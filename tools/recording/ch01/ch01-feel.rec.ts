// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page, type Response } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。CH01（ST17 の /code-review の 9 件）で変えた主観の画面を、幅 360 px・暗で撮る:
 * 主観の読み出しの失敗が日の欄と暦に出る → 滞在の行で断られた値は印に残らない（記録あり・記録なし）→ 届かなかった値は印に残る →
 * 開いたまま日の終わりをまたぐ（23:58 に +1、00:01 に -1 で「振り返りとして残しました」）→ 届かずに送り直した記録は知らせない。
 *
 * 断る・届かない・読み出しの失敗は `page.route` の差し替えで、DB には書かない。日の終わりをまたぐ工程は本物のサーバへ書くので、
 * 実行ごとに乱数で選んだ 2100 年以降の日を `page.clock` の今日にする（時計はページを開く前に 1 回だけ入れる）。
 * アサーションは `web/e2e/feel-ch01.spec.ts` の主張の写し。足したのは見るための停止（HOLD）だけ。
 */
declare const process: { env: Record<string, string | undefined> };

test.use({ viewport: { width: 360, height: 740 }, colorScheme: "dark" });

const BEARER = { Authorization: `Bearer ${process.env.API_TOKEN ?? "dev-token-0123456789abcdef"}` };
const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const DAY_MS = 86_400_000;

const isIngest = (r: Response): boolean => r.url().includes("/api/ingest") && r.request().method() === "POST";
const shift = (day: string, n: number): string => new Date(Date.parse(`${day}T12:00:00Z`) + n * DAY_MS).toISOString().slice(0, 10);
const at = (day: string, hm: string): Date => new Date(`${day}T${hm}:00+09:00`);

/** 2100-01-01 から約 8 年の中の、実行ごとに違う日（e2e と同じ）。 */
function freshDay(): string {
  return shift("2100-01-01", Math.floor(Math.random() * 3000));
}

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
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

/** 断られる応答（1 件ごとの結果つき）。サーバへは何も書かない。 */
async function rejectIngest(page: Page): Promise<void> {
  await page.route("**/api/ingest", (route) =>
    route.fulfill({
      status: 400,
      contentType: "application/json",
      body: JSON.stringify([{ id: null, duplicate: false, accepted: false, error: "invalid_feeling_target" }]),
    }),
  );
}

/** 滞在に付く記録を外した応答にして、記録の無い行にする。 */
async function withoutStayRecords(page: Page): Promise<void> {
  await page.route("**/api/feelings?*", async (route) => {
    const res = await route.fetch();
    const body = (await res.json()) as { days: { stays: unknown[]; stays_with_records: number; hidden_stay_records: number }[] };
    for (const d of body.days) Object.assign(d, { stays: [], stays_with_records: 0, hidden_stay_records: 0 });
    await route.fulfill({ response: res, json: body });
  });
}

const stayRow = (page: Page): Locator => page.getByTestId("feel-stay-row").first();
const stayBtn = (page: Page, name: string): Locator => stayRow(page).getByRole("button", { name, exact: true });

/** 画面を開き直す。ハッシュだけの遷移は読み直さないので、差し替えた応答が効くよう読み込み直す（e2e は毎回まっさらなページ）。 */
async function openFresh(page: Page, hash: string): Promise<void> {
  await page.goto(`/${hash}`);
  await page.reload();
  await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("feel-stays-loading")).toHaveCount(0);
}

async function openStays(page: Page, hash: string): Promise<void> {
  await openFresh(page, hash);
  await expect(stayRow(page)).toBeVisible();
  await stayRow(page).scrollIntoViewIfNeeded();
}

async function pressSaved(page: Page, testid: string): Promise<void> {
  const [res] = await Promise.all([page.waitForResponse(isIngest), page.getByTestId(testid).click()]);
  expect(res.status()).toBe(200);
}

test("CH01 録画: 主観の画面の押した印と前/後を記録に合わせる（読み出しの失敗・滞在の行の断り・日の終わりをまたぐ）", async ({ page, browser, browserName }) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  test.setTimeout(240_000);
  const day = freshDay();
  await page.clock.install({ time: at(day, "21:00") });

  await test.step("1. 主観の読み出しが 500 のとき、日の欄と暦の両方に失敗が出て、快–不快のボタンはどれも押されていない", async () => {
    await page.route("**/api/feelings?*", (route) => route.fulfill({ status: 500, body: "" }));
    await page.goto("/#/feel/2026-10-03");
    await expect(page.getByTestId("feel-notice")).toContainText("気分を読み出せませんでした");
    await expect(page.getByTestId("feel-calendar-failed")).toContainText("暦を読み出せませんでした");
    for (const v of [-2, -1, 0, 1, 2]) {
      await expect(page.getByTestId(`valence-${v}`)).toHaveAttribute("aria-pressed", "false");
    }
    await hold(page, 2);
    await page.unrouteAll({ behavior: "ignoreErrors" });
  });

  await test.step("2. +1 の記録がある滞在の行（09-11）で「とても不快」が断られると、断られた文が出て、+1 の印に戻る", async () => {
    await rejectIngest(page);
    await openStays(page, "#/feel/2026-09-11");
    await expect(stayBtn(page, "+1 快")).toHaveAttribute("aria-pressed", "true");
    await hold(page);
    await stayBtn(page, "-2 とても不快").click();
    await expect(page.getByTestId("feel-notice-bad")).toContainText("書けませんでした");
    await expect(stayBtn(page, "+1 快")).toHaveAttribute("aria-pressed", "true");
    await expect(stayBtn(page, "-2 とても不快")).toHaveAttribute("aria-pressed", "false");
    await hold(page, 2);
    await page.unrouteAll({ behavior: "ignoreErrors" });
  });

  await test.step("3. 記録の無い滞在の行で「快」が断られると、その行のどのボタンも押されていない", async () => {
    await rejectIngest(page);
    await withoutStayRecords(page);
    await openStays(page, "#/feel/2026-09-11");
    await stayBtn(page, "+1 快").click();
    await expect(page.getByTestId("feel-notice-bad")).toContainText("書けませんでした");
    for (const name of ["-2 とても不快", "-1 不快", "0 どちらでもない", "+1 快", "+2 とても快"]) {
      await expect(stayBtn(page, name)).toHaveAttribute("aria-pressed", "false");
    }
    await hold(page, 2);
    await page.unrouteAll({ behavior: "ignoreErrors" });
  });

  await test.step("4. 要求が届かない状態で滞在の行の「快」を押すと、届かなかった文が出て、+1 の印が残る", async () => {
    await page.route("**/api/ingest", (route) => route.abort("failed"));
    await withoutStayRecords(page);
    await openStays(page, "#/feel/2026-09-11");
    await stayBtn(page, "+1 快").click();
    const bad = page.getByTestId("feel-notice-bad");
    await expect(bad).toContainText("サーバに届きませんでした");
    await expect(bad).not.toContainText("書けませんでした");
    await expect(stayBtn(page, "+1 快")).toHaveAttribute("aria-pressed", "true");
    await hold(page, 2);
    await page.unrouteAll({ behavior: "ignoreErrors" });
  });

  await test.step("5. 23:58 に開いて 23:59 に「快」を押すと「前」として残り、振り返りの知らせは出ない", async () => {
    const before = (await stored(page, day)).length;
    await page.clock.setSystemTime(at(day, "23:58"));
    // 押した後の読み直しが済んでから時計を進める（e2e と同じ）
    await openFresh(page, `#/feel/${day}`);
    await hold(page);
    await page.clock.setSystemTime(at(day, "23:59"));
    const reread = page.waitForResponse((r) => r.url().includes("/api/feelings?") && r.request().method() === "GET");
    await pressSaved(page, "valence-1");
    await reread;
    await expect(page.getByTestId("feel-notice-ok")).toContainText("保存しました");
    await expect(page.getByTestId("feel-notice-ok")).not.toContainText("振り返りとして残しました");
    await expect(page.getByTestId("valence-1")).toHaveAttribute("aria-pressed", "true");
    const after = await stored(page, day);
    expect(after).toHaveLength(before + 1);
    expect(after[0].phase).toBe("before");
    await hold(page, 2);
  });

  await test.step("6. 開いたまま翌日 00:01 に「不快」を押すと「後」として残り、「振り返りとして残しました」と「前に書いた」+1 が見える", async () => {
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
    await hold(page, 3);
  });

  await test.step("7. 別の日を 23:59 に届かない状態で「快」を押し、00:01 に送り直すと「前」として残り、振り返りの知らせは出ない", async () => {
    const other = shift(day, 2);
    const before = (await stored(page, other)).length;
    await page.clock.setSystemTime(at(other, "23:59"));
    await openFresh(page, `#/feel/${other}`);
    await page.route("**/api/ingest", (route) => route.abort("failed"));
    await page.getByTestId("valence-1").click();
    await expect(page.getByTestId("feel-notice-bad")).toContainText("サーバに届きませんでした");
    await hold(page, 2);
    await page.unroute("**/api/ingest");
    await page.clock.setSystemTime(at(shift(other, 1), "00:01"));
    const [res] = await Promise.all([page.waitForResponse(isIngest), page.getByTestId("feel-resend").click()]);
    expect(res.status()).toBe(200);
    await expect(page.getByTestId("feel-notice-ok")).toContainText("保存しました");
    await expect(page.getByTestId("feel-notice-ok")).not.toContainText("振り返りとして残しました");
    const after = await stored(page, other);
    expect(after).toHaveLength(before + 1);
    expect(after[0].phase).toBe("before");
    await hold(page, 3);
  });
});
