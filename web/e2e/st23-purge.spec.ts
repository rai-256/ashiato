// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test as base, type APIRequestContext, type Locator, type Page } from "@playwright/test";

declare const process: { env: Record<string, string | undefined> };

/**
 * 試験の中で `/api` を直に叩く `request` は、API の合言葉（Bearer）で通す（`places.spec.ts` と同じ。既定の `storageState` は
 * 先に走る login の e2e のログアウトで失効していることがあり、それに依らない）。画面（`page`）には何も付けない。
 */
const test = base.extend({
  request: async ({ playwright, baseURL }, use) => {
    const ctx = await playwright.request.newContext({
      baseURL,
      storageState: { cookies: [], origins: [] },
      extraHTTPHeaders: { Authorization: `Bearer ${process.env.API_TOKEN ?? "dev-token-0123456789abcdef"}` },
    });
    await use(ctx);
    await ctx.dispose();
  },
});

/**
 * 「消した」の行から本文ごと消す（ST23 Task 9 / design D12・D16）を、本物のブラウザで確かめる。
 *
 * **消す Scenario は、試験の中で自分の日を作って消す**（spec review R16）。共有の偽データの滞在は読むだけで消さない。
 * 実行ごとに、まだ使っていない日付（2020-01-01 から 1 日ずつ。`GET /day` が空の日を探す）に位置を `/ingest` で送って滞在を作り、
 * `POST /stays/erase` で「消した」にしてから画面を開く。使い捨ての日なので `STACK_RESET` なしで何度走らせても同じ結果になる。
 * （brief は 2031 年からとしていたが、まだ来ていない日は一覧が消した区間を並べないので、過去の日にした。）
 * 書庫の写し 2 個は `tools/seed.d/202610250400_st23-purge.sh` が置く。
 */

const USER = "00000000-0000-0000-0000-000000000000";
const FIRST_DAY = "2020-01-01";
const MINUTE_MS = 60_000;
/** この走りの印。書庫の項目の原文に入れ、走りごとに内容の鍵が違うようにする */
const TAG = Math.random().toString(36).slice(2, 8);

// ------------------------------------------------------------------ サーバへ（画面と同じ `/api` の経路）

interface Entry {
  kind: string;
  start: string;
  end: string;
  id?: string;
  stay_ids?: string[];
  purged?: boolean;
  purge_stay_id?: string;
  late_purged?: number;
}

async function entriesOf(request: APIRequestContext, date: string): Promise<Entry[]> {
  const res = await request.get(`/api/day?date=${date}`);
  expect(res.status(), `GET /day ${date}: ${await res.text()}`).toBe(200);
  return ((await res.json()) as { entries: Entry[] }).entries;
}

/** 使っていない日付を 1 つ返す（その日に記録なしの行しか無い日）。使い終えた日は次から飛ばす。 */
let nextDay = FIRST_DAY;
async function takeEmptyDay(request: APIRequestContext): Promise<string> {
  for (;;) {
    const date = nextDay;
    nextDay = new Date(Date.parse(`${date}T12:00:00Z`) + 86_400_000).toISOString().slice(0, 10);
    const entries = await entriesOf(request, date);
    if (entries.every((e) => e.kind === "no-record")) return date;
  }
}

function at(date: string, minute: number): string {
  return new Date(Date.parse(`${date}T00:00:00+09:00`) + minute * MINUTE_MS).toISOString();
}

interface Spot {
  lat: number;
  lon: number;
}

/** 偽データから離れた乱数の座標（違う日どうしで居た所が混ざらないようにする） */
function randomSpot(): Spot {
  const round7 = (x: number): number => Math.round(x * 1e7) / 1e7;
  return { lat: round7(5 + Math.random() * 20), lon: round7(100 + Math.random() * 25) };
}

function locationItem(date: string, minute: number, spot: Spot): Record<string, unknown> {
  const body = { lat: spot.lat, lon: spot.lon, acc_m: 10 };
  return {
    id: crypto.randomUUID(),
    user_id: USER,
    logical_source: "c01-location",
    external_id: null,
    device_id: `e2e-st23-${TAG}`,
    origin: "collected",
    event_time: at(date, minute),
    tz_offset_min: 540,
    tz_id: "Asia/Tokyo",
    schema_version: 1,
    raw: JSON.stringify(body),
    payload: body,
  };
}

