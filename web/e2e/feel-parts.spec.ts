import { expect, test, type Locator, type Page, type Response } from "@playwright/test";

/**
 * S-3 の滞在の行・暦・通知の時刻の設定（ST17 / tasks 9.2 / design D10・D13・D14）。本物のブラウザ・本物のサーバ。
 *
 * 見え方（暦・滞在の行の最新・前の日から・消した滞在・吸収）は**偽データの固定の日**（`tools/seed.sh normal` の主観の節。
 * 2026-09-04〜10-03）に対してだけ assert し、書かない。書く検査（滞在の行を押す・前の日から続く滞在に書く）は、
 * 実行ごとに乱数で選んだ 2100 年以降の日に、偽データの「前の日から続く滞在」と同じ形の位置の記録を足して作る
 * （主観の行は DB の錠で消せない。件数は書く前との差分で数える）。
 */

test.use({ viewport: { width: 360, height: 740 }, colorScheme: "dark" });

// `@types/node` は入れない（playwright.config.ts と同じ）
declare const process: { env: Record<string, string | undefined> };

/** 試験の中で `/api` を直に叩く要求は API の合言葉（Bearer）で通す（ログインの印は `secure` で、画面の外の要求には付かない）。 */
const BEARER = { Authorization: `Bearer ${process.env.API_TOKEN ?? "dev-token-0123456789abcdef"}` };

const DAY_MS = 86_400_000;
const USER = "00000000-0000-0000-0000-000000000000";
const isIngest = (r: Response): boolean => r.url().includes("/api/ingest") && r.request().method() === "POST";

const shift = (day: string, n: number): string => new Date(Date.parse(`${day}T12:00:00Z`) + n * DAY_MS).toISOString().slice(0, 10);
const jstDate = (d: Date): string => new Date(d.getTime() + 9 * 3_600_000).toISOString().slice(0, 10);

/** 2100-01-01 から約 8 年の中の、実行ごとに違う日（実際の今日がいつになっても「これから」の側に残る十分先）。 */
function freshDay(): string {
  return shift("2100-01-01", Math.floor(Math.random() * 3000));
}

async function openOn(page: Page, hash: string): Promise<void> {
  await page.goto(`/${hash}`);
  await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
  // 滞在の読み出しが済むまで待つ（読み込み中の表示が消える）
  await expect(page.getByTestId("feel-stays-loading")).toHaveCount(0);
}

const cal = (page: Page, date: string): Locator => page.getByTestId(`cal-${date}`);
const mark = (page: Page, date: string): Locator => cal(page, date).getByTestId("cal-mark");
const row = (page: Page, i: number): Locator => page.getByTestId("feel-stay-row").nth(i);
/** 行の快–不快のボタン（名前は数値と言葉。他の行・元気さのボタンと混ざらない） */
const btn = (r: Locator, name: string): Locator => r.getByRole("button", { name, exact: true });

// ---------------------------------------------------------------- 偽データの固定の日（書かない）

// Scenario: 暦に未記入の日が破線と未で出る
// Scenario: 使い始める前の日は未記入と出ない
// Scenario: 暦は対象の日で終わる 30 日
test("2026-10-03 の暦は 09-04〜10-03 の 30 日で、未記入の 9 日だけに破線の枠と「未」があり、使い始める前の 2 日には無い", async ({ page }) => {
  await openOn(page, "#/feel/2026-10-03");
  const cells = page.locator('[data-testid^="cal-20"]');
  await expect(cells).toHaveCount(30);
  await expect(cells.first()).toHaveAttribute("data-testid", "cal-2026-09-04");
  await expect(cells.last()).toHaveAttribute("data-testid", "cal-2026-10-03");

  const missing = ["2026-09-08", "2026-09-09", "2026-09-13", "2026-09-16", "2026-09-19", "2026-09-23", "2026-09-27", "2026-10-01", "2026-10-02"];
  for (const d of missing) {
    await expect(cal(page, d)).toHaveCSS("border-top-style", "dashed");
    await expect(mark(page, d)).toHaveText("未");
  }
  const others = (await cells.all()).length - missing.length;
  expect(others).toBe(21);
  for (const c of await cells.all()) {
    const id = (await c.getAttribute("data-testid"))?.replace("cal-", "") ?? "";
    if (missing.includes(id)) continue;
    await expect(c, `${id} に破線は付かない`).not.toHaveCSS("border-top-style", "dashed");
    await expect(c.getByTestId("cal-mark"), `${id} に「未」は付かない`).not.toHaveText("未");
  }
  for (const d of ["2026-09-04", "2026-09-05"]) {
    await expect(cal(page, d)).toHaveAttribute("data-status", "before_start");
    await expect(mark(page, d)).toHaveText("");
  }
});

