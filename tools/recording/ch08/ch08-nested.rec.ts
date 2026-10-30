// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type APIRequestContext, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。CH08「本文ごと消した時間と重なる滞在」を 1 本の流れで撮る:
 * 一覧に出ている重なる滞在がある「消した」の行で本文ごと消す → 確認に「一覧に出ている重なる滞在 1 件は残ります…」→ 本文ごと消す
 * → その時間帯に位置が 1 件届き、重なる滞在が作り直しで隠れて本文が消える →「後から 2 件も本文ごと消した」
 * → 別の日: 本文ごと消した時間の下に字下げした消した行（「戻す」は無く、戻せない旨）→ その行から本文ごと消す → 親の下に「本文ごと消した」で残る
 * → 別の日: 一部だけ重なる消した行の「戻す」は、戻しても一覧に戻らないので断られ、行は残る。
 *
 * **自分の日を作って消す**（`web/e2e/ch08-nested.spec.ts` と同じ。本文ごと消すは戻せないので、共有の偽データの滞在は消さない）。
 * 使っていない過去の日に、位置と派生の滞在を `/ingest` で送り、API で消し・本文ごと消して場面を作る（画面の外）。
 * アサーションは spec の主張の写し（spec を import すると向こうのテストまで登録されるので、helper はここに写す）。
 * 足したのは見るための停止（HOLD）と、画面から「本文ごと消す」を押すところ（spec は API で消す場面がある）だけ。
 */
declare const process: { env: Record<string, string | undefined> };

/** 画面の外から `/api` を直に叩く要求は API の合言葉（Bearer）で通す（録画では `tools/record-env.sh` が渡す）。 */
const BEARER = { Authorization: `Bearer ${process.env.API_TOKEN ?? "dev-token-0123456789abcdef"}` };
const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const USER = "00000000-0000-0000-0000-000000000000";
const MINUTE_MS = 60_000;
const DAY_MS = 86_400_000;
const TAG = Math.random().toString(36).slice(2, 8);

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

interface Entry {
  kind: string;
}

async function entriesOf(request: APIRequestContext, date: string): Promise<Entry[]> {
  const res = await request.get(`/api/day?date=${date}`, { headers: BEARER });
  expect(res.status(), `GET /day ${date}: ${await res.text()}`).toBe(200);
  return ((await res.json()) as { entries: Entry[] }).entries;
}

/** 2010-01-01 から約 5 年の中の、記録なしの行しか無い日（実行ごとに違う日から探す。使った日は次から飛ばす）。 */
let cursor = Date.parse("2010-01-01T12:00:00Z") + Math.floor(Math.random() * 1800) * DAY_MS;
async function takeEmptyDay(request: APIRequestContext): Promise<string> {
  for (;;) {
    const date = new Date(cursor).toISOString().slice(0, 10);
    cursor += DAY_MS;
    if ((await entriesOf(request, date)).every((e) => e.kind === "no-record")) return date;
  }
}

function at(date: string, minute: number): string {
  return new Date(Date.parse(`${date}T00:00:00+09:00`) + minute * MINUTE_MS).toISOString();
}

const hm = (h: number, m: number): number => h * 60 + m;

/** 偽データから離れた乱数の座標（spec と同じ） */
function randomSpot(): { lat: number; lon: number } {
  const round7 = (x: number): number => Math.round(x * 1e7) / 1e7;
  return { lat: round7(5 + Math.random() * 20), lon: round7(100 + Math.random() * 25) };
}