/** 書庫の項目（`/ingest` で送る。`tools/seed.sh` の書庫の項目の送り方と同じ形） */
function archiveItem(source: string, date: string, minute: number, raw: Record<string, unknown>): Record<string, unknown> {
  return {
    id: crypto.randomUUID(),
    user_id: USER,
    logical_source: source,
    external_id: null,
    device_id: "s01-c03",
    origin: "collected",
    event_time: at(date, minute),
    tz_offset_min: 540,
    tz_id: "Asia/Tokyo",
    schema_version: 1,
    raw: JSON.stringify(raw),
    payload: raw,
  };
}

async function ingest(request: APIRequestContext, items: unknown[]): Promise<void> {
  const res = await request.post("/api/ingest", { data: items });
  const body = (await res.json()) as { accepted: boolean; error: string | null }[];
  expect(
    body.filter((r) => !r.accepted),
    `取り込みが断られた（status ${res.status()}）`,
  ).toEqual([]);
}

/** 位置を `from` 分から `count` 件、1 分ごとに送る（1 回の取り込み）。 */
async function sendLocations(request: APIRequestContext, date: string, spot: Spot, from: number, count: number): Promise<void> {
  await ingest(
    request,
    Array.from({ length: count }, (_, i) => locationItem(date, from + i, spot)),
  );
}

interface Made {
  date: string;
  /** 本文ごと消す前の「消した」の行の時間 */
  start: string;
  end: string;
  stayId: string;
}

async function onlyStay(request: APIRequestContext, date: string): Promise<Entry> {
  const stays = (await entriesOf(request, date)).filter((e) => e.kind === "stay");
  expect(stays, `${date} の滞在`).toHaveLength(1);
  return stays[0];
}

async function erase(request: APIRequestContext, stayId: string): Promise<void> {
  const res = await request.post("/api/stays/erase", { data: { stay_id: stayId } });
  expect(res.ok(), "POST /stays/erase").toBe(true);
}

/** 自分の日に 13 分の滞在を作って「消した」にする（位置 13 件が一緒に消える）。 */
async function makeErasedDay(request: APIRequestContext): Promise<Made> {
  const date = await takeEmptyDay(request);
  await sendLocations(request, date, randomSpot(), 10 * 60, 13);
  const stay = await onlyStay(request, date);
  await erase(request, stay.id ?? "");
  return { date, start: stay.start, end: stay.end, stayId: stay.id ?? "" };
}

/**
 * 確認に出る件数を決めた日（位置 70 件・書庫の位置 3 件・位置を持つ書庫の項目 1 件・前の版 20 件）を作って「消した」にする。
 * 前の版は、終わりが延びる位置を 21 回に分けて送り、送るたびの作り直しで積む（最初の 11 件で滞在ができ、残る 59 件を 20 回に分ける）。
 */
async function makeBigErasedDay(request: APIRequestContext): Promise<Made> {
  const date = await takeEmptyDay(request);
  const spot = randomSpot();
  const base = 10 * 60;
  await sendLocations(request, date, spot, base, 11);
  let sent = 11;
  for (let step = 0; step < 20; step += 1) {
    const count = step < 19 ? 3 : 2;
    await sendLocations(request, date, spot, base + sent, count);
    sent += count;
  }
  expect(sent, "送った位置").toBe(70);
  const stay = await onlyStay(request, date);
  await ingest(request, [
    ...[20, 30, 40].map((minute) => archiveItem("c03-legacy-location", date, base + minute, { lat: spot.lat, lon: spot.lon, tag: `${TAG}-${date}-${minute}` })),
    archiveItem("c03-youtube-watch", date, base + 50, { title: "視聴", locationInfos: [{ name: "東京" }], tag: `${TAG}-${date}` }),
  ]);
  await erase(request, stay.id ?? "");
  return { date, start: stay.start, end: stay.end, stayId: stay.id ?? "" };
}

/** 時間を決めた滞在を直に足す（派生の滞在。位置は送らない。隣り合う・重なる滞在は位置からは作れないので）。 */
async function ingestStay(request: APIRequestContext, date: string, fromMinute: number, toMinute: number): Promise<string> {
  const id = crypto.randomUUID();
  const body = { start: at(date, fromMinute), end: at(date, toMinute), lat: 35.68, lon: 139.76 };
  await ingest(request, [
    {
      id,
      user_id: USER,
      logical_source: "s01-stay",
      external_id: id,
      device_id: null,
      origin: "derived",
      event_time: at(date, fromMinute),
      tz_offset_min: 540,
      tz_id: "Asia/Tokyo",
      schema_version: 1,
      raw: JSON.stringify(body),
      payload: body,
    },
  ]);
  return id;
}

