// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page, type Request, type Response } from "@playwright/test";

/**
 * S-3 主観入力の画面（ST17 / tasks 8.2 / design D12）—— **押した瞬間に保存する**。本物のブラウザ・本物のサーバ。
 *
 * **主観の行は DB の錠で消せない**ので、書く検査は実行ごとに乱数で選んだ 2030 年以降の日を `page.clock` の
 * 「今日」にして書き、件数は書く前との差分で数える（偽データの日には書かない）。
 */

test.use({ viewport: { width: 360, height: 740 }, colorScheme: "dark" });

// `@types/node` は入れない（playwright.config.ts と同じ）
declare const process: { env: Record<string, string | undefined> };

/** 試験の中で `/api` を直に叩く要求は API の合言葉（Bearer）で通す（ログインの印は `secure` で、画面の外の要求には付かない）。 */
const BEARER = { Authorization: `Bearer ${process.env.API_TOKEN ?? "dev-token-0123456789abcdef"}` };

const DAY_MS = 86_400_000;
const isIngest = (r: Response): boolean => r.url().includes("/api/ingest") && r.request().method() === "POST";
const isIngestReq = (r: Request): boolean => r.url().includes("/api/ingest") && r.method() === "POST";

/** 2030-01-01 から約 8 年の中の、実行ごとに違う日。 */
function freshDay(): string {
  return new Date(Date.UTC(2030, 0, 1) + Math.floor(Math.random() * 3000) * DAY_MS).toISOString().slice(0, 10);
}
const shift = (day: string, n: number): string => new Date(Date.parse(`${day}T12:00:00Z`) + n * DAY_MS).toISOString().slice(0, 10);
const at = (day: string, hm: string): Date => new Date(`${day}T${hm}:00+09:00`);

interface Stored {
  scales: { id: string; value: number }[];
  note: string | null;
  phase: "before" | "after";
}
async function records(page: Page, day: string): Promise<Stored[]> {
  const res = await page.request.get(`/api/feelings?from=${day}&to=${day}`, { headers: BEARER });
  expect(res.status()).toBe(200);
  const body = (await res.json()) as { days: { day: { records: Stored[] } }[] };
  return body.days[0]?.day.records ?? [];
}

async function ready(page: Page): Promise<void> {
  await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
}

/** 時計を `time` に置いて（以後は進む）、アドレスを開く。 */
async function openAt(page: Page, time: Date, hash: string): Promise<void> {
  await page.clock.install({ time });
  await page.goto(`/${hash}`);
  await ready(page);
}

/** 押して、保存の応答（200）が返るまで待つ。 */
async function pressSaved(page: Page, testid: string): Promise<void> {
  const [res] = await Promise.all([page.waitForResponse(isIngest), page.getByTestId(testid).click()]);
  expect(res.status()).toBe(200);
}

function countIngest(page: Page): { n: () => number } {
  let n = 0;
  page.on("request", (r) => {
    if (isIngestReq(r)) n += 1;
  });
  return { n: () => n };
}

const EMOJI = "😀";

// Scenario: 通知から開いて 1 回押すと保存される
test("幅 360 px・暗で今日のアドレスを開いて快を 1 回押すと、保存の応答が返り、記録が 1 件増え、保存と記入の日時が出る", async ({ page }) => {
  const day = freshDay();
  const started = Date.now();
  const before = (await records(page, day)).length;
  await page.clock.install({ time: at(day, "12:00") });
  await page.goto("/#/feel");
  await ready(page);
  await pressSaved(page, "valence-1");
  expect(Date.now() - started, "ページを開いてから保存の応答まで 30 秒を下回る").toBeLessThan(30_000);

  const after = await records(page, day);
  expect(after.length - before).toBe(1);
  expect(after[0].scales.find((s) => s.id === "valence")?.value).toBe(1);
  const ok = page.getByTestId("feel-notice-ok");
  await expect(ok).toContainText("保存しました");
  await expect(ok).toContainText("+1 快");
  await expect(ok).toContainText("記入");
  await expect(ok).toContainText(`対象の日 ${day}`);
});