function locationItem(date: string, minute: number, spot: { lat: number; lon: number }): Record<string, unknown> {
  const body = { lat: spot.lat, lon: spot.lon, acc_m: 10 };
  return {
    id: crypto.randomUUID(),
    user_id: USER,
    logical_source: "c01-location",
    external_id: null,
    device_id: `rec-ch08-${TAG}`,
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
  const res = await request.post("/api/ingest", { headers: BEARER, data: items });
  const body = (await res.json()) as { accepted: boolean; error: string | null }[];
  expect(body.filter((r) => !r.accepted), `取り込みが断られた（status ${res.status()}）`).toEqual([]);
}

/** 時間を決めた滞在を直に足す（派生の滞在。重なる滞在は位置からは作れないので。spec と同じ）。 */
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

/** 滞在を足して消す（先に消してから重ねる滞在を足す。生きたまま重ねると取り込みの作り直しが吸収する。spec と同じ）。 */
async function eraseStay(request: APIRequestContext, date: string, from: number, to: number): Promise<string> {
  const id = await ingestStay(request, date, from, to);
  const res = await request.post("/api/stays/erase", { headers: BEARER, data: { stay_id: id } });
  expect(res.ok(), "POST /stays/erase").toBe(true);
  return id;
}

/** 画面を通さずに本文ごと消す（確認の口と同じ件数を添える）。 */
async function purgeByApi(request: APIRequestContext, stayId: string): Promise<void> {
  const pre = await request.get(`/api/stays/purge/preview?stay_id=${stayId}`, { headers: BEARER });
  expect(pre.ok(), "GET /stays/purge/preview").toBe(true);
  const { counts } = (await pre.json()) as { counts: unknown };
  const res = await request.post("/api/stays/purge", { headers: BEARER, data: { stay_id: stayId, expected: counts } });
  expect(res.ok(), "POST /stays/purge").toBe(true);
}

async function openDay(page: Page, date: string): Promise<void> {
  await page.goto(`/#/day/${date}`);
  await page.reload();
  await expect(page.getByTestId("day-view")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
}

/** 親の行（`row-purged`）の次の兄弟が `row-nested` であること */
async function nextSiblingIsNested(page: Page): Promise<boolean> {
  return page
    .getByTestId("row-purged")
    .first()
    .evaluate((el) => (el.nextElementSibling as HTMLElement | null)?.dataset.testid === "row-nested");
}

test("CH08 録画: 確認に重なる滞在の一文 → 後から 2 件 → 入れ子の行（戻せない）から本文ごと消す → 断られる「戻す」", async ({ page, browser, browserName }) => {
  // 実行環境の記録用（ハーネスの record_summarize.py が results.json から拾う）。アサーションではない
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  // 見るための停止（HOLD）が積もって既定の 30 秒を超える
  test.setTimeout(180_000);
  const request = page.request;

  // ---- 1 日目: 確認の一文（Q4）と、後からの件数（Q3・Q8）
  const day1 = await test.step("0. 準備: 使っていない日に「消した 10:00 – 11:00」と、重なる一覧に出ている滞在 10:30 – 11:30 を作る", async () => {
    const d = await takeEmptyDay(request);
    await eraseStay(request, d, hm(10, 0), hm(11, 0));
    await ingestStay(request, d, hm(10, 30), hm(11, 30));
    return d;
  });

  const erased = page.getByTestId("row-erased");
  await test.step(`1. ${day1}: 「消した 10:00 – 11:00」の行と、重なる滞在の行が並ぶ`, async () => {
    await openDay(page, day1);
    await expect(erased).toHaveCount(1);
    await expect(erased).toContainText("消した 10:00 – 11:00");
    await expect(page.getByTestId("row-stay")).toHaveCount(1);
    await hold(page);
  });

  // spec「確認に一覧に出ている重なる滞在が添えられる」
  await test.step("2. 「本文ごと消す」の確認に「一覧に出ている重なる滞在 1 件は残ります…」、消える滞在は 1 件 → 本文ごと消す", async () => {
    await erased.getByRole("button", { name: "本文ごと消す" }).click();
    const dialog = page.getByRole("alertdialog");
    await expect(dialog.getByTestId("purge-visible")).toHaveText("一覧に出ている重なる滞在 1 件は残ります。後で作り直しで隠れたら、その本文も消えます");
    await expect(dialog.getByTestId("purge-counts")).toContainText("滞在 1 件");
    await hold(page, 3);
    await dialog.getByRole("button", { name: "本文ごと消す" }).click();
    await expect(page.getByTestId("row-purged")).toHaveCount(1);
    await expect(page.getByTestId("row-purged")).toContainText("本文ごと消した 10:00 – 11:00");
    await hold(page, 2);
  });

  // spec「作り直しで隠れて本文を消した滞在も後からの件数に入る」
  await test.step("3. その時間帯に位置が 1 件届く → 重なる滞在が作り直しで隠れて本文も消え、「後から 2 件も本文ごと消した」", async () => {
    await ingest(request, [locationItem(day1, hm(10, 3), randomSpot())]);
    await openDay(page, day1);
    await expect(page.getByTestId("row-purged")).toHaveCount(1);
    await expect(page.getByTestId("purged-late")).toHaveText("後から 2 件も本文ごと消した");
    await hold(page, 2);
  });

  // ---- 2 日目: 本文ごと消した時間の下の入れ子の行（Q7・Q2・C5）
  const day2 = await test.step("4. 準備: 別の日に 10:15 – 10:45 を消してから、10:00 – 11:00 を消して本文ごと消す", async () => {
    const d = await takeEmptyDay(request);
    await eraseStay(request, d, hm(10, 15), hm(10, 45));
    const parent = await eraseStay(request, d, hm(10, 0), hm(11, 0));
    await purgeByApi(request, parent);
    return d;
  });

  const nested = page.getByTestId("row-nested");
  // spec「すっぽり入る消した時間は本文ごと消した行の下に出る」「すっぽり入る消した滞在の行には戻すが出ない」
  await test.step(`5. ${day2}: 「本文ごと消した 10:00 – 11:00」のすぐ下に字下げした 10:15 – 10:45。「戻す」は無く、戻せない旨`, async () => {
    await openDay(page, day2);
    const parent = page.getByTestId("row-purged");
    await expect(parent).toContainText("本文ごと消した 10:00 – 11:00");
    await expect(nested).toHaveCount(1);
    await expect(nested).toContainText("消した 10:15 – 10:45");
    expect(await nextSiblingIsNested(page), "親の行のすぐ下").toBe(true);
    const [pb, nb] = [await parent.boundingBox(), await nested.boundingBox()];
    expect(nb?.x ?? 0, "字下げ").toBeGreaterThan(pb?.x ?? 0);
    await expect(nested.getByRole("button", { name: "戻す" })).toHaveCount(0);
    await expect(nested).toContainText("本文ごと消した時間の中なので戻せません");
    await hold(page, 2);
  });

  // spec「すっぽり入る消した滞在の行から本文ごと消せる」
  await test.step("6. 入れ子の行の「本文ごと消す」→ 確認 → 親の下に「本文ごと消した 10:15 – 10:45」で残り、口は無い", async () => {
    await nested.getByRole("button", { name: "本文ごと消す" }).click();
    const dialog = page.getByRole("alertdialog");
    await expect(dialog).toBeVisible();
    await hold(page, 2);
    await dialog.getByRole("button", { name: "本文ごと消す" }).click();
    await expect(nested).toHaveCount(1);
    await expect(nested).toContainText("本文ごと消した 10:15 – 10:45");
    await expect(page.getByTestId("row-purged")).toHaveCount(1);
    expect(await nextSiblingIsNested(page), "親の行のすぐ下").toBe(true);
    await expect(nested.getByRole("button")).toHaveCount(0);
    await hold(page);
  });

  // spec「すっぽり入る消した滞在を本文ごと消した後も同じ行の下に残る」
  await test.step("7. 再読み込みしても同じ（親の行は 1 つ、その下に「本文ごと消した 10:15 – 10:45」）", async () => {
    await openDay(page, day2);
    await expect(page.getByTestId("row-purged")).toHaveCount(1);
    expect(await nextSiblingIsNested(page), "親の行のすぐ下").toBe(true);
    await expect(nested).toHaveText(/本文ごと消した 10:15 – 10:45/);
    await hold(page);
  });

  // ---- 3 日目: 一部だけ重なる消した行の「戻す」の断り（Q2・D4）
  const day3 = await test.step("8. 準備: 別の日に 11:00 より後の位置 2 件と、10:40 – 11:05 を消し、10:00 – 11:00 を本文ごと消す", async () => {
    const d = await takeEmptyDay(request);
    await ingest(request, [locationItem(d, hm(11, 5), randomSpot()), locationItem(d, hm(11, 10), randomSpot())]);
    await eraseStay(request, d, hm(10, 40), hm(11, 5));
    const p = await eraseStay(request, d, hm(10, 0), hm(11, 0));
    await purgeByApi(request, p);
    return d;
  });

  // spec「戻しても一覧に戻らない消した行の戻すは断られた旨が出る」
  await test.step(`9. ${day3}: 「消した 11:00 – 11:05」の「戻す」→「戻しても一覧に戻らないため、戻せません」、行は残る`, async () => {
    await openDay(page, day3);
    await expect(erased).toHaveCount(1);
    await expect(erased).toContainText("消した 11:00 – 11:05");
    await hold(page);
    await erased.getByRole("button", { name: "戻す" }).click();
    await expect(erased.getByRole("alert")).toContainText("戻しても一覧に戻らないため、戻せません");
    await expect(erased.getByRole("button", { name: "戻す" })).toBeVisible();
    await expect(page.getByTestId("row-erased")).toHaveCount(1);
    await expect(page.getByTestId("row-stay")).toHaveCount(0);
    await hold(page, 2);
  });
});
