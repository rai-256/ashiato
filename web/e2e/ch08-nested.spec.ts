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

async function ingest(request: APIRequestContext, items: unknown[]): Promise<void> {
  const res = await request.post("/api/ingest", { data: items });
  const body = (await res.json()) as { accepted: boolean; error: string | null }[];
  expect(
    body.filter((r) => !r.accepted),
    `取り込みが断られた（status ${res.status()}）`,
  ).toEqual([]);
}



async function erase(request: APIRequestContext, stayId: string): Promise<void> {
  const res = await request.post("/api/stays/erase", { data: { stay_id: stayId } });
  expect(res.ok(), "POST /stays/erase").toBe(true);
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


async function openDay(page: Page, date: string): Promise<void> {
  await page.goto(`/#/day/${date}`);
  await page.reload();
  await expect(page.getByTestId("day-view")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
}




/**
 * 本文ごと消した時間の下の入れ子の行・入れ子の行の口・戻す口の断り・後からの件数・確認の一文（CH08 Task 6 / design D8）を、
 * 本物のブラウザで確かめる。場面は `st23-purge.spec.ts` と同じく、試験の中で自分の日を作り、API で滞在を足し・消し・本文ごと消して作る。
 */

/** 滞在を足して消す（先に消してから重ねる滞在を足す。生きたまま重ねると取り込みの作り直しが吸収する）。 */
async function eraseStay(request: APIRequestContext, date: string, from: number, to: number): Promise<string> {
  const id = await ingestStay(request, date, from, to);
  await erase(request, id);
  return id;
}

const hm = (h: number, m: number): number => h * 60 + m;

/** 親（本文ごと消す）と、その時間にすっぽり入る消した滞在を作る。子は親より先に足して消す。 */
async function makeNested(request: APIRequestContext, children: [number, number][], parent: [number, number] = [hm(10, 0), hm(11, 0)]): Promise<{ date: string; parent: string; children: string[] }> {
  const date = await takeEmptyDay(request);
  const kids: string[] = [];
  for (const [a, b] of children) kids.push(await eraseStay(request, date, a, b));
  const p = await eraseStay(request, date, parent[0], parent[1]);
  await purgeByApi(request, p);
  return { date, parent: p, children: kids };
}

const nestedRows = (page: Page): Locator => page.getByTestId("row-nested");

/** 親の行（`row-purged`）の次の兄弟が `row-nested` であること */
async function nextSiblingIsNested(page: Page, index = 0): Promise<boolean> {
  return page.getByTestId("row-purged").nth(index).evaluate((el) => (el.nextElementSibling as HTMLElement | null)?.dataset.testid === "row-nested");
}

// Scenario: すっぽり入る消した滞在の行には戻すが出ない
test("本文ごと消した時間の下の 10:15 – 10:45 の行に「戻す」は無く、戻せない旨が出る", async ({ page, request }) => {
  const made = await makeNested(request, [[hm(10, 15), hm(10, 45)]]);
  await openDay(page, made.date);

  await expect(page.getByTestId("row-purged")).toContainText("本文ごと消した 10:00 – 11:00");
  const row = nestedRows(page);
  await expect(row).toHaveCount(1);
  await expect(row).toContainText("消した 10:15 – 10:45");
  await expect(row.getByRole("button", { name: "戻す" })).toHaveCount(0);
  await expect(row).toContainText("本文ごと消した時間の中なので戻せません");
});

// Scenario: 戻しても一覧に戻らない消した行の戻すは断られた旨が出る
test("11:00 より後の位置が 2 件ある 10:40 – 11:05 の消した行の「戻す」は、断られた旨が出て行は残る", async ({ page, request }) => {
  const date = await takeEmptyDay(request);
  await ingest(request, [locationItem(date, hm(11, 5), randomSpot()), locationItem(date, hm(11, 10), randomSpot())]);
  await eraseStay(request, date, hm(10, 40), hm(11, 5));
  const p = await eraseStay(request, date, hm(10, 0), hm(11, 0));
  await purgeByApi(request, p);
  await openDay(page, date);

  const row = page.getByTestId("row-erased");
  await expect(row).toHaveCount(1);
  await expect(row).toContainText("消した 11:00 – 11:05");
  await row.getByRole("button", { name: "戻す" }).click();

  await expect(row.getByRole("alert")).toContainText("戻しても一覧に戻らないため、戻せません");
  await expect(row.getByRole("button", { name: "戻す" })).toBeVisible();
  await expect(page.getByTestId("row-erased")).toHaveCount(1);
  await expect(page.getByTestId("row-stay")).toHaveCount(0);
});

// Scenario: 作り直しで隠れて本文を消した滞在も後からの件数に入る
test("本文ごと消した時間帯に位置が 1 件届き、重なる滞在 1 件が作り直しで隠れて本文が消えた日は、後から 2 件も本文ごと消したと出る", async ({ page, request }) => {
  const date = await takeEmptyDay(request);
  const p = await eraseStay(request, date, hm(10, 0), hm(11, 0));
  await purgeByApi(request, p);
  await ingestStay(request, date, hm(10, 30), hm(11, 30));
  // 取り込みの作り直しは位置が届いたときに走る。ここで、一覧に出ていた重なる滞在が隠れて本文が消える
  await ingest(request, [locationItem(date, hm(10, 3), randomSpot())]);
  await openDay(page, date);

  await expect(page.getByTestId("row-purged")).toHaveCount(1);
  await expect(page.getByTestId("purged-late")).toHaveText("後から 2 件も本文ごと消した");
});

// Scenario: すっぽり入る消した滞在の行から本文ごと消せる
test("入れ子の行の「本文ごと消す」で確認すると、その行は「本文ごと消した」として親の下に残り、口は無い", async ({ page, request }) => {
  const made = await makeNested(request, [[hm(10, 15), hm(10, 45)]]);
  await openDay(page, made.date);
  await nestedRows(page).getByRole("button", { name: "本文ごと消す" }).click();
  const dialog = page.getByRole("alertdialog");
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "本文ごと消す" }).click();

  const row = nestedRows(page);
  await expect(row).toHaveCount(1);
  await expect(row).toContainText("本文ごと消した 10:15 – 10:45");
  await expect(page.getByTestId("row-purged")).toHaveCount(1);
  expect(await nextSiblingIsNested(page), "親の行のすぐ下").toBe(true);
  await expect(row.getByRole("button")).toHaveCount(0);
  expect((await preview(request, made.children[0])).purged, "本文が消えている").toBe(true);
});