// Scenario: 日付の無いアドレスは夜中も暦の今日を開く
test("Asia/Tokyo の 00:30 に日付の無いアドレスを開くと、見出しの日付は暦の今日", async ({ page }) => {
  const day = freshDay();
  await openAt(page, at(day, "00:30"), "#/feel");
  await expect(page.getByTestId("feel-title").locator("time")).toHaveAttribute("datetime", day);
});

// Scenario: 暦に無い日付のアドレスは書く操作を出さない
test("暦に無い日付のアドレスは、日付が読めないことと今日への入口を出し、快–不快のボタンを置かない", async ({ page }) => {
  await page.goto("/#/feel/2026-13-45");
  await expect(page.getByTestId("feel-invalid")).toBeVisible();
  await expect(page.getByRole("alert")).toContainText("2026-13-45");
  await expect(page.getByTestId("valence-1")).toHaveCount(0);
  await page.getByRole("link", { name: "今日の気分を書く" }).click();
  await expect(page).toHaveURL(/#\/feel$/);
  await ready(page);
});

// Scenario: 保存したら対象の日が出る
test("翌日に前日の画面で快–不快を押すと、保存と対象の日と記入の日時が出る", async ({ page }) => {
  const day = freshDay();
  const before = (await records(page, day)).length;
  await openAt(page, at(shift(day, 1), "12:00"), `#/feel/${day}`);
  await pressSaved(page, "valence-1");
  const ok = page.getByTestId("feel-notice-ok");
  await expect(ok).toContainText("保存しました");
  await expect(ok).toContainText(`対象の日 ${day}`);
  await expect(ok).toContainText("記入");
  expect((await records(page, day)).length - before).toBe(1);
});

// Scenario: 昨日の分は前の日へ 1 回で書ける
test("今日の画面で前の日へを 1 回押して快–不快を押すと、昨日の「日」に 1 件入る", async ({ page }) => {
  const day = freshDay();
  const yesterday = shift(day, -1);
  const [t, y] = [(await records(page, day)).length, (await records(page, yesterday)).length];
  await openAt(page, at(day, "12:00"), "#/feel");
  await page.getByRole("button", { name: "前の日へ" }).click();
  await expect(page).toHaveURL(new RegExp(`#/feel/${yesterday}$`));
  await ready(page);
  await expect(page.getByTestId("feel-title").locator("time")).toHaveAttribute("datetime", yesterday);
  await pressSaved(page, "valence-1");
  expect((await records(page, yesterday)).length - y).toBe(1);
  expect((await records(page, day)).length - t).toBe(0);
});

// Scenario: 次の日へ移って未来の日に書ける
test("今日の画面で次の日へを押して快–不快を押すと、明日の「日」に 1 件入る", async ({ page }) => {
  const day = freshDay();
  const tomorrow = shift(day, 1);
  const before = (await records(page, tomorrow)).length;
  await openAt(page, at(day, "12:00"), "#/feel");
  await page.getByRole("button", { name: "次の日へ" }).click();
  await expect(page).toHaveURL(new RegExp(`#/feel/${tomorrow}$`));
  await ready(page);
  await pressSaved(page, "valence-1");
  expect((await records(page, tomorrow)).length - before).toBe(1);
});

// Scenario: 快不快のボタンは数値と言葉を持つ
test("5 つのボタンがそれぞれ数値と言葉を持つ", async ({ page }) => {
  await openAt(page, at(freshDay(), "12:00"), "#/feel");
  const group = page.getByRole("group", { name: "快–不快" });
  const expected = ["-2 とても不快", "-1 不快", "0 どちらでもない", "+1 快", "+2 とても快"];
  await expect(group.getByRole("button")).toHaveCount(5);
  for (const [i, name] of expected.entries()) {
    const button = group.getByRole("button").nth(i);
    await expect(button).toHaveAccessibleName(name);
    const [value, word] = name.split(" ");
    await expect(button).toContainText(value);
    await expect(button).toContainText(word);
  }
});

// Scenario: ひとことと元気さは押さずに見える
test("何も押す前から、ひとことの欄と元気さの 5 つのボタンが見えている", async ({ page }) => {
  await openAt(page, at(freshDay(), "12:00"), "#/feel");
  await expect(page.getByTestId("feel-note")).toBeVisible();
  const energy = page.getByRole("group", { name: "元気さ" }).getByRole("button");
  await expect(energy).toHaveCount(5);
  for (const b of await energy.all()) await expect(b).toBeVisible();
});

// Scenario: ひとことを足すと記録がもう 1 件積まれる
test("快–不快 +1 の後にひとこと「散歩した」を入れて欄を離れると、記録がもう 1 件積まれる", async ({ page }) => {
  const day = freshDay();
  await openAt(page, at(day, "12:00"), "#/feel");
  await pressSaved(page, "valence-1");
  await page.getByTestId("feel-note").fill("散歩した");
  const [res] = await Promise.all([page.waitForResponse(isIngest), page.getByTestId("feel-note").blur()]);
  expect(res.status()).toBe(200);
  const all = await records(page, day);
  expect(all).toHaveLength(2);
  expect(all[0].note).toBe("散歩した");
  expect(all[0].scales.find((s) => s.id === "valence")?.value).toBe(1);
  expect(all[1].note).toBeNull();
});

// Scenario: 元気さを押すと今の快不快と一緒に積まれる
test("快–不快 -1 の後に元気さ +2 を押すと、-1 と +2 の記録がもう 1 件積まれる", async ({ page }) => {
  const day = freshDay();
  await openAt(page, at(day, "12:00"), "#/feel");
  await pressSaved(page, "valence--1");
  await pressSaved(page, "energy-2");
  const all = await records(page, day);
  expect(all).toHaveLength(2);
  expect(all[0].scales.map((s) => [s.id, s.value])).toEqual([
    ["valence", -1],
    ["energy", 2],
  ]);
});

// Scenario: 快不快を選ぶ前は元気さだけを送らない
test("記録の無い日に元気さだけを押しても、記録は送られない", async ({ page }) => {
  const day = freshDay();
  const sends = countIngest(page);
  await openAt(page, at(day, "12:00"), "#/feel");
  await page.getByTestId("energy-2").click();
  await expect(page.getByTestId("energy-2")).toHaveAttribute("aria-pressed", "true");
  await page.waitForTimeout(500);
  expect(sends.n()).toBe(0);
  expect(await records(page, day)).toHaveLength(0);
});

// Scenario: 届かなかった記録は同じ原文で送り直す
test("届かなかった後に「もう一度送る」を押すと、1 回目と同じ原文が送られ、1 件だけ入る", async ({ page }) => {
  const day = freshDay();
  const bodies: (string | null)[] = [];
  await page.route("**/api/ingest", (route) => {
    bodies.push(route.request().postData());
    return bodies.length === 1 ? route.abort("failed") : route.continue();
  });
  await openAt(page, at(day, "12:00"), "#/feel");
  await page.getByTestId("valence-1").click();
  await expect(page.getByTestId("feel-notice-bad")).toContainText("サーバに届きませんでした");
  const [res] = await Promise.all([page.waitForResponse(isIngest), page.getByTestId("feel-resend").click()]);
  expect(res.status()).toBe(200);
  expect(bodies).toHaveLength(2);
  expect(bodies[1]).toBe(bodies[0]);
  expect(await records(page, day)).toHaveLength(1);
});

// Scenario: 届かなかったとき選んだ値とひとことが残る
test("ひとことを入れて届かない状態で快–不快を押すと、届かなかった文が出て、値とひとことが残る", async ({ page }) => {
  await page.route("**/api/ingest", (route) => route.abort("failed"));
  await openAt(page, at(freshDay(), "12:00"), "#/feel");
  await page.getByTestId("feel-note").fill("雨だった");
  await page.getByTestId("valence--2").click();
  const bad = page.getByTestId("feel-notice-bad");
  await expect(bad).toContainText("サーバに届きませんでした");
  await expect(bad).not.toContainText("受け付けられませんでした");
  await expect(page.getByTestId("feel-note")).toHaveValue("雨だった");
  await expect(page.getByTestId("valence--2")).toHaveAttribute("aria-pressed", "true");
});

// Scenario: 長すぎるひとことは送る前に知らせる
test("絵文字 2,001 個で欄の直後に長すぎる知らせが出て、2,000 個に戻すと消える（2,000 個では出ない）", async ({ page }) => {
  const sends = countIngest(page);
  await openAt(page, at(freshDay(), "12:00"), "#/feel");
  const field = page.getByTestId("feel-note");
  const over = page.getByTestId("feel-note-over");
  await field.fill(EMOJI.repeat(2000));
  await expect(over).toHaveCount(0);
  await field.fill(EMOJI.repeat(2001));
  await expect(over).toBeVisible();
  await expect(over).toContainText("2,001 / 2,000 文字");
  await expect(over).toContainText("長すぎ");
  expect(await over.evaluate((el) => el.previousElementSibling?.id)).toBe("feel-note");
  const [f, o] = [await field.boundingBox(), await over.boundingBox()];
  const gap = (o?.y ?? NaN) - ((f?.y ?? NaN) + (f?.height ?? NaN));
  expect(gap, "知らせの上端が欄の下端から 0〜16 px").toBeGreaterThanOrEqual(0);
  expect(gap).toBeLessThanOrEqual(16);
  await field.fill(EMOJI.repeat(2000));
  await expect(over).toHaveCount(0);
  expect(sends.n()).toBe(0);
});

// Scenario: 長すぎるひとことのまま押しても送らない
test("2,001 文字のまま、快・元気さ・欄を離れる・「ひとことを残す」のどれでも送らず、値とひとことが残る", async ({ context }) => {
  const day = freshDay();
  const operations: [string, (page: Page) => Promise<void>][] = [
    ["快を押す", (page) => page.getByTestId("valence-1").click()],
    ["元気さを押す", (page) => page.getByTestId("energy-1").click()],
    ["欄を離れる", (page) => page.getByTestId("feel-note").blur()],
    ["ひとことを残す", (page) => page.getByTestId("feel-keep-note").click()],
  ];
  for (const [name, operate] of operations) {
    // 操作ごとに新しいページ（`page.clock.install` は 1 ページに 1 回）
    const page = await context.newPage();
    const sends = countIngest(page);
    await openAt(page, at(day, "12:00"), "#/feel");
    await page.getByTestId("feel-note").fill(EMOJI.repeat(2001));
    await operate(page);
    const bad = page.getByTestId("feel-notice-bad");
    await expect(bad, name).toContainText("保存していません");
    await expect(bad).toContainText("長すぎ");
    await expect(bad).not.toContainText("サーバに届きませんでした");
    await expect(page.getByTestId("feel-note")).toHaveValue(EMOJI.repeat(2001));
    if (name === "快を押す") await expect(page.getByTestId("valence-1")).toHaveAttribute("aria-pressed", "true");
    if (name === "元気さを押す") await expect(page.getByTestId("energy-1")).toHaveAttribute("aria-pressed", "true");
    await page.waitForTimeout(300);
    expect(sends.n(), name).toBe(0);
    await page.close();
  }
  const page = await context.newPage();
  expect(await records(page, day)).toHaveLength(0);
  await page.close();
});

// Scenario: 断られたとき理由の文が出る
test("ひとことが長すぎるとして断られると、その文が出て、入れたひとことが残る", async ({ page }) => {
  await page.route("**/api/ingest", (route) =>
    route.fulfill({
      status: 400,
      contentType: "application/json",
      body: JSON.stringify([{ id: null, duplicate: false, accepted: false, error: "invalid_note" }]),
    }),
  );
  await openAt(page, at(freshDay(), "12:00"), "#/feel");
  await page.getByTestId("feel-note").fill("断られる");
  await page.getByTestId("valence-1").click();
  const bad = page.getByTestId("feel-notice-bad");
  await expect(bad).toContainText("ひとことが長すぎます（2,000 文字まで）");
  await expect(bad).not.toContainText("サーバに届きませんでした");
  await expect(page.getByTestId("feel-note")).toHaveValue("断られる");
});

// Scenario: 開くと最新の記録の値が選ばれている
test("今日の「日」に +1、続けて -1 の記録があるとき、開くと -1 が押された状態で出る", async ({ page }) => {
  const day = freshDay();
  await openAt(page, at(day, "12:00"), "#/feel");
  await pressSaved(page, "valence-1");
  await pressSaved(page, "valence--1");
  await page.reload();
  await ready(page);
  await expect(page.getByTestId("valence--1")).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("valence-1")).toHaveAttribute("aria-pressed", "false");
});