// Scenario: 暦に記入ありの日の値が出る
// Scenario: 前と後がある日の暦は後の値を出す
test("「日」の最新が +1 の日の欄は +1、前（+1）と後（+2）がある日の欄は後の +2", async ({ page }) => {
  await openOn(page, "#/feel/2026-10-03");
  await expect(mark(page, "2026-09-07")).toHaveText("+1");
  await expect(mark(page, "2026-09-12")).toHaveText("+2");
  await expect(mark(page, "2026-09-14")).toHaveText("-1");
  await expect(cal(page, "2026-09-07")).toHaveAttribute("data-status", "recorded");
});

// Scenario: これからの日は暦で未記入と出ない
test("3 日後を対象にして開くと、今日より後の日に破線の枠も「未」も無い", async ({ page }) => {
  const today = jstDate(new Date());
  const target = shift(today, 3);
  await openOn(page, `#/feel/${target}`);
  for (const d of [shift(today, 1), shift(today, 2), target]) {
    await expect(cal(page, d)).toHaveAttribute("data-status", "future");
    await expect(cal(page, d)).not.toHaveCSS("border-top-style", "dashed");
    await expect(mark(page, d)).toHaveText("");
  }
});

// Scenario: 暦の日を押すとその日を書ける
test("暦の 3 日前の日を押すと、見出しの日付とアドレスがその日になる", async ({ page }) => {
  await openOn(page, "#/feel/2026-10-03");
  await cal(page, "2026-09-30").click();
  await expect(page).toHaveURL(/#\/feel\/2026-09-30$/);
  await expect(page.getByTestId("feel-title").locator("time")).toHaveAttribute("datetime", "2026-09-30");
  await expect(page.getByTestId("valence-1")).toBeVisible();
});

// Scenario: 滞在の行に最新の気分が出る
// Scenario: 滞在の書き直した履歴を開ける
// Scenario: その日の滞在が始まりの順に並ぶ
test("2026-09-15 は滞在 2 件が始まりの順に並び、最新（後の +2 / -1）が押された状態で、書き直しの履歴は新しい順に開く", async ({ page }) => {
  await openOn(page, "#/feel/2026-09-15");
  await expect(page.getByTestId("feel-stay-row")).toHaveCount(2);
  await expect(page.getByTestId("feel-stay-range")).toHaveText(["10:00 – 10:40", "15:00 – 15:40"]);
  await expect(btn(row(page, 0), "+2 とても快")).toHaveAttribute("aria-pressed", "true");
  await expect(btn(row(page, 0), "+1 快")).toHaveAttribute("aria-pressed", "false");
  await expect(btn(row(page, 1), "-1 不快")).toHaveAttribute("aria-pressed", "true");

  await expect(row(page, 1).getByTestId("feel-stay-history-toggle")).toHaveCount(0);
  await row(page, 0).getByRole("button", { name: "書き直した履歴 2 件" }).click();
  const lines = row(page, 0).getByTestId("feel-stay-history-row");
  await expect(lines).toHaveCount(2);
  await expect(lines.nth(0)).toContainText("+2");
  await expect(lines.nth(0)).toContainText("9 月 16 日 7:50");
  await expect(lines.nth(1)).toContainText("+1");
  await expect(lines.nth(1)).toContainText("9 月 15 日 12:30");
});

// Scenario: 前の日から続く滞在の行は前の日からと出る
// Scenario: 前の日から続く滞在に書いた気分はその行に出る
test("2026-09-11 は前の日（09-10 19:00）から続く滞在の行に「前の日から」があり、前の日に書いた +1 がその行に出る", async ({ page }) => {
  await openOn(page, "#/feel/2026-09-11");
  const carried = row(page, 0);
  await expect(carried.getByTestId("feel-stay-carried")).toHaveText("前の日から");
  await expect(carried.getByTestId("feel-stay-range")).toContainText("9/10 19:00");
  await expect(btn(carried, "+1 快")).toHaveAttribute("aria-pressed", "true");
});

// Scenario: 消した滞在は気分の行に並ばない
// Scenario: 消した滞在の気分は数だけ出る
test("2026-09-14 は消した滞在の行が無く、「消した滞在の気分 1 件」だけが出て値は出ない", async ({ page }) => {
  await openOn(page, "#/feel/2026-09-14");
  await expect(page.getByTestId("feel-stay-row")).toHaveCount(0);
  const hidden = page.getByTestId("feel-stays-hidden");
  await expect(hidden).toHaveText("消した滞在の気分 1 件（ここには出しません）");
  await expect(page.getByTestId("feel-stays")).not.toContainText("不快");
});

// Scenario: 吸収された滞在の気分は吸収先の行に出る
test("2026-09-12 は吸収された滞在に書いた 0 が、吸収先の 1 つの行に押された状態で出る", async ({ page }) => {
  await openOn(page, "#/feel/2026-09-12");
  await expect(page.getByTestId("feel-stay-row")).toHaveCount(1);
  await expect(btn(row(page, 0), "0 どちらでもない")).toHaveAttribute("aria-pressed", "true");
});

// Scenario: 滞在の無い日は滞在が無いと出る
test("滞在が 1 件も無い日は、滞在が無いことを示す文が出て、読み出しの失敗の文は出ない", async ({ page }) => {
  await openOn(page, `#/feel/${freshDay()}`);
  await expect(page.getByTestId("feel-stays-empty")).toHaveText("この日の滞在はありません");
  await expect(page.getByTestId("feel-stays-failed")).toHaveCount(0);
});

// Scenario: 滞在の読み出しの失敗は滞在が無いと出ない
test("滞在の読み出しが 500 のとき、失敗が出て、滞在が無いことを示す文は出ない", async ({ page }) => {
  await page.route("**/api/stays?*", (route) => route.fulfill({ status: 500, body: "" }));
  await page.goto("/#/feel/2026-09-15");
  await expect(page.getByTestId("feel-stays-failed")).toContainText("滞在を読み出せませんでした");
  await expect(page.getByTestId("feel-stays-empty")).toHaveCount(0);
});

// Scenario: 読み出しの失敗と未記入を区別する
test("主観の読み出しが 500 のとき、失敗が出て、暦に「未」も破線も出ない", async ({ page }) => {
  await page.route("**/api/feelings?*", (route) => route.fulfill({ status: 500, body: "" }));
  await page.goto("/#/feel/2026-10-03");
  await expect(page.getByTestId("feel-calendar-failed")).toContainText("暦を読み出せませんでした");
  await expect(page.getByTestId("feel-notice")).toContainText("気分を読み出せませんでした");
  await expect(page.locator('[data-testid^="cal-20"]')).toHaveCount(30);
  for (const c of await page.locator('[data-testid^="cal-20"]').all()) {
    await expect(c.getByTestId("cal-mark")).toHaveText("");
    await expect(c).not.toHaveCSS("border-top-style", "dashed");
  }
});

// ---------------------------------------------------------------- 書く（2100 年以降の日に、前の日から続く滞在などを作る）

interface Placed {
  /** 対象の日 */
  day: string;
}

/** 偽データの `stay_points` と同じ形（1 分ごとに揺らして留まり、そのあと 10 分、北へ歩き去る）。 */
function stayPoints(start: Date, end: Date, north: number, tag: string): unknown[] {
  const lat0 = 35.6812;
  const lon0 = 139.7671;
  const mPerDeg = 111_320;
  const out: unknown[] = [];
  const point = (at: Date, n: number, jitter: number): void => {
    const lat = lat0 + (n + jitter) / mPerDeg;
    const lon = lon0 + jitter / (mPerDeg * Math.cos((lat0 * Math.PI) / 180));
    const body = { lat: Math.round(lat * 1e7) / 1e7, lon: Math.round(lon * 1e7) / 1e7, acc_m: 20 };
    out.push({
      id: crypto.randomUUID(),
      user_id: USER,
      logical_source: "c01-location",
      external_id: null,
      device_id: `e2e-feel-parts-${tag}`,
      origin: "collected",
      event_time: at.toISOString().replace(/\.\d{3}Z$/, "Z"),
      tz_offset_min: 540,
      tz_id: "Asia/Tokyo",
      schema_version: 1,
      raw: JSON.stringify(body),
      payload: body,
    });
  };
  for (let t = start.getTime(); t <= end.getTime(); t += 60_000) point(new Date(t), north, (Math.random() - 0.5) * 20);
  for (let i = 1; i <= 10; i += 1) point(new Date(end.getTime() + i * 60_000), north + 300 * i, 0);
  return out;
}

const jst = (day: string, hm: string): Date => new Date(`${day}T${hm}:00+09:00`);

async function ingest(page: Page, items: unknown[]): Promise<void> {
  for (let i = 0; i < items.length; i += 200) {
    const res = await page.request.post("/api/ingest", { headers: BEARER, data: items.slice(i, i + 200) });
    expect(res.status()).toBe(200);
    const body = (await res.json()) as { accepted: boolean }[];
    expect(body.every((x) => x.accepted)).toBe(true);
  }
}

/** 対象の日に滞在 3 件（10:00 / 13:00 / 16:00）と、前の日 19:00 から朝 6:30 まで続く滞在を置く。 */
async function placeStays(page: Page): Promise<Placed> {
  const day = freshDay();
  const prev = shift(day, -1);
  const tag = day;
  await ingest(page, [
    ...stayPoints(jst(prev, "19:00"), jst(day, "06:30"), 60_000, tag),
    ...stayPoints(jst(day, "10:00"), jst(day, "10:40"), 64_000, tag),
    ...stayPoints(jst(day, "13:00"), jst(day, "13:40"), 66_000, tag),
    ...stayPoints(jst(day, "16:00"), jst(day, "16:40"), 68_000, tag),
  ]);
  return { day };
}

interface FeelingsRead {
  days: { date: string; status: string; stays: { stay: string; records: { scales: { id: string; value: number }[] }[] }[] }[];
}
async function readFeelings(page: Page, from: string, to: string): Promise<FeelingsRead> {
  const res = await page.request.get(`/api/feelings?from=${from}&to=${to}`, { headers: BEARER });
  expect(res.status()).toBe(200);
  return (await res.json()) as FeelingsRead;
}
const stayRecordCount = (v: FeelingsRead): number => v.days.reduce((n, d) => n + d.stays.reduce((m, s) => m + s.records.length, 0), 0);

async function pressStaySaved(page: Page, button: Locator): Promise<void> {
  const [res] = await Promise.all([page.waitForResponse(isIngest), button.click()]);
  expect(res.status()).toBe(200);
}

// Scenario: その日の滞在が始まりの順に並ぶ
// Scenario: 滞在の行を押すとその滞在に保存される
test("滞在 3 件の日は時刻の範囲とともに始まりの順に並び、2 件目の「快」を押すとその滞在の識別子と始まり・終わりで 1 件入る", async ({ page }) => {
  const { day } = await placeStays(page);
  const before = stayRecordCount(await readFeelings(page, shift(day, -1), day));
  await openOn(page, `#/feel/${day}`);
  const rows = page.getByTestId("feel-stay-row");
  await expect(rows).toHaveCount(4); // 前の日から続く 1 件 + 当日の 3 件
  const ranges = await page.getByTestId("feel-stay-range").allTextContents();
  expect(ranges[1]).toBe("10:00 – 10:40");
  expect(ranges[2]).toBe("13:00 – 13:40");
  expect(ranges[3]).toBe("16:00 – 16:40");

  const second = row(page, 2);
  const stayId = (await second.getAttribute("data-stay")) ?? "";
  const requested = page.waitForRequest((r) => r.url().includes("/api/ingest") && r.method() === "POST");
  await pressStaySaved(page, btn(second, "+1 快"));
  const sent = ((await requested).postDataJSON() as { raw: string }[])[0];
  const raw = JSON.parse(sent.raw) as { target: { kind: string; stay: string; start: string; end: string }; scales: { id: string; value: number }[] };
  expect(raw.target.kind).toBe("stay");
  expect(raw.target.stay).toBe(stayId);
  expect(Date.parse(raw.target.start)).toBe(jst(day, "13:00").getTime());
  expect(Date.parse(raw.target.end)).toBe(jst(day, "13:40").getTime());
  expect(raw.scales).toEqual([{ id: "valence", version: 1, value: 1 }]);

  const after = await readFeelings(page, shift(day, -1), day);
  expect(stayRecordCount(after) - before).toBe(1);
  const bucket = after.days.flatMap((d) => d.stays).find((s) => s.stay === stayId);
  expect(bucket?.records[0].scales.find((s) => s.id === "valence")?.value).toBe(1);
  await expect(btn(second, "+1 快")).toHaveAttribute("aria-pressed", "true");
  await expect(btn(row(page, 1), "+1 快")).toHaveAttribute("aria-pressed", "false");
});

// Scenario: 前の日から続く滞在に書いた気分はその行に出る
test("前の日から続く滞在の行の「快」を押して読み直すと、その行の +1 が押された状態で出て、対象の日は記入ありにならない", async ({ page }) => {
  const { day } = await placeStays(page);
  await openOn(page, `#/feel/${day}`);
  const carried = row(page, 0);
  await expect(carried.getByTestId("feel-stay-carried")).toHaveText("前の日から");
  await pressStaySaved(page, btn(carried, "+1 快"));
  const sentTo = (await readFeelings(page, shift(day, -1), day)).days;
  expect(sentTo.find((d) => d.date === shift(day, -1))?.stays.length, "記録は始まりの日（前の日）に数える").toBe(1);

  await page.reload();
  await expect(page.getByTestId("valence-1")).toBeVisible();
  await expect(page.getByTestId("feel-stays-loading")).toHaveCount(0);
  await expect(btn(row(page, 0), "+1 快")).toHaveAttribute("aria-pressed", "true");
  await expect(cal(page, day)).not.toHaveAttribute("data-status", "recorded");
});

// ---------------------------------------------------------------- 通知の時刻

/** 通知の時刻を既定の 21:00 へ戻す（台帳は追記のみなので、戻す行を足す）。 */
async function resetReminder(page: Page): Promise<void> {
  const res = await page.request.post("/api/feelings/reminder", { headers: BEARER, data: { at: "21:00" } });
  expect(res.status()).toBe(200);
}

// Scenario: 画面で通知の時刻を変えられる
// Scenario: 画面で変えた通知の時刻が通知の判断に効く
test("通知の時刻を 22:30 にして保存すると、画面のいまの時刻が 22:30 になり、通知の判断の応答の時刻も 22:30 になる", async ({ page }) => {
  try {
    const day = freshDay();
    await openOn(page, `#/feel/${day}`);
    await expect(page.getByTestId("feel-reminder-current")).toHaveText(/^いまの通知の時刻: \d\d:\d\d$/);
    await page.getByTestId("feel-reminder-at").fill("22:30");
    const [res] = await Promise.all([
      page.waitForResponse((r) => r.url().includes("/api/feelings/reminder") && r.request().method() === "POST"),
      page.getByTestId("feel-reminder-save").click(),
    ]);
    expect(res.status()).toBe(200);
    await expect(page.getByTestId("feel-reminder-current")).toHaveText("いまの通知の時刻: 22:30");
    await expect(page.getByTestId("feel-reminder-ok")).toHaveText("通知の時刻を 22:30 にしました");

    const decision = await page.request.get(`/api/feelings/reminder?date=${day}`, { headers: BEARER });
    expect(decision.status()).toBe(200);
    expect(((await decision.json()) as { at: string }).at).toBe("22:30");
  } finally {
    await resetReminder(page);
  }
});

// Scenario: 通知の時刻を保存できなかったとき入れた値が残る
test("サーバに届かない状態で通知の時刻を 22:30 にして保存すると、保存できなかったことが出て、欄は 22:30 のまま", async ({ page }) => {
  await openOn(page, `#/feel/${freshDay()}`);
  await expect(page.getByTestId("feel-reminder-current")).toHaveText(/^いまの通知の時刻: \d\d:\d\d$/);
  const shown = await page.getByTestId("feel-reminder-current").textContent();
  await page.route("**/api/feelings/reminder", (route) => (route.request().method() === "POST" ? route.abort("failed") : route.continue()));
  await page.getByTestId("feel-reminder-at").fill("22:30");
  await page.getByTestId("feel-reminder-save").click();
  await expect(page.getByTestId("feel-reminder-bad")).toContainText("保存できませんでした");
  await expect(page.getByTestId("feel-reminder-at")).toHaveValue("22:30");
  await expect(page.getByTestId("feel-reminder-current")).toHaveText(shown ?? "");
});