// Scenario: すっぽり入る消した滞在の行の本文ごと消すは 44 px を下回らない
test("入れ子の行の「本文ごと消す」は幅と高さが 44 CSS px 以上ある", async ({ page, request }) => {
  const made = await makeNested(request, [[hm(10, 15), hm(10, 45)]]);
  await openDay(page, made.date);

  const box = await nestedRows(page).getByRole("button", { name: "本文ごと消す" }).boundingBox();
  expect(box?.width ?? 0, "幅が 44 CSS px 未満").toBeGreaterThanOrEqual(44);
  expect(box?.height ?? 0, "高さが 44 CSS px 未満").toBeGreaterThanOrEqual(44);
});

// Scenario: 確認に一覧に出ている重なる滞在が添えられる
test("重なる一覧に出ている 10:30 – 11:30 の滞在がある「消した 10:00 – 11:00」の確認に、残る旨が出て消える滞在は 1 件", async ({ page, request }) => {
  const date = await takeEmptyDay(request);
  await eraseStay(request, date, hm(10, 0), hm(11, 0));
  await ingestStay(request, date, hm(10, 30), hm(11, 30));
  await openDay(page, date);
  const row = page.getByTestId("row-erased");
  await expect(row).toContainText("消した 10:00 – 11:00");
  await row.getByRole("button", { name: "本文ごと消す" }).click();

  const dialog = page.getByRole("alertdialog");
  await expect(dialog.getByTestId("purge-visible")).toHaveText("一覧に出ている重なる滞在 1 件は残ります。後で作り直しで隠れたら、その本文も消えます");
  await expect(dialog.getByTestId("purge-counts")).toContainText("滞在 1 件");
});