// Scenario: もう一方の前後の最新は 1 行で添えられる
test("「前」の記録 +2 だけがある日を翌日に開くと、押されたボタンは無く「前に書いた」と +2 が 1 行で出る", async ({ page }) => {
  const day = freshDay();
  await openAt(page, at(day, "12:00"), `#/feel/${day}`);
  await pressSaved(page, "valence-2");
  // 翌日へ時計を進めて開き直す（対象の日の終わりを過ぎたので、いま書けば「後」）
  await page.clock.setSystemTime(at(shift(day, 1), "12:00"));
  await page.reload();
  await ready(page);
  for (const v of ["-2", "-1", "0", "1", "2"]) {
    await expect(page.getByTestId(`valence-${v}`)).toHaveAttribute("aria-pressed", "false");
  }
  const line = page.getByTestId("feel-other");
  await expect(line).toContainText("前に書いた");
  await expect(line).toContainText("+2");
});

// Scenario: 書き直した履歴を開くと全部が出る
test("記録が 3 件ある日で「書き直した履歴 3 件」を押すと、3 件が記入の日時の新しい順に出る", async ({ page }) => {
  const day = freshDay();
  await openAt(page, at(day, "12:00"), "#/feel");
  await pressSaved(page, "valence-1");
  await page.clock.setSystemTime(at(day, "12:05"));
  await pressSaved(page, "valence--1");
  await page.clock.setSystemTime(at(day, "12:10"));
  await pressSaved(page, "valence-2");
  await page.reload();
  await ready(page);
  await page.getByRole("button", { name: "書き直した履歴 3 件" }).click();
  const rows = page.getByTestId("feel-history-row");
  await expect(rows).toHaveCount(3);
  await expect(rows.nth(0)).toContainText("+2 とても快");
  await expect(rows.nth(1)).toContainText("-1 不快");
  await expect(rows.nth(2)).toContainText("+1 快");
  for (const row of await rows.all()) await expect(row).toContainText("記入");
});

