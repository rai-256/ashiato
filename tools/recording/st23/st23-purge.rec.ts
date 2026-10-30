// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type APIRequestContext, type Locator, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST23「本文ごと消す」を 1 本の流れで撮る:
 * 滞在が並んだ日 → 「この滞在を消す」の確認は今までどおり（本文ごと消すは無い）→ 消すと「消した」の行に「戻す」と「本文ごと消す」
 * → 本文ごと消すの確認（消える件数・消えずに残るもの・戻せない）→ やめると何も消えない → もう一度開いて本文ごと消す
 * → 「本文ごと消した」の行（戻すも本文ごと消すも無い）→ 再読み込みしても同じ → その時間帯に位置が 2 件後から届くと行に件数が出る。
 *
 * **自分の日を作って消す**（`e2e/st23-purge.spec.ts` と同じ。本文ごと消すは戻せないので、共有の偽データの滞在は消さない）。
 * 使っていない過去の日（一覧が記録なしの行だけの日）に位置 13 件を `/ingest` で送って滞在を作る。
 * アサーションは spec の主張の写し（spec を import すると向こうのテストまで登録されるので、helper はここに写す）。
 * 足したのは見るための停止（HOLD）と、画面から「消す」ところ（spec は API で消す）だけ。
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
  start: string;
  end: string;
  id?: string;
}

async function entriesOf(request: APIRequestContext, date: string): Promise<Entry[]> {
  const res = await request.get(`/api/day?date=${date}`, { headers: BEARER });
  expect(res.status(), `GET /day ${date}: ${await res.text()}`).toBe(200);
  return ((await res.json()) as { entries: Entry[] }).entries;
}

/** 2015-01-01 から約 5 年の中の、記録なしの行しか無い日（実行ごとに違う日から探す）。 */
async function takeEmptyDay(request: APIRequestContext): Promise<string> {
  let t = Date.parse("2015-01-01T12:00:00Z") + Math.floor(Math.random() * 1800) * DAY_MS;
  for (;;) {
    const date = new Date(t).toISOString().slice(0, 10);
    if ((await entriesOf(request, date)).every((e) => e.kind === "no-record")) return date;
    t += DAY_MS;
  }
}

function at(date: string, minute: number): string {
  return new Date(Date.parse(`${date}T00:00:00+09:00`) + minute * MINUTE_MS).toISOString();
}

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
    device_id: `rec-st23-${TAG}`,
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