interface Preview {
  counts: { stays: number; locations: number; archive_locations: number; archive_items: number; versions: number };
  copies: number;
  purged: boolean;
}

async function preview(request: APIRequestContext, stayId: string): Promise<Preview> {
  const res = await request.get(`/api/stays/purge/preview?stay_id=${stayId}`);
  expect(res.ok(), "GET /stays/purge/preview").toBe(true);
  return (await res.json()) as Preview;
}

/** 画面を通さずに本文ごと消す（確認の口と同じ件数を添える）。 */
async function purgeByApi(request: APIRequestContext, stayId: string): Promise<void> {
  const { counts } = await preview(request, stayId);
  const res = await request.post("/api/stays/purge", { data: { stay_id: stayId, expected: counts } });
  expect(res.ok(), "POST /stays/purge").toBe(true);
}

// ------------------------------------------------------------------ 画面

async function openDay(page: Page, date: string): Promise<void> {
  await page.goto(`/#/day/${date}`);
  await page.reload();
  await expect(page.getByTestId("day-view")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
}

/** 時刻を `H:MM` で書く（画面の関数を使わず書き直す。Asia/Tokyo） */
function clockText(iso: string): string {
  return new Intl.DateTimeFormat("en-GB", { timeZone: "Asia/Tokyo", hour: "numeric", minute: "2-digit", hourCycle: "h23" })
    .format(new Date(iso))
    .replace(/^0(\d):/, "$1:");
}

const rangeText = (start: string, end: string): string => `${clockText(start)} – ${clockText(end)}`;

async function openConfirm(page: Page, made: Made): Promise<Locator> {
  await openDay(page, made.date);
  const row = page.getByTestId("row-erased");
  await expect(row).toHaveCount(1);
  await row.getByRole("button", { name: "本文ごと消す" }).click();
  const dialog = page.getByRole("alertdialog");
  await expect(dialog).toBeVisible();
  return dialog;
}

// Scenario: 消した行に本文ごと消すが出る
test("「消した」の行に「戻す」と「本文ごと消す」がこの順に並ぶ", async ({ page, request }) => {
  const made = await makeErasedDay(request);
  await openDay(page, made.date);

  const row = page.getByTestId("row-erased");
  await expect(row).toHaveCount(1);
  const restore = row.getByRole("button", { name: "戻す" });
  const purge = row.getByRole("button", { name: "本文ごと消す" });
  await expect(restore).toBeVisible();
  await expect(purge).toBeVisible();
  const [r, p] = [await restore.boundingBox(), await purge.boundingBox()];
  expect((r?.x ?? 0) < (p?.x ?? 0), "「戻す」が「本文ごと消す」より前にある").toBe(true);
});

// Scenario: 本文ごと消すの操作は 44 px を下回らない
test("「本文ごと消す」は幅と高さが 44 CSS px 以上ある", async ({ page, request }) => {
  const made = await makeErasedDay(request);
  await openDay(page, made.date);

  const box = await page.getByTestId("row-erased").getByRole("button", { name: "本文ごと消す" }).boundingBox();
  expect(box?.width ?? 0, "本文ごと消すの幅が 44 CSS px 未満").toBeGreaterThanOrEqual(44);
  expect(box?.height ?? 0, "本文ごと消すの高さが 44 CSS px 未満").toBeGreaterThanOrEqual(44);
});

test.describe("確認の中身（読むだけ。1 つの日を使う）", () => {
  test.describe.configure({ mode: "serial" });
  let made: Made;
  test.beforeAll(async ({ request }) => {
    made = await makeBigErasedDay(request);
  });

  // Scenario: 確認に消える件数がソースの組ごとに出る
  test("確認に滞在・位置・書庫の位置・書庫の項目・前の版の件数が出る", async ({ page }) => {
    const dialog = await openConfirm(page, made);

    await expect(dialog.getByTestId("purge-counts")).toHaveText("消えるもの: 滞在 1 件・位置 70 件・書庫の位置 3 件・書庫の項目 1 件（前の版 20 件）。");
    await expect(dialog).toContainText(`${rangeText(made.start, made.end)} の本文を消します。戻せません。`);
  });

  // Scenario: 確認に消えずに残るものが出る
  test("確認に書庫の写し・本人のフォルダの書庫・書き出したファイルと、痕跡が毎晩 3:30 に消えることが出る", async ({ page, request }) => {
    const { copies } = await preview(request, made.stayId);
    expect(copies, "seed.d が置いた書庫の写し").toBeGreaterThanOrEqual(2);
    const dialog = await openConfirm(page, made);

    const remains = dialog.getByTestId("purge-remains");
    await expect(remains).toContainText(`書庫の写し ${copies} 個`);
    await expect(remains).toContainText("本人のフォルダの書庫");
    await expect(remains).toContainText("書き出したファイル");
    await expect(remains).toContainText("ディスクの痕跡は次の書き直し（毎晩 3:30）で消えます。");
  });

  // Scenario: 本文ごと消す確認をやめると何も消えない
  test("確認で「やめる」を選ぶと「消した」の行と「戻す」が残り、本文は消えていない", async ({ page, request }) => {
    const before = await preview(request, made.stayId);
    const dialog = await openConfirm(page, made);
    await dialog.getByRole("button", { name: "やめる" }).click();

    await expect(page.getByRole("alertdialog")).toHaveCount(0);
    const row = page.getByTestId("row-erased");
    await expect(row).toHaveCount(1);
    await expect(row.getByRole("button", { name: "戻す" })).toBeVisible();
    await expect(page.getByTestId("row-purged")).toHaveCount(0);
    const after = await preview(request, made.stayId);
    expect(after.purged, "本文を消してしまった").toBe(false);
    expect(after.counts).toEqual(before.counts);
    expect(after.counts.locations, "その時間の位置の本文が残っている").toBe(70);
  });
});

// Scenario: 確認して本文ごと消すと本文ごと消したの行になる
test("確認して「本文ごと消す」と、その時間が「本文ごと消した」の行になり、戻すも本文ごと消すも無い", async ({ page, request }) => {
  const made = await makeErasedDay(request);
  const dialog = await openConfirm(page, made);
  await dialog.getByRole("button", { name: "本文ごと消す" }).click();

  const purged = page.getByTestId("row-purged");
  await expect(purged).toHaveCount(1);
  await expect(purged).toContainText(`本文ごと消した ${rangeText(made.start, made.end)}`);
  await expect(page.getByTestId("row-erased")).toHaveCount(0);
  await expect(purged.getByRole("button", { name: "戻す" })).toHaveCount(0);
  await expect(purged.getByRole("button", { name: "本文ごと消す" })).toHaveCount(0);
  expect((await preview(request, made.stayId)).purged, "本文が消えている").toBe(true);
});

// Scenario: 件数が変わって断られると数え直した確認が出る
test("確認を開いた後に位置が 1 件届いてから確認すると、何も消えず 1 件多い件数で出し直される", async ({ page, request }) => {
  const made = await makeErasedDay(request);
  const dialog = await openConfirm(page, made);
  await expect(dialog.getByTestId("purge-counts")).toContainText("位置 13 件");

  // 確認を開いた後に、その時間帯へ位置が 1 件届く
  await ingest(request, [locationItem(made.date, 10 * 60 + 5, randomSpot())]);
  await dialog.getByRole("button", { name: "本文ごと消す" }).click();

  await expect(page.getByTestId("purge-changed")).toBeVisible();
  await expect(dialog.getByTestId("purge-counts")).toContainText("位置 14 件");
  await expect(page.getByTestId("row-purged")).toHaveCount(0);
  const after = await preview(request, made.stayId);
  expect(after.purged, "断られたのに本文が消えた").toBe(false);
  expect(after.counts.locations).toBe(14);
});

// Scenario: 後から届いて本文を消した件数が行に出る
test("本文ごと消した時間帯に位置が 2 件届くと、行に後から届いた 2 件も本文ごと消したと出る", async ({ page, request }) => {
  const made = await makeErasedDay(request);
  await purgeByApi(request, made.stayId);
  await ingest(request, [locationItem(made.date, 10 * 60 + 3, randomSpot()), locationItem(made.date, 10 * 60 + 7, randomSpot())]);
  await openDay(page, made.date);

  const purged = page.getByTestId("row-purged");
  await expect(purged).toHaveCount(1);
  await expect(purged).toContainText("後から届いた 2 件も本文ごと消した");
});

// Scenario: この滞在を消すの確認は変わらない
test("滞在の詳細の「この滞在を消す」の確認は一緒に消える位置の件数だけを出し、「消す」と「やめる」を置く", async ({ page, request }) => {
  const date = await takeEmptyDay(request);
  await sendLocations(request, date, randomSpot(), 10 * 60, 13);
  await openDay(page, date);
  const row = page.getByTestId("row-stay");
  await expect(row).toHaveCount(1);
  await row.getByRole("button").first().click();
  await row.getByRole("button", { name: "この滞在を消す" }).click();

  const confirm = row.getByTestId("erase-confirm");
  await expect(confirm).toBeVisible();
  await expect(confirm).toContainText("この滞在と一緒に消える位置の記録 13 件です。消しますか？");
  await expect(confirm.getByRole("button", { name: "消す", exact: true })).toBeVisible();
  await expect(confirm.getByRole("button", { name: "やめる" })).toBeVisible();
  await expect(confirm.getByRole("button", { name: "本文ごと消す" })).toHaveCount(0);
  await confirm.getByRole("button", { name: "やめる" }).click();
  await expect(page.getByTestId("row-stay")).toHaveCount(1);
});

// Scenario: 本文ごと消した行には戻すが出ない
test("本文ごと消した日の一覧で、「本文ごと消した」の行に「戻す」は無い", async ({ page, request }) => {
  const made = await makeErasedDay(request);
  await purgeByApi(request, made.stayId);
  await openDay(page, made.date);

  const purged = page.getByTestId("row-purged");
  await expect(purged).toHaveCount(1);
  await expect(page.getByRole("button", { name: "戻す" })).toHaveCount(0);
});

// Scenario: 本文ごと消した時間は本文ごと消したと書かれた行で出る
test("本文ごと消した時間は「本文ごと消した」と時間を持つ行で出て、滞在の詳細を開く操作は無い", async ({ page, request }) => {
  const made = await makeErasedDay(request);
  await purgeByApi(request, made.stayId);
  await openDay(page, made.date);

  const purged = page.getByTestId("row-purged");
  await expect(purged).toHaveCount(1);
  await expect(purged).toContainText(`本文ごと消した ${rangeText(made.start, made.end)}`);
  await expect(purged.getByRole("button")).toHaveCount(0);
  await expect(purged.locator("[aria-expanded]")).toHaveCount(0);
});

// Scenario: 本文ごと消した時間と消した時間は隣り合ってもまとまらない
test("10:00 – 11:00 を本文ごと消し 11:00 – 11:30 を消した日は、二つの行が別々に出て「戻す」は消した行の 1 つだけ", async ({ page, request }) => {
  const date = await takeEmptyDay(request);
  // 先の滞在を消してから次の滞在を足す（生きたまま重ねると、取り込みの作り直しが吸収してしまう）
  const purged = await ingestStay(request, date, 10 * 60, 11 * 60);
  await erase(request, purged);
  const erased = await ingestStay(request, date, 11 * 60, 11 * 60 + 30);
  await erase(request, erased);
  await purgeByApi(request, purged);
  await openDay(page, date);

  await expect(page.getByTestId("row-purged")).toHaveCount(1);
  await expect(page.getByTestId("row-purged")).toContainText("本文ごと消した 10:00 – 11:00");
  await expect(page.getByTestId("row-erased")).toHaveCount(1);
  await expect(page.getByTestId("row-erased")).toContainText("消した 11:00 – 11:30");
  await expect(page.getByRole("button", { name: "戻す" })).toHaveCount(1);
  await expect(page.getByTestId("row-erased").getByRole("button", { name: "戻す" })).toHaveCount(1);
});

// Scenario: 本文ごと消した時間と重なる消した時間は差し引いて出る
test("10:00 – 11:00 と 10:30 – 11:30 を別々に消し前者だけ本文ごと消した日は、「本文ごと消した 10:00 – 11:00」と「消した 11:00 – 11:30」が出る", async ({ page, request }) => {
  const date = await takeEmptyDay(request);
  const purged = await ingestStay(request, date, 10 * 60, 11 * 60);
  await erase(request, purged);
  const overlapping = await ingestStay(request, date, 10 * 60 + 30, 11 * 60 + 30);
  await erase(request, overlapping);
  await purgeByApi(request, purged);
  await openDay(page, date);

  await expect(page.getByTestId("row-purged")).toHaveCount(1);
  await expect(page.getByTestId("row-purged")).toContainText("本文ごと消した 10:00 – 11:00");
  await expect(page.getByTestId("row-erased")).toHaveCount(1);
  await expect(page.getByTestId("row-erased")).toContainText("消した 11:00 – 11:30");
});