// Scenario: 前後を選ぶ操作が無い
test("前に書くか後に書くかを選ぶ操作が無い", async ({ page }) => {
  await openAt(page, at(freshDay(), "12:00"), "#/feel");
  await expect(page.getByRole("radio")).toHaveCount(0);
  await expect(page.getByRole("combobox")).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^(前|後)$|前に書|後に書/ })).toHaveCount(0);
});

// Scenario: 主観を書き換える操作が無い
test("履歴の各行を押しても入力欄は増えず、書き換えや削除の要求を送る操作は無い", async ({ page }) => {
  const day = freshDay();
  const sends = countIngest(page);
  const writes: string[] = [];
  await openAt(page, at(day, "12:00"), "#/feel");
  await pressSaved(page, "valence-1");
  await pressSaved(page, "valence-2");
  await page.reload();
  await ready(page);
  page.on("request", (r) => {
    if (r.method() !== "GET") writes.push(`${r.method()} ${r.url()}`);
  });
  const textboxes = await page.getByRole("textbox").count();
  await page.getByRole("button", { name: "書き直した履歴 2 件" }).click();
  for (const row of await page.getByTestId("feel-history-row").all()) await row.click();
  await expect(page.getByRole("textbox")).toHaveCount(textboxes);
  await expect(page.getByRole("button", { name: /書き換|削除|消す|編集/ })).toHaveCount(0);
  await expect(page.getByTestId("feel-history").getByRole("button")).toHaveCount(0);
  expect(writes).toEqual([]);
  expect(sends.n()).toBe(2);
});