async function openDay(page: Page, date: string): Promise<void> {
  await page.goto(`/#/day/${date}`);
  await page.reload();
  await expect(page.getByTestId("day-view")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
}

/** 時刻を `H:MM` で書く（画面の関数を使わず書き直す。Asia/Tokyo。spec と同じ） */
function clockText(iso: string): string {
  return new Intl.DateTimeFormat("en-GB", { timeZone: "Asia/Tokyo", hour: "numeric", minute: "2-digit", hourCycle: "h23" })
    .format(new Date(iso))
    .replace(/^0(\d):/, "$1:");
}

async function openConfirm(page: Page): Promise<Locator> {
  const row = page.getByTestId("row-erased");
  await expect(row).toHaveCount(1);
  await row.getByRole("button", { name: "本文ごと消す" }).click();
  const dialog = page.getByRole("alertdialog");
  await expect(dialog).toBeVisible();
  return dialog;
}

test("ST23 録画: 消した → 本文ごと消すの確認 → やめる → 本文ごと消す → 後から届いた件数", async ({ page, browser, browserName }) => {
  // 実行環境の記録用（ハーネスの record_summarize.py が results.json から拾う）。アサーションではない
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  // 見るための停止（HOLD）が積もって既定の 30 秒を超える
  test.setTimeout(120_000);
  const request = page.request;

  // 0. 自分の日を作る（画面の外。位置 13 件 → 滞在 1 つ）
  const date = await test.step("0. 準備: 使っていない日に位置 13 件を送って滞在を作る", async () => {
    const d = await takeEmptyDay(request);
    const spot = randomSpot();
    await ingest(request, Array.from({ length: 13 }, (_, i) => locationItem(d, 10 * 60 + i, spot)));
    return d;
  });
  const stays = (await entriesOf(request, date)).filter((e) => e.kind === "stay");
  expect(stays, `${date} の滞在`).toHaveLength(1);
  const range = `${clockText(stays[0].start)} – ${clockText(stays[0].end)}`;

  // 1. 滞在が並んでいる
  const row = page.getByTestId("row-stay");
  await test.step(`1. ${date}: 滞在 ${range} が並んでいる`, async () => {
    await openDay(page, date);
    await expect(row).toHaveCount(1);
    await hold(page);
  });

  // 2. 「この滞在を消す」の確認は変わらない（spec「この滞在を消すの確認は変わらない」）→ 消す
  await test.step("2. 「この滞在を消す」: 確認は位置の件数と「消す」「やめる」だけ → 消す", async () => {
    await row.getByRole("button").first().click();
    await row.getByRole("button", { name: "この滞在を消す" }).click();
    const confirm = row.getByTestId("erase-confirm");
    await expect(confirm).toBeVisible();
    await expect(confirm).toContainText("この滞在と一緒に消える位置の記録 13 件です。消しますか？");
    await expect(confirm.getByRole("button", { name: "本文ごと消す" })).toHaveCount(0);
    await hold(page, 2);
    await confirm.getByRole("button", { name: "消す", exact: true }).click();
  });

  // 3. 「消した」の行に「戻す」と「本文ごと消す」がこの順に並ぶ（spec「消した行に本文ごと消すが出る」）
  const erased = page.getByTestId("row-erased");
  await test.step(`3. 「消した ${range}」の行に「戻す」と「本文ごと消す」`, async () => {
    await expect(erased).toHaveCount(1);
    const restore = erased.getByRole("button", { name: "戻す" });
    const purge = erased.getByRole("button", { name: "本文ごと消す" });
    await expect(restore).toBeVisible();
    await expect(purge).toBeVisible();
    const [r, p] = [await restore.boundingBox(), await purge.boundingBox()];
    expect((r?.x ?? 0) < (p?.x ?? 0), "「戻す」が「本文ごと消す」より前にある").toBe(true);
    await hold(page);
  });

  // 4. 本文ごと消すの確認（spec「確認に消える件数がソースの組ごとに出る」「確認に消えずに残るものが出る」）→ やめる
  await test.step("4. 本文ごと消すの確認: 消えるもの・残るもの・戻せない → 「やめる」", async () => {
    const dialog = await openConfirm(page);
    await expect(dialog.getByTestId("purge-counts")).toContainText("位置 13 件");
    await expect(dialog).toContainText(`${range} の本文を消します。戻せません。`);
    const remains = dialog.getByTestId("purge-remains");
    await expect(remains).toContainText("本人のフォルダの書庫");
    await expect(remains).toContainText("書き出したファイル");
    await expect(remains).toContainText("ディスクの痕跡は次の書き直し（毎晩 3:30）で消えます。");
    await hold(page, 3);
    await dialog.getByRole("button", { name: "やめる" }).click();
    await expect(page.getByRole("alertdialog")).toHaveCount(0);
    await expect(erased).toHaveCount(1);
    await expect(erased.getByRole("button", { name: "戻す" })).toBeVisible();
    await expect(page.getByTestId("row-purged")).toHaveCount(0);
    await hold(page);
  });

  // 5. 確認して本文ごと消す（spec「確認して本文ごと消すと本文ごと消したの行になる」）
  const purged = page.getByTestId("row-purged");
  await test.step(`5. もう一度開いて「本文ごと消す」→「本文ごと消した ${range}」`, async () => {
    const dialog = await openConfirm(page);
    await hold(page);
    await dialog.getByRole("button", { name: "本文ごと消す" }).click();
    await expect(purged).toHaveCount(1);
    await expect(purged).toContainText(`本文ごと消した ${range}`);
    await expect(erased).toHaveCount(0);
    await expect(purged.getByRole("button")).toHaveCount(0);
    await hold(page, 2);
  });

  // 6. 再読み込みしても同じ（spec「本文ごと消した行には戻すが出ない」）
  await test.step("6. 再読み込み後も「本文ごと消した」のまま・「戻す」は無い", async () => {
    await openDay(page, date);
    await expect(purged).toHaveCount(1);
    await expect(purged).toContainText(`本文ごと消した ${range}`);
    await expect(page.getByRole("button", { name: "戻す" })).toHaveCount(0);
    await hold(page);
  });

  // 7. 後から位置が 2 件届く（spec「後から届いて本文を消した件数が行に出る」）
  await test.step("7. その時間帯に位置が 2 件後から届く → 行に「後から 2 件も本文ごと消した」（CH08 Q3 の文言）", async () => {
    await ingest(request, [locationItem(date, 10 * 60 + 3, randomSpot()), locationItem(date, 10 * 60 + 7, randomSpot())]);
    await openDay(page, date);
    await expect(purged).toHaveCount(1);
    await expect(purged).toContainText("後から 2 件も本文ごと消した");
    await hold(page, 2);
  });
});
