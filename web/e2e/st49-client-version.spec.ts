// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type APIRequestContext, type Page, type Request } from "@playwright/test";
import pkg from "../package.json" with { type: "json" };

/**
 * 画面が送る記録の 1 件の `client_version`（ST49 / design D20）。本物の画面から気分・場所・個人属性の主張を 1 件ずつ送り、
 * `/api/ingest` の本文を読む。送り先は本物の S-01。
 *
 * 場所は追記のみ（器は消せない）なので、走りごとに違う座標・名前にする（places.spec.ts と同じ考え方）。
 */
declare const process: { env: Record<string, string | undefined> };

test.describe.configure({ timeout: 90_000 });

/** D5 の正規表現 */
const STAMP = /^[0-9][^+]*\+([0-9a-f]{40}|unknown)(\.dirty)?(\.debug)?$/;
const USER = "00000000-0000-0000-0000-000000000000";
const BEARER = { Authorization: `Bearer ${process.env.API_TOKEN ?? "dev-token-0123456789abcdef"}` };
const TAG = Math.random().toString(36).slice(2, 8);
const DAY_MS = 86_400_000;
const JST_MS = 9 * 3_600_000;
const rand = (lo: number, hi: number): number => lo + Math.random() * (hi - lo);
const round7 = (x: number): number => Math.round(x * 1e7) / 1e7;

const isIngestReq = (r: Request): boolean => r.url().includes("/api/ingest") && r.method() === "POST";

/** 押して、`/api/ingest` の本文（要素の配列）を読む。 */
async function ingestBodyOf(page: Page, press: () => Promise<void>): Promise<{ client_version?: unknown }[]> {
  const [req] = await Promise.all([page.waitForRequest(isIngestReq), press()]);
  return JSON.parse(req.postData() ?? "[]") as { client_version?: unknown }[];
}

/** 偽データより前の日に、自分の居た所を 1 か所作る（places.spec.ts の seedVisits と同じ作り）。中心を返す。 */
async function seedVisit(request: APIRequestContext): Promise<{ lat: number; lon: number }> {
  const lat = round7(rand(5, 25));
  const lon = round7(rand(100, 125));
  const events = (await (await request.get("/api/events", { headers: BEARER })).json()) as { logical_source: string; event_time: string }[];
  const times = events.filter((r) => r.logical_source === "c01-location").map((r) => Date.parse(r.event_time));
  const earliest = times.length === 0 ? Date.now() : Math.min(...times);
  const start = Math.floor((earliest + JST_MS) / DAY_MS) * DAY_MS - JST_MS - DAY_MS + 3_600_000;
  const items = Array.from({ length: 13 }, (_, m) => {
    const body = { lat, lon, acc_m: 10 };
    return {
      id: crypto.randomUUID(),
      user_id: USER,
      logical_source: "c01-location",
      external_id: null,
      device_id: `e2e-${TAG}`,
      origin: "collected",
      event_time: new Date(start + m * 60_000).toISOString(),
      tz_offset_min: 540,
      tz_id: "Asia/Tokyo",
      schema_version: 1,
      raw: JSON.stringify(body),
      payload: body,
    };
  });
  const res = await request.post("/api/ingest", { data: items, headers: BEARER });
  expect(res.ok()).toBe(true);
  return { lat, lon };
}

// Scenario: 画面が送る気分・場所・個人属性の主張の 1 件は、画面の組み立ての印を持つ
test("気分・場所・個人属性の主張を 1 件ずつ送ると、どの本文の要素も同じ印の client_version を持ち、版番号は web/package.json のもの", async ({ page, request }) => {
  const version = pkg.version;
  const seen: string[] = [];
  const check = (body: { client_version?: unknown }[]): void => {
    expect(body.length).toBeGreaterThan(0);
    for (const item of body) {
      expect(typeof item.client_version).toBe("string");
      const stamp = item.client_version as string;
      expect(stamp).toMatch(STAMP);
      expect(stamp.startsWith(`${version}+`), `版番号は ${version}`).toBe(true);
      seen.push(stamp);
    }
  };

  // 気分（feel.spec.ts の記入の流れ）。実行ごとに違う遠い日にして偽データの日を避ける
  const day = new Date(Date.UTC(2100, 0, 1) + Math.floor(Math.random() * 3000) * DAY_MS).toISOString().slice(0, 10);
  await page.clock.install({ time: new Date(`${day}T12:00:00+09:00`) });
  await page.goto("/#/feel");
  await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
  check(await ingestBodyOf(page, () => page.getByTestId("valence-1").click()));

  // 場所（places.spec.ts の登録の流れ）
  const c = await seedVisit(request);
  await page.goto("/#/master/places");
  await page.reload();
  await expect(page.getByTestId("places-view")).toBeVisible();
  await expect(page.getByTestId("places-loading")).toBeHidden({ timeout: 15_000 });
  await page.getByRole("button", { name: "場所を足す" }).click();
  await expect(page.getByTestId("place-candidate").first()).toBeVisible();
  const coord = `${c.lat.toFixed(4)}, ${c.lon.toFixed(4)}`;
  const row = page.getByTestId("place-candidate").filter({ hasText: coord });
  if ((await row.count()) === 0) await page.getByRole("button", { name: /^残り \d+ か所$/ }).click();
  await expect(row).toHaveCount(1);
  await row.getByRole("button", { name: "名前を付ける" }).click();
  const form = row.getByTestId("place-add-form");
  await form.getByLabel("名前", { exact: true }).fill(`印-${TAG}`);
  check(await ingestBodyOf(page, () => form.getByRole("button", { name: "登録する" }).click()));

  // 個人属性の主張（MasterView の主張を書く流れ。種類を足してから、その種類に値を積む）
  await page.goto("/#/master");
  await page.reload();
  await expect(page.getByTestId("master-view")).toBeVisible();
  await expect(page.getByTestId("master-loading")).toBeHidden({ timeout: 15_000 });
  await page.getByRole("button", { name: "種類を足す" }).click();
  await page.getByLabel("種類を足す", { exact: true }).fill(`種類-${TAG}`);
  await page.getByRole("button", { name: "決める" }).click();
  const card = page.getByTestId("kind-card").filter({ hasText: `種類-${TAG}` });
  await expect(card).toHaveCount(1);
  await card.getByRole("button", { name: "書く" }).click();
  const write = card.getByTestId("write-form");
  await write.getByLabel("値", { exact: true }).fill("印の確かめ");
  await write.getByRole("radio", { name: "分からない" }).check();
  check(await ingestBodyOf(page, () => write.getByRole("button", { name: "積む" }).click()));

  expect(seen.length).toBeGreaterThanOrEqual(3);
  expect(new Set(seen).size, "3 つとも同じ印").toBe(1);
});