// Scenario: 一覧に出ている重なる滞在が無ければ確認に添えない
test("重なる一覧に出ている滞在の無い消した滞在の確認に、一覧に出ている重なる滞在の文は出ない", async ({ page, request }) => {
  const date = await takeEmptyDay(request);
  await eraseStay(request, date, hm(10, 0), hm(11, 0));
  await openDay(page, date);
  await page.getByTestId("row-erased").getByRole("button", { name: "本文ごと消す" }).click();

  const dialog = page.getByRole("alertdialog");
  await expect(dialog).toBeVisible();
  await expect(dialog).not.toContainText("一覧に出ている重なる滞在");
  await expect(dialog.getByTestId("purge-visible")).toHaveCount(0);
});

// Scenario: すっぽり入る消した時間は本文ごと消した行の下に出る
test("「本文ごと消した 10:00 – 11:00」のすぐ下に、字下げした 10:15 – 10:45 の行がある", async ({ page, request }) => {
  const made = await makeNested(request, [[hm(10, 15), hm(10, 45)]]);
  await openDay(page, made.date);

  const parent = page.getByTestId("row-purged");
  await expect(parent).toContainText("本文ごと消した 10:00 – 11:00");
  expect(await nextSiblingIsNested(page), "親の行の次の兄弟").toBe(true);
  const row = nestedRows(page);
  await expect(row).toContainText("10:15 – 10:45");
  const [pb, nb] = [await parent.boundingBox(), await row.boundingBox()];
  expect(nb?.x ?? 0, "字下げ").toBeGreaterThan(pb?.x ?? 0);
});

// Scenario: すっぽり入る消した時間が 2 つあれば始まりの順に並ぶ
test("10:30 – 10:50 と 10:05 – 10:20 は、10:05 – 10:20 の行、10:30 – 10:50 の行の順に並ぶ", async ({ page, request }) => {
  const made = await makeNested(request, [[hm(10, 30), hm(10, 50)], [hm(10, 5), hm(10, 20)]]);
  await openDay(page, made.date);

  const rows = nestedRows(page);
  await expect(rows).toHaveCount(2);
  await expect(rows.nth(0)).toContainText("10:05 – 10:20");
  await expect(rows.nth(1)).toContainText("10:30 – 10:50");
  expect(await nextSiblingIsNested(page), "親の行のすぐ下").toBe(true);
});

// Scenario: 重なるすっぽり入る消した時間は 1 つの行にまとまる
test("10:10 – 10:30 と 10:20 – 10:40 を別々に消した日は、字下げした行が 10:10 – 10:40 の 1 つだけ", async ({ page, request }) => {
  const made = await makeNested(request, [[hm(10, 10), hm(10, 30)], [hm(10, 20), hm(10, 40)]]);
  await openDay(page, made.date);

  await expect(nestedRows(page)).toHaveCount(1);
  await expect(nestedRows(page)).toContainText("10:10 – 10:40");
});

// Scenario: 2 つの本文ごと消した時間に跨る消した時間は重なりの大きい方の下に出る
test("10:40 – 11:30 の字下げした行は「本文ごと消した 11:00 – 12:00」の下にだけある", async ({ page, request }) => {
  const date = await takeEmptyDay(request);
  await eraseStay(request, date, hm(10, 40), hm(11, 30));
  const first = await eraseStay(request, date, hm(10, 0), hm(11, 0));
  const second = await eraseStay(request, date, hm(11, 0), hm(12, 0));
  await purgeByApi(request, first);
  await purgeByApi(request, second);
  await openDay(page, date);

  const parents = page.getByTestId("row-purged");
  await expect(parents).toHaveCount(2);
  await expect(parents.nth(0)).toContainText("本文ごと消した 10:00 – 11:00");
  await expect(parents.nth(1)).toContainText("本文ごと消した 11:00 – 12:00");
  await expect(nestedRows(page)).toHaveCount(1);
  expect(await nextSiblingIsNested(page, 0), "10:00 – 11:00 の下に入れ子の行がある").toBe(false);
  expect(await nextSiblingIsNested(page, 1), "11:00 – 12:00 の下に入れ子の行がある").toBe(true);
  await expect(nestedRows(page)).toContainText("10:40 – 11:30");
});

// Scenario: 日の境で切ると本文ごと消した時間に入る消した時間も下に出る
test("23:30 – 翌 00:30 を消し翌日 00:00 – 01:00 を本文ごと消した後の翌日は、00:00 – 01:00 の下に 00:00 – 00:30 の行がある", async ({ page, request }) => {
  let date = await takeEmptyDay(request);
  for (;;) {
    const next = new Date(Date.parse(`${date}T12:00:00Z`) + 86_400_000).toISOString().slice(0, 10);
    if ((await entriesOf(request, next)).every((e) => e.kind === "no-record")) {
      nextDay = new Date(Date.parse(`${next}T12:00:00Z`) + 86_400_000).toISOString().slice(0, 10);
      break;
    }
    date = await takeEmptyDay(request);
  }
  const next = new Date(Date.parse(`${date}T12:00:00Z`) + 86_400_000).toISOString().slice(0, 10);
  await eraseStay(request, date, hm(23, 30), hm(24, 30));
  const p = await eraseStay(request, next, hm(0, 0), hm(1, 0));
  await purgeByApi(request, p);
  await openDay(page, next);

  await expect(page.getByTestId("row-purged")).toContainText("本文ごと消した 0:00 – 1:00");
  expect(await nextSiblingIsNested(page), "親の行のすぐ下").toBe(true);
  await expect(nestedRows(page)).toContainText("0:00 – 0:30");
});

// Scenario: すっぽり入る消した滞在を本文ごと消した後も同じ行の下に残る
test("入れ子の消した滞在を本文ごと消した日は、「本文ごと消した」の行が 10:00 – 11:00 の 1 つで、その下に「本文ごと消した 10:15 – 10:45」の行がある", async ({ page, request }) => {
  const made = await makeNested(request, [[hm(10, 15), hm(10, 45)]]);
  await purgeByApi(request, made.children[0]);
  await openDay(page, made.date);

  await expect(page.getByTestId("row-purged")).toHaveCount(1);
  await expect(page.getByTestId("row-purged")).toContainText("本文ごと消した 10:00 – 11:00");
  expect(await nextSiblingIsNested(page), "親の行のすぐ下").toBe(true);
  await expect(nestedRows(page)).toHaveText(/本文ごと消した 10:15 – 10:45/);
});

// Scenario: 本文ごと消したすっぽり入る時間はまとめずに 1 行ずつ出る
test("10:10 – 10:30 と 10:20 – 10:40 を本文ごと消した日は、親の下に「本文ごと消した 10:10 – 10:30」と「本文ごと消した 10:20 – 10:40」の 2 行がある", async ({ page, request }) => {
  const made = await makeNested(request, [[hm(10, 10), hm(10, 30)], [hm(10, 20), hm(10, 40)]]);
  for (const id of made.children) await purgeByApi(request, id);
  await openDay(page, made.date);

  const rows = nestedRows(page);
  await expect(rows).toHaveCount(2);
  await expect(rows.nth(0)).toContainText("本文ごと消した 10:10 – 10:30");
  await expect(rows.nth(1)).toContainText("本文ごと消した 10:20 – 10:40");
  await expect(page.getByTestId("row-purged")).toHaveCount(1);
});
