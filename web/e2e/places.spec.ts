// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test as base, type APIRequestContext, type Locator, type Page, type Request, type Route } from "@playwright/test";
import type { Candidate, Place, PreviousCoord } from "../src/places";

declare const process: { env: Record<string, string | undefined> };

/**
 * 試験の中で `/api` を直に叩く `request` は、API の合言葉（Bearer）で通す。既定の `storageState`（ログインの印）は、
 * 先に走る login の e2e のログアウトで失効していることがあり、それに依らない（ST28 の認証との合わせ。`tools/stack.sh` と同じ既定値）。
 * 画面（`page`）には何も付けない。
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
 * 場所の画面の e2e（ST21 / tasks 10.1〜10.4）。**画面の Scenario を「人間の確認待ち」へ逃がさない。**
 *
 * アサートするのは**数値と経路**（実寸・可視・URL・送った求めの本文と宛先・文字から計算した比）。スクリーンショット比較は使わない。
 *
 * **場所の器は消せない**（追記のみ）ので、手元で何度走らせても通る形にする:
 *   - 走りごとに違う座標（偽データの居た所 35.68, 139.76 と重ならない範囲の乱数）に、**画面と同じ経路（`/api/ingest`）で位置の記録を送って
 *     自分の居た所を作る**（10 分以上・100 m 以内。滞在の作り直しは取り込みの後に走る）
 *   - 場所の名前は走りごとに違う印を付け、数は**同じ試験の中で読んだ `/api/places` と `/api/places/candidates` の値と突き合わせる**
 *     （前の走りの場所が残っていても数が合う）
 *   - 場所が 0 件・居た所が 0 件・登録 6 の量・読み出しの失敗・届かない・`place_id_taken` は `page.route` で応答を差し替えて作る
 *   - 「届いたが応答が返らなかった」は **`route.fetch()` でサーバへ通してから応答を捨てて**作る（`route.abort()` は届かない側）
 */
test.describe.configure({ timeout: 90_000 });

const USER = "00000000-0000-0000-0000-000000000000";
const TODAY = "2026-10-02";
/** この走りの印。名前を走りごとに違うものにして、前の走りの場所と混ざらないようにする */
const TAG = Math.random().toString(36).slice(2, 8);

// ------------------------------------------------------------------ 数・文字の書き方（画面と突き合わせる側は、画面の関数を使わず書き直す）

const coordText = (lat: number, lon: number): string => `${lat.toFixed(4)}, ${lon.toFixed(4)}`;
const durationText = (minutes: number): string => (minutes >= 60 ? `${Math.floor(minutes / 60)} 時間` : `${minutes} 分`);
const rand = (lo: number, hi: number): number => lo + Math.random() * (hi - lo);
const round7 = (x: number): number => Math.round(x * 1e7) / 1e7;

// ------------------------------------------------------------------ サーバへ（画面と同じ `/api` の経路）

interface Spot {
  lat: number;
  lon: number;
}

async function ingest(request: APIRequestContext, items: unknown[]): Promise<void> {
  const res = await request.post("/api/ingest", { data: items });
  const body = (await res.json()) as { accepted: boolean; error: string | null }[];
  expect(
    body.filter((r) => !r.accepted),
    `取り込みが断られた（status ${res.status()}）`,
  ).toEqual([]);
}

async function apiPlaces(request: APIRequestContext): Promise<Place[]> {
  const res = await request.get("/api/places");
  expect(res.ok()).toBe(true);
  return ((await res.json()) as { places: Place[] }).places;
}

async function apiCandidates(request: APIRequestContext): Promise<Candidate[]> {
  const res = await request.get("/api/places/candidates");
  expect(res.ok()).toBe(true);
  return ((await res.json()) as { candidates: Candidate[] }).candidates;
}

const DAY_MS = 86_400_000;
const JST_MS = 9 * 3_600_000;

/**
 * 位置の記録の置き場所（時刻）。**いまある位置の記録のどれとも時間が重ならない日**を選ぶ。
 *
 * 滞在は利用者の位置を時刻順に 1 本で見て作るので、違う場所の記録が同じ時間に混ざると 100 m 以内にまとまらず滞在にならない
 * （実測: 試験どうしで時間を重ねたら、後から送った分の居た所ができなかった）。そこで、いまある位置の記録のうちいちばん古い日の
 * **前の日（JST）の 01:00 から**並べる。1 回の呼び出しが 1 日を使い、日をまたがないので、滞在が日の境で割れない。
 */
async function freeDayStart(request: APIRequestContext): Promise<number> {
  const res = await request.get("/api/events");
  const rows = (await res.json()) as { logical_source: string; event_time: string }[];
  const times = rows.filter((r) => r.logical_source === "c01-location").map((r) => Date.parse(r.event_time));
  const earliest = times.length === 0 ? Date.now() : Math.min(...times);
  const earliestDay = Math.floor((earliest + JST_MS) / DAY_MS) * DAY_MS - JST_MS;
  return earliestDay - DAY_MS + 3_600_000;
}

/**
 * **自分の居た所を n か所作る。** 偽データから離れた乱数の座標に、1 分ごとの位置を 13 分ずつ（10 分以上・100 m 以内）送る。
 * 居た所どうしは 2 km 離す（まとまらない）。返すのは、`/api/places/candidates` が実際に返した中心
 * （画面が送る座標と突き合わせる値）。**名前の無い居た所の並びは新しい順なので、自分の居た所は先頭に来るとは限らない**
 * （どの試験も座標で行を探し、上位 10 件に無ければ「残り N か所」を押す）。
 */
async function seedVisits(request: APIRequestContext, n: number): Promise<Candidate[]> {
  const lat0 = rand(5, 25);
  const lon0 = rand(100, 125);
  const spots: Spot[] = Array.from({ length: n }, (_, k) => ({ lat: round7(lat0 + 0.02 * k), lon: round7(lon0) }));
  const start = await freeDayStart(request);
  const items = spots.flatMap((s, k) =>
    Array.from({ length: 13 }, (_, m) => {
      const body = { lat: s.lat, lon: s.lon, acc_m: 10 };
      return {
        id: crypto.randomUUID(),
        user_id: USER,
        logical_source: "c01-location",
        external_id: null,
        device_id: `e2e-${TAG}`,
        origin: "collected",
        event_time: new Date(start + (k * 15 + m) * 60_000).toISOString(),
        tz_offset_min: 540,
        tz_id: "Asia/Tokyo",
        schema_version: 1,
        raw: JSON.stringify(body),
        payload: body,
      };
    }),
  );
  await ingest(request, items);
  const found = await apiCandidates(request);
  return spots.map((s) => {
    const hit = found.find((c) => Math.abs(c.lat - s.lat) < 0.001 && Math.abs(c.lon - s.lon) < 0.001);
    if (hit === undefined) throw new Error("送った位置から居た所ができていない（滞在の作り直しが走っていない）");
    return hit;
  });
}

const nonce = (): string => {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return btoa(String.fromCharCode(...bytes)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
};
let seq = 0;
/** API で作る記録の書いた日時。1 時間前から 1 秒ずつ進める（画面が押した時刻で書く版より必ず前になる） */
const stamp = (): string => new Date(Date.now() - 3_600_000 + seq++ * 1000).toISOString();

function placeItem(place: string, body: Record<string, unknown>): unknown {
  const id = crypto.randomUUID();
  return {
    id,
    user_id: USER,
    logical_source: "s01-place",
    external_id: null,
    device_id: null,
    origin: "authored",
    event_time: stamp(),
    tz_offset_min: 540,
    tz_id: "Asia/Tokyo",
    schema_version: 1,
    raw: JSON.stringify({ record: id, place, nonce: nonce(), ...body }),
    payload: {},
  };
}

/** 画面の「登録する」と同じ求め（器 → 名前・初めての座標・広さ）を API から送る。場所の識別子を返す。 */
async function registerViaApi(request: APIRequestContext, name: string, at: Spot, radius = 100): Promise<string> {
  const id = crypto.randomUUID();
  expect((await request.post("/api/places", { data: { id } })).ok()).toBe(true);
  await ingest(request, [
    placeItem(id, { field: "name", name }),
    placeItem(id, { field: "coord", lat: at.lat, lon: at.lon, change: "first", valid_from: null, supersedes: null }),
    placeItem(id, { field: "radius", radius_m: radius }),
  ]);
  return id;
}

const changeViaApi = (request: APIRequestContext, id: string, body: Record<string, unknown>): Promise<void> =>
  ingest(request, [placeItem(id, body)]);

const placeNamed = async (request: APIRequestContext, name: string): Promise<Place> => {
  const hit = (await apiPlaces(request)).find((p) => p.name === name);
  if (hit === undefined) throw new Error(`場所「${name}」が /api/places に無い`);
  return hit;
};

/** 場所の記録（`s01-place`）のうち、その場所の `field` のものの数。`/api/events` は全部を返す */
async function placeRecordCount(request: APIRequestContext, place: string, field: string): Promise<number> {
  const res = await request.get("/api/events");
  const rows = (await res.json()) as { logical_source: string; raw: string }[];
  return rows
    .filter((r) => r.logical_source === "s01-place" && r.raw.trim() !== "")
    .map((r) => JSON.parse(r.raw) as { place?: string; field?: string })
    .filter((r) => r.place === place && r.field === field).length;
}

// ------------------------------------------------------------------ 画面

async function openPlaces(page: Page): Promise<void> {
  // ハッシュだけの遷移は読み直さない（day-stays.spec.ts と同じ）
  await page.goto("/#/master/places");
  await page.reload();
  await expect(page.getByTestId("places-view")).toBeVisible();
  await expect(page.getByTestId("places-loading")).toBeHidden({ timeout: 15_000 });
}

const cards = (page: Page): Locator => page.getByTestId("place-card");
const cardOf = (page: Page, name: string): Locator =>
  cards(page).filter({ has: page.getByRole("heading", { name, exact: true }) });
const candRow = (page: Page, c: Candidate): Locator =>
  page.getByTestId("place-candidate").filter({ hasText: coordText(c.lat, c.lon) });

/** 居た所の行（上位 10 件に無ければ「残り N か所」を押して出す） */
async function showCandidate(page: Page, c: Candidate): Promise<Locator> {
  await expect(page.getByTestId("place-candidate").first()).toBeVisible();
  const row = candRow(page, c);
  if ((await row.count()) === 0) await page.getByRole("button", { name: /^残り \d+ か所$/ }).click();
  await expect(row).toHaveCount(1);
  return row;
}

/** 座標を変えるフォームの選択肢（上位 10 件に無ければ「残り N か所」を押して出す） */
async function showOption(card: Locator, c: Candidate): Promise<Locator> {
  await expect(card.getByTestId("coord-option").first()).toBeVisible();
  const option = card.getByTestId("coord-option").filter({ hasText: coordText(c.lat, c.lon) });
  const more = card.getByRole("button", { name: /^残り \d+ か所$/ });
  if ((await option.count()) === 0 && (await more.count()) > 0) await more.click();
  await expect(option).toHaveCount(1);
  return option;
}

/** 「場所を足す」→ その居た所の「名前を付ける」→ 名前と広さを入れる。送りはしない。 */
async function fillAdd(page: Page, c: Candidate, name: string, radius?: number): Promise<Locator> {
  await page.getByRole("button", { name: "場所を足す" }).click();
  const row = await showCandidate(page, c);
  await row.getByRole("button", { name: "名前を付ける" }).click();
  const form = row.getByTestId("place-add-form");
  await form.getByLabel("名前", { exact: true }).fill(name);
  if (radius !== undefined) await form.getByRole("radio", { name: `${radius} m` }).check();
  return form;
}

interface Sent {
  method: string;
  path: string;
  body: string | null;
}
/** 画面が送った求めを集める（本文の比較用）。宛先の origin も一緒に残す */
function watch(page: Page): { sent: Sent[]; origins: Set<string> } {
  const sent: Sent[] = [];
  const origins = new Set<string>();
  page.on("request", (req: Request) => {
    const u = new URL(req.url());
    if (u.protocol === "http:" || u.protocol === "https:") origins.add(u.origin);
    sent.push({ method: req.method(), path: u.pathname, body: req.postData() });
  });
  return { sent, origins };
}
const bodiesOf = (sent: Sent[], method: string, path: string): string[] =>
  sent.filter((s) => s.method === method && s.path === path).map((s) => s.body ?? "");
const countOf = (sent: Sent[], method: string, path: string): number => bodiesOf(sent, method, path).length;
const rawsOf = (body: string): Record<string, unknown>[] =>
  (JSON.parse(body) as { raw: string }[]).map((i) => JSON.parse(i.raw) as Record<string, unknown>);

const isPath =
  (path: string) =>
  (u: URL): boolean =>
    u.pathname === path;

/** `GET /api/places` を差し替える（POST は本物へ通す） */
async function stubPlaces(page: Page, places: Place[]): Promise<void> {
  await page.route(isPath("/api/places"), (route) =>
    route.request().method() === "GET" ? route.fulfill({ json: { today: TODAY, places } }) : route.fallback(),
  );
}
async function stubCandidates(page: Page, candidates: Candidate[]): Promise<void> {
  await page.route(isPath("/api/places/candidates"), (route) => route.fulfill({ json: { candidates } }));
}
const failWith = async (page: Page, path: string, status: number): Promise<void> => {
  await page.route(isPath(path), (route) => route.request().method() === "GET" ? route.fulfill({ status, body: "boom" }) : route.fallback());
};

/**
 * 送った `sent` 件のうち先頭が断られた応答。**サーバは 1 件ごとに 1 結果を返す**ので件数を揃える
 * （揃えないと画面は途中で本文が切れたと読み「届かなかった」にする。review/code.md R21）
 */
const refusal = (error: string, sent = 1): { id: string | null; duplicate: boolean; accepted: boolean; error: string | null }[] => [
  { id: null, duplicate: false, accepted: false, error },
  ...Array.from({ length: sent - 1 }, () => ({ id: "x", duplicate: false, accepted: true, error: null })),
];

/** 取り込みの求めを 1 回目だけ「届いたが応答が返らなかった」にする（サーバへ通してから応答を捨てる） */
async function dropFirstIngestReply(page: Page): Promise<{ calls: () => number }> {
  let n = 0;
  await page.route("**/api/ingest", async (route: Route) => {
    n += 1;
    if (n === 1) {
      await route.fetch();
      await route.abort("failed");
      return;
    }
    await route.fallback();
  });
  return { calls: () => n };
}

/** 応答を保留にする門。`open()` で流す */
function gate(): { wait: Promise<void>; open: () => void } {
  let open: () => void = () => undefined;
  const wait = new Promise<void>((resolve) => {
    open = resolve;
  });
  return { wait, open };
}

// ------------------------------------------------------------------ 差し替え用の場所

const T = { written_at: "2026-09-01T09:00:00+09:00", ingested_at: "2026-09-01T00:00:01Z" };
const HOURS = (peak: number): number[] => Array.from({ length: 24 }, (_, h) => (h === peak ? 60 : h === peak + 1 ? 30 : 0));

function fakePlace(i: number, over: Partial<Place> = {}): Place {
  const rid = (k: number): string => `f${i}000000-0000-4000-8000-00000000000${k}`;
  const first = { ...T, record_id: rid(1), lat: 35 + i / 100, lon: 139 + i / 100, change: "first" as const, valid_from: null, supersedes: null };
  const before: PreviousCoord = { ...first, record_id: rid(2), state: "fixed", fixed_by: rid(1) };
  return {
    id: `f${i}000000-0000-4000-8000-000000000000`,
    name: `差し替え${i}`,
    note: null,
    radius_m: 100,
    name_record: { ...T, record_id: rid(3) },
    coord: first,
    stays: { count: 3, minutes: 150, last_day: `2026-09-${20 + i}`, hours: HOURS(9) },
    previous_names: [{ ...T, record_id: rid(4), name: `旧名${i}` }],
    previous_coords: [before],
    ...over,
  };
}

const fakeCandidate = (i: number): Candidate => ({
  lat: 34 + i / 100,
  lon: 138 + i / 100,
  stays: { count: 2, minutes: 45, first_day: "2026-09-01", last_day: "2026-09-30", hours: HOURS(10) },
});

// ------------------------------------------------------------------ 色（getComputedStyle から比を計算する）

type Rgba = [number, number, number, number];

function parseColor(s: string): Rgba {
  const nums = s.match(/-?[\d.]+/g)?.map(Number) ?? [];
  if (s.startsWith("color(")) return [nums[0] * 255, nums[1] * 255, nums[2] * 255, nums[3] ?? 1];
  if (s.startsWith("rgb")) return [nums[0], nums[1], nums[2], nums[3] ?? 1];
  throw new Error(`読めない色: ${s}`);
}

/** 外側から重ねる。いちばん外が透明なら白（ブラウザの既定の地） */
function composite(layers: string[]): [number, number, number] {
  let out: [number, number, number] = [255, 255, 255];
  for (const layer of [...layers].reverse()) {
    const [r, g, b, a] = parseColor(layer);
    out = [r * a + out[0] * (1 - a), g * a + out[1] * (1 - a), b * a + out[2] * (1 - a)];
  }
  return out;
}

function luminance([r, g, b]: [number, number, number]): number {
  const lin = (c: number): number => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function ratio(a: [number, number, number], b: [number, number, number]): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

interface Painted {
  label: string;
  fg: string;
  /** 自分から外へ向かう背景の色（`background-color`） */
  bgs: string[];
}

/** 画面の中の「文字を持つ要素」（と入力欄）の色と、重なる背景の色。 */
async function paintedText(page: Page): Promise<Painted[]> {
  return page.evaluate(() => {
    const root = document.querySelector("[data-testid=master-view]");
    if (root === null) return [];
    const out: { label: string; fg: string; bgs: string[] }[] = [];
    for (const el of Array.from(root.querySelectorAll("*"))) {
      if (el.tagName === "STYLE") continue;
      const own = Array.from(el.childNodes).some((n) => n.nodeType === 3 && (n.textContent ?? "").trim() !== "");
      if (!own && !el.matches("input:not([type=radio])")) continue;
      if (el.getClientRects().length === 0) continue;
      const bgs: string[] = [];
      for (let n: Element | null = el; n !== null; n = n.parentElement) bgs.push(getComputedStyle(n).backgroundColor);
      const text = (el.textContent ?? "").trim().slice(0, 24);
      out.push({ label: `<${el.tagName.toLowerCase()}> ${text}`, fg: getComputedStyle(el).color, bgs });
    }
    return out;
  });
}

// ================================================================== 10.1 タブと一覧

// Scenario: タブは個人属性と場所の 2 つで人物のタブは無い
// Scenario: 場所のタブを押すと場所の画面に移る
test("マスタ管理のタブは「個人属性」「場所」の 2 つで、場所を押すと場所の画面に移る", async ({ page }) => {
  await page.goto("/#/master");
  await page.reload();
  const tabs = page.getByRole("tab");
  await expect(tabs).toHaveText(["個人属性", "場所"]);
  await expect(tabs.nth(0)).toHaveAttribute("aria-selected", "true");
  await expect(tabs.nth(1)).toHaveAttribute("aria-selected", "false");
  await expect(page.getByRole("tab", { name: /人物/ })).toHaveCount(0);

  await tabs.nth(1).click();
  await expect(page.getByRole("tab", { name: "場所" })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByRole("tab", { name: "個人属性" })).toHaveAttribute("aria-selected", "false");
  await expect(page).toHaveURL(/#\/master\/places$/);
  await expect(page.getByTestId("places-view")).toBeVisible();
  await expect(page.getByTestId("kind-card")).toHaveCount(0);
  await expect(page.getByTestId("write-form")).toHaveCount(0);
});

// Scenario: 場所のタブには登録した場所のカードだけが出る
// Scenario: 場所のカードは最近居た順に並ぶ
// Scenario: 場所のカードに合計と最後に居た日と広さと帯と座標が出る
// Scenario: 滞在の当たらない場所はまだ居たことが無いと出る
test("場所のカードは登録した場所だけを最近居た順に並べ、合計・最後に居た日・広さ・帯・座標を押さずに見せる", async ({ page, request }) => {
  const [a, b, unnamed] = await seedVisits(request, 3);
  const nameA = `居た所A-${TAG}`;
  const nameB = `居た所B-${TAG}`;
  const nameC = `居なかった所-${TAG}`;
  await registerViaApi(request, nameA, a, 100);
  await registerViaApi(request, nameB, b, 300);
  const far = { lat: round7(rand(-30, -10)), lon: round7(rand(-70, -50)) }; // 位置の記録が 1 件も無い所
  await registerViaApi(request, nameC, far, 50);

  await openPlaces(page);
  const api = await apiPlaces(request);

  // 並び: /api/places の順（最後に居た日の新しい順）のまま。居たことの無い場所は後ろ
  expect((await cards(page).count()), "カードの数は登録した場所の数").toBe(api.length);
  await expect(page.getByTestId("place-name")).toHaveText(api.map((p) => p.name));
  const days = api.map((p) => p.stays.last_day);
  expect(days.filter((d) => d !== null)).toEqual([...days.filter((d): d is string => d !== null)].sort().reverse());
  expect(days.indexOf(null) === -1 || days.slice(days.indexOf(null)).every((d) => d === null), "居たことの無い場所は最後に集まる").toBe(true);

  // 名前の無い居た所は、カードにならず、座標も出ていない
  await expect(page.getByTestId("place-add-panel")).toHaveCount(0);
  expect(await page.evaluate(() => document.body.innerText)).not.toContain(coordText(unnamed.lat, unnamed.lon));

  // 合計・最後に居た日・広さ・24 区分の帯・座標が、押さずに見えている
  for (const [name, at, radius] of [
    [nameA, a, 100],
    [nameB, b, 300],
  ] as const) {
    const apiPlace = api.find((p) => p.name === name)!;
    expect(apiPlace.stays.count, `${name} に滞在が当たっていない`).toBeGreaterThan(0);
    const card = cardOf(page, name);
    await expect(card.getByTestId("place-total")).toHaveText(durationText(apiPlace.stays.minutes));
    await expect(card.getByTestId("place-meta")).toHaveText(`最後に居た日 ${apiPlace.stays.last_day} · 広さ ${radius} m`);
    await expect(card.getByTestId("place-band")).toBeVisible();
    await expect(card.locator("[data-testid=place-band] [data-hour]")).toHaveCount(24);
    await expect(card.getByTestId("place-coord")).toHaveText(coordText(at.lat, at.lon));
    const box = await card.getByTestId("place-band").boundingBox();
    expect(box?.width ?? 0, `${name} の帯に幅が無い`).toBeGreaterThan(0);
    expect(box?.height ?? 0, `${name} の帯に高さが無い`).toBeGreaterThan(0);
  }

  // 滞在の当たらない場所
  const never = cardOf(page, nameC);
  await expect(never.getByTestId("place-meta")).toContainText("まだ居たことが無い");
  await expect(never.getByTestId("place-meta")).not.toContainText("最後に居た日");
  await expect(never.getByTestId("place-meta")).toContainText("広さ 50 m");
});

// Scenario: 前の名前と座標は押したときだけ出る
// Scenario: 前の座標は直したか移ったかが文字で出る
// Scenario: 前の座標の直したは文字で出る
// Scenario: 予定の移転は予定の文字で出る
// Scenario: 場所の識別子は画面に出ない
test("前の名前・座標は押したときだけ出て、直した・移る前・予定が文字で出て、どの識別子も画面に出ない", async ({ page, request }) => {
  const oldName = `前の名前-${TAG}`;
  const name = `いまの名前-${TAG}`;
  const at = (k: number): Spot => ({ lat: round7(rand(-30, -10) + k * 0.05), lon: round7(rand(-70, -50)) });
  const [p0, p1, p2, p3] = [at(0), at(1), at(2), at(3)];
  const id = await registerViaApi(request, oldName, p0);
  await changeViaApi(request, id, { field: "name", name });
  const first = (await placeNamed(request, name)).coord.record_id;
  // p0 → p1（間違いを直す）→ p2（2026-04 に移った）→ p3（2099-01 に移る予定）
  await changeViaApi(request, id, { field: "coord", lat: p1.lat, lon: p1.lon, change: "fix", valid_from: null, supersedes: first });
  await changeViaApi(request, id, {
    field: "coord", lat: p2.lat, lon: p2.lon, change: "move", valid_from: { precision: "month", date: "2026-04" }, supersedes: null,
  });
  await changeViaApi(request, id, {
    field: "coord", lat: p3.lat, lon: p3.lon, change: "move", valid_from: { precision: "month", date: "2099-01" }, supersedes: null,
  });

  await openPlaces(page);
  const card = cardOf(page, name);
  const toggle = card.getByRole("button", { name: /^前の名前・座標 \d+/ });
  await expect(toggle).toBeVisible();
  const placeApi = await placeNamed(request, name);
  expect(placeApi.previous_names.length + placeApi.previous_coords.length).toBeGreaterThanOrEqual(4);

  // 押す前は前の名前も前の座標も見えない
  await expect(card.getByTestId("place-previous")).toHaveCount(0);
  await expect(card).not.toContainText(oldName);
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await toggle.click();
  await expect(card.getByTestId("place-previous")).toBeVisible();
  await expect(card.getByTestId("previous-name")).toContainText(oldName);

  // 直した / 移る前（〜2026-04）/ 予定（2099-01 から）が文字で出る（色や記号だけではない）
  const coords = card.getByTestId("previous-coord");
  await expect(coords.filter({ hasText: coordText(p0.lat, p0.lon) })).toContainText("直した");
  const moved = coords.filter({ hasText: coordText(p1.lat, p1.lon) });
  await expect(moved).toContainText("移る前");
  await expect(moved).toContainText("2026-04");
  await expect(moved).not.toContainText("直した");
  const upcoming = coords.filter({ hasText: coordText(p3.lat, p3.lon) });
  await expect(upcoming).toContainText("予定");
  await expect(upcoming).toContainText("2099-01");

  // どの場所のどの識別子も画面の文字に無い（全カードの「前の名前・座標」を開いた状態で）
  const closed = page.getByRole("button", { name: /^前の名前・座標 \d+ ▸$/ });
  // 開くたびに 1 つ減る。減らなければ上限で止めて、開ききったことを確かめる（無限に回さない）
  for (let left = await closed.count(), guard = 0; left > 0 && guard < 50; left = await closed.count(), guard += 1) await closed.first().click();
  await expect(closed).toHaveCount(0);
  const text = await page.evaluate(() => document.body.innerText);
  const all = await apiPlaces(request);
  const ids = all.flatMap((p) => [
    p.id,
    p.name_record.record_id,
    p.coord.record_id,
    ...p.previous_names.map((n) => n.record_id),
    ...p.previous_coords.map((k) => k.record_id),
  ]);
  expect(ids.length).toBeGreaterThan(0);
  for (const one of ids) expect(text, `識別子 ${one} が画面に出ている`).not.toContain(one);
});

// Scenario: 場所がまだ無いと出る
test("場所が 1 つも無いと、場所がまだ無いことと「場所を足す」が出る", async ({ page }) => {
  await stubPlaces(page, []);
  await openPlaces(page);
  await expect(page.getByTestId("places-empty")).toContainText("場所がまだありません");
  await expect(cards(page)).toHaveCount(0);
  await expect(page.getByRole("button", { name: "場所を足す" })).toBeVisible();
  await expect(page.getByTestId("places-failed")).toHaveCount(0);
});

// Scenario: 場所の画面は外へ求めを送らない
test("場所のタブ・場所を足す・名前を付けるフォーム・座標を変えるフォームを開いても、求めの宛先は画面を配るサーバだけ", async ({ page, request }) => {
  const [a, b] = await seedVisits(request, 2);
  const name = `外へ送らない-${TAG}`;
  await registerViaApi(request, name, a);
  const { origins } = watch(page);

  await openPlaces(page);
  await page.getByRole("button", { name: "場所を足す" }).click();
  const row = await showCandidate(page, b);
  await row.getByRole("button", { name: "名前を付ける" }).click();
  await expect(row.getByTestId("place-add-form")).toBeVisible();
  await cardOf(page, name).getByRole("button", { name: "座標を変える" }).click();
  await expect(cardOf(page, name).getByTestId("place-change-form")).toBeVisible();
  await expect(cardOf(page, name).getByTestId("coord-option").first()).toBeVisible();

  const own = new URL(page.url()).origin;
  expect([...origins], "画面が求めを送った宛先").toEqual([own]);
});

// Scenario: 登録 6 の量で 1 画面目に場所が 3 枚見える
test("登録 6 の量（当たった滞在と前の名前・座標つき）で、幅 360・高さ 640 の 1 画面目に場所が 3 枚見える（design D21）", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 640 });
  await stubPlaces(page, Array.from({ length: 6 }, (_, i) => fakePlace(i)));
  await openPlaces(page);
  await expect(cards(page)).toHaveCount(6);
  const tops: number[] = [];
  for (const card of await cards(page).all()) tops.push((await card.boundingBox())?.y ?? Number.POSITIVE_INFINITY);
  const inFirstScreen = tops.filter((y) => y < 640).length;
  expect(inFirstScreen, `1 画面目（上から 640 px）に上端が入るカード。上端: ${tops.map(Math.round).join(", ")}`).toBeGreaterThanOrEqual(3);
});

// ================================================================== 10.2 足す

// Scenario: 場所を足すを押すと名前の無い居た所が上位 10 件出る
// Scenario: 残りを押すと全部出る
// Scenario: 名前の無い居た所に手がかりが出る
test("場所を足すを押すと、名前の無い居た所が上位 10 件と「残り N か所」で出て、残りを押すと全部出る", async ({ page, request }) => {
  await seedVisits(request, 13);
  await openPlaces(page);
  await page.getByRole("button", { name: "場所を足す" }).click();
  const api = await apiCandidates(request);
  expect(api.length, "居た所を 13 以上にしてある").toBeGreaterThanOrEqual(13);

  const rows = page.getByTestId("place-candidate");
  await expect(rows).toHaveCount(10);
  const rest = page.getByRole("button", { name: /^残り \d+ か所$/ });
  await expect(rest).toHaveText(`残り ${api.length - 10} か所`);
  // 並びは API の順のまま
  await expect(page.getByTestId("candidate-coord")).toHaveText(api.slice(0, 10).map((c) => coordText(c.lat, c.lon)));

  // 手がかり: 滞在の長さの合計・24 区分の帯・滞在の件数・最初と最後に居た日・「名前を付ける」
  for (let i = 0; i < 10; i++) {
    const row = rows.nth(i);
    const c = api[i];
    await expect(row.getByTestId("candidate-total")).toHaveText(durationText(c.stays.minutes));
    await expect(row.locator("[data-testid=place-band] [data-hour]")).toHaveCount(24);
    await expect(row.getByTestId("candidate-meta")).toHaveText(`${c.stays.count} 件 · ${c.stays.first_day} 〜 ${c.stays.last_day}`);
    await expect(row.getByRole("button", { name: "名前を付ける" })).toBeVisible();
  }

  await rest.click();
  await expect(rows).toHaveCount(api.length);
  await expect(rest).toHaveCount(0);
});

// Scenario: 名前を付けるフォームに緯度経度の欄が無い
// Scenario: 広さの最初は 100 m
test("名前を付けるフォームは名前・座標・広さ・補足で、座標は書き換えられず、広さの最初は 100 m", async ({ page, request }) => {
  const [c] = await seedVisits(request, 1);
  await openPlaces(page);
  const form = await fillAdd(page, c, `フォーム-${TAG}`);
  await expect(form.getByLabel("名前", { exact: true })).toBeVisible();
  await expect(form.getByLabel("補足")).toBeVisible();
  await expect(form.getByTestId("place-add-coord")).toHaveText(coordText(c.lat, c.lon));
  // 入力欄は名前と補足の 2 つだけ（座標・緯度・経度の欄が無い）。座標は文字で、書き換えられない
  await expect(form.locator("input:not([type=radio])")).toHaveCount(2);
  await expect(form.getByTestId("place-add-coord").locator("input")).toHaveCount(0);
  await expect(form.getByLabel(/緯度|経度|lat|lon/i)).toHaveCount(0);
  await expect(form.getByRole("button", { name: /いまの位置|現在地|位置を使う|位置情報/ })).toHaveCount(0);
  // 広さの選択肢と最初の値
  await expect(form.getByRole("radio")).toHaveCount(4);
  await expect(form.getByRole("radio", { name: "100 m" })).toBeChecked();
  for (const m of [50, 200, 300]) await expect(form.getByRole("radio", { name: `${m} m` })).not.toBeChecked();
});

// Scenario: 名前を付けて登録するとカードが増える
// Scenario: 登録で送る座標は居た所の中心
// Scenario: 登録の書いた日時は入力させない
// Scenario: 登録が受理されるとフォームが閉じる
test("名前と広さ 200 m で登録すると、居た所の中心の座標で送られ、書いた日時は押した時刻で、フォームが閉じてカードが増える", async ({ page, request }) => {
  const [c] = await seedVisits(request, 1);
  const name = `スーパー-${TAG}`;
  const fixed = new Date(Date.now() - 120_000);
  await page.clock.setFixedTime(fixed);
  const { sent } = watch(page);
  await openPlaces(page);
  const before = (await apiPlaces(request)).length;
  const form = await fillAdd(page, c, name, 200);
  // 書いた日時を入れる欄は無い
  await expect(form.getByLabel(/日時|書いた|時刻/)).toHaveCount(0);
  const reads = { places: countOf(sent, "GET", "/api/places"), candidates: countOf(sent, "GET", "/api/places/candidates") };
  await form.getByRole("button", { name: "登録する" }).click();

  await expect(page.getByTestId("place-add-form")).toHaveCount(0);
  await expect(cardOf(page, name)).toHaveCount(1);
  // 受理された後に、場所と名前の無い居た所がもう一度読み出される
  expect(countOf(sent, "GET", "/api/places")).toBeGreaterThan(reads.places);
  expect(countOf(sent, "GET", "/api/places/candidates")).toBeGreaterThan(reads.candidates);

  // 送った本文: 初めての座標は居た所の中心、書いた日時は押した時刻
  const container = bodiesOf(sent, "POST", "/api/places");
  expect(container).toHaveLength(1);
  const placeId = (JSON.parse(container[0]) as { id: string }).id;
  const ingestBodies = bodiesOf(sent, "POST", "/api/ingest");
  expect(ingestBodies).toHaveLength(1);
  const items = JSON.parse(ingestBodies[0]) as { event_time: string }[];
  const raws = rawsOf(ingestBodies[0]);
  const coord = raws.find((r) => r.field === "coord");
  expect(coord, "座標の記録が送られている").toBeDefined();
  expect(coord).toMatchObject({ lat: c.lat, lon: c.lon, change: "first", place: placeId });
  expect(raws.find((r) => r.field === "radius")).toMatchObject({ radius_m: 200 });
  expect(items.map((i) => i.event_time)).toEqual(items.map(() => fixed.toISOString()));

  // 読み直した後: カードがあり、広さは 200 m、その居た所は名前の無い居た所に無い
  await page.reload();
  await expect(cardOf(page, name)).toHaveCount(1);
  await expect(cardOf(page, name).getByTestId("place-meta")).toContainText("広さ 200 m");
  expect((await apiPlaces(request)).length).toBe(before + 1);
  await page.getByRole("button", { name: "場所を足す" }).click();
  await expect(page.getByTestId("candidate-coord").filter({ hasText: coordText(c.lat, c.lon) })).toHaveCount(0);
  expect((await apiCandidates(request)).some((k) => k.lat === c.lat && k.lon === c.lon)).toBe(false);
});

// Scenario: 登録を 2 回押しても場所は 1 つ
test("登録の応答が返らず押し直しても、器の識別子と各記録の原文は同じで、場所は 1 つ増えるだけ", async ({ page, request }) => {
  const [c] = await seedVisits(request, 1);
  const name = `二度押し-${TAG}`;
  const { sent } = watch(page);
  const dropped = await dropFirstIngestReply(page);
  await openPlaces(page);
  const before = (await apiPlaces(request)).length;
  const form = await fillAdd(page, c, name, 200);
  const register = form.getByRole("button", { name: "登録する" });

  await register.click();
  await expect(page.getByTestId("place-problem")).toContainText("サーバに届きませんでした");
  expect(dropped.calls(), "1 回目はサーバへ届いている").toBe(1);
  // 入力は残っている
  await expect(form.getByLabel("名前", { exact: true })).toHaveValue(name);
  await expect(form.getByRole("radio", { name: "200 m" })).toBeChecked();

  await register.click();
  await expect(page.getByTestId("place-add-form")).toHaveCount(0);
  const containers = bodiesOf(sent, "POST", "/api/places");
  const ingests = bodiesOf(sent, "POST", "/api/ingest");
  expect(containers).toHaveLength(2);
  expect(ingests).toHaveLength(2);
  expect(containers[1], "器の識別子が同じ").toBe(containers[0]);
  expect(ingests[1], "各記録の識別子・原文・書いた日時が同じ").toBe(ingests[0]);
  expect(rawsOf(ingests[1])).toEqual(rawsOf(ingests[0]));

  expect((await apiPlaces(request)).length, "読み直した場所の数").toBe(before + 1);
  expect((await apiPlaces(request)).filter((p) => p.name === name)).toHaveLength(1);
});

// 実サーバへ通す（final review I1）: 1 回目の束はサーバで受理され応答だけが落ちた後、名前を打ち直して押し直しても、
// 器は 1 つで、座標・広さの記録は 1 件ずつ（変えた名前の記録だけが組み直される。design D13（仮））
test("登録の応答が返らず名前を直して押し直しても、器の識別子は同じで、座標の記録は 1 件・場所は 1 つ増えるだけ", async ({ page, request }) => {
  const [c] = await seedVisits(request, 1);
  const typo = `打ち間違い-${TAG}`;
  const name = `打ち直し-${TAG}`;
  const { sent } = watch(page);
  const dropped = await dropFirstIngestReply(page);
  await openPlaces(page);
  const before = (await apiPlaces(request)).length;
  const form = await fillAdd(page, c, typo);
  await form.getByRole("button", { name: "登録する" }).click();
  await expect(page.getByTestId("place-problem")).toContainText("サーバに届きませんでした");
  expect(dropped.calls(), "1 回目はサーバへ届いている").toBe(1);

  await form.getByLabel("名前", { exact: true }).fill(name);
  await form.getByRole("button", { name: "登録する" }).click();
  await expect(page.getByTestId("place-add-form")).toHaveCount(0);

  const ids = bodiesOf(sent, "POST", "/api/places").map((b) => (JSON.parse(b) as { id: string }).id);
  expect(ids).toHaveLength(2);
  expect(ids[1], "器の識別子が同じ").toBe(ids[0]);
  const place = ids[0];
  expect(await placeRecordCount(request, place, "coord"), "座標の記録は 1 件").toBe(1);
  expect(await placeRecordCount(request, place, "radius"), "広さの記録は 1 件").toBe(1);
  expect(await placeRecordCount(request, place, "name"), "名前の記録は打ち間違いと打ち直しの 2 件").toBe(2);

  const after = await apiPlaces(request);
  expect(after.length, "場所は 1 つ増えるだけ").toBe(before + 1);
  const mine = after.filter((p) => p.name === name || p.name === typo);
  expect(mine).toHaveLength(1);
  expect(mine[0].id).toBe(place);
  expect(mine[0].name, "いまの名前は打ち直した名前").toBe(name);
});

// Scenario: 器の識別子が取られていたら押し直しで識別子を作り直す
test("器を作る求めが place_id_taken で断られたら、押し直しの器の識別子は 1 回目と違う", async ({ page, request }) => {
  const [c] = await seedVisits(request, 1);
  const name = `識別子の衝突-${TAG}`;
  const { sent } = watch(page);
  let n = 0;
  await page.route(isPath("/api/places"), (route) => {
    if (route.request().method() !== "POST") return route.fallback();
    n += 1;
    return n === 1 ? route.fulfill({ status: 400, json: { error: "place_id_taken" } }) : route.fallback();
  });
  await openPlaces(page);
  const form = await fillAdd(page, c, name);
  await form.getByRole("button", { name: "登録する" }).click();
  await expect(page.getByTestId("place-problem")).toContainText("もう一度");
  expect(bodiesOf(sent, "POST", "/api/ingest"), "器が断られたら記録は送らない").toHaveLength(0);

  await form.getByRole("button", { name: "登録する" }).click();
  await expect(page.getByTestId("place-add-form")).toHaveCount(0);
  const ids = bodiesOf(sent, "POST", "/api/places").map((b) => (JSON.parse(b) as { id: string }).id);
  expect(ids).toHaveLength(2);
  expect(ids[1], "2 回目の識別子は 1 回目と違う").not.toBe(ids[0]);
  expect((await apiPlaces(request)).filter((p) => p.name === name)).toHaveLength(1);
});

// Scenario: 登録を送っている間は登録するを押せない
test("登録を送っている間は「登録する」が押せず、応答が返ると閉じる", async ({ page, request }) => {
  const [c] = await seedVisits(request, 1);
  const hold = gate();
  await page.route("**/api/ingest", async (route) => {
    await hold.wait;
    await route.fallback();
  });
  await openPlaces(page);
  const form = await fillAdd(page, c, `送信中-${TAG}`);
  const register = form.getByRole("button", { name: "登録する" });
  await expect(register).toBeEnabled();
  await register.click();
  await expect(register).toBeDisabled();
  hold.open();
  await expect(page.getByTestId("place-add-form")).toHaveCount(0);
});

// Scenario: 名前が空だと断られて入力が残る
// Scenario: 場所の登録が届かなかったとき入力が残り届かなかったと出る
test("名前が空だと断られたときと届かなかったときで文が違い、どちらも入力が残る", async ({ page, request }) => {
  const [c] = await seedVisits(request, 1);
  await page.route(isPath("/api/places"), (route) =>
    route.request().method() === "POST" ? route.fulfill({ json: { id: "x" } }) : route.fallback(),
  );
  let mode: "refuse" | "drop" = "refuse";
  await page.route("**/api/ingest", (route) =>
    mode === "refuse"
      ? route.fulfill({ status: 200, json: refusal("invalid_place_name", (route.request().postDataJSON() as unknown[]).length) })
      : route.abort("failed"),
  );
  await openPlaces(page);
  // 名前が空白だけのうちは「登録する」を押せない（design D13（仮）。送ると名前だけ断られ、位置と補足が名前の無い器に残る）。
  // 断られる応答（WHEN）は差し替えで作る
  const form = await fillAdd(page, c, " ", 300);
  await expect(form.getByRole("button", { name: "登録する" })).toBeDisabled();
  await form.getByLabel("名前", { exact: true }).fill(`断られる-${TAG}`);
  await form.getByLabel("補足").fill("メモ");
  await form.getByRole("button", { name: "登録する" }).click();
  const problem = page.getByTestId("place-problem");
  await expect(problem).toContainText("名前が空");
  const refused = await problem.innerText();
  await expect(form.getByRole("radio", { name: "300 m" })).toBeChecked();
  await expect(form.getByLabel("補足")).toHaveValue("メモ");

  mode = "drop";
  await form.getByLabel("名前", { exact: true }).fill(`届かない-${TAG}`);
  await form.getByRole("button", { name: "登録する" }).click();
  await expect(problem).toContainText("サーバに届きませんでした");
  expect(await problem.innerText(), "断られたときの文と違う").not.toBe(refused);
  await expect(form.getByLabel("名前", { exact: true })).toHaveValue(`届かない-${TAG}`);
  await expect(form.getByRole("radio", { name: "300 m" })).toBeChecked();
  await expect(form.getByLabel("補足")).toHaveValue("メモ");
});

// Scenario: 居た所がまだ無いと出る
test("名前の無い居た所が 1 つも無いと、場所を足すで居た所がまだ無いことが出る", async ({ page }) => {
  await stubCandidates(page, []);
  await openPlaces(page);
  await page.getByRole("button", { name: "場所を足す" }).click();
  await expect(page.getByTestId("candidates-empty")).toContainText("居た所がまだありません");
  await expect(page.getByTestId("place-candidate")).toHaveCount(0);
});

// ================================================================== 10.3 変える

/** 変える試験の下ごしらえ: 居た所を 3 つ作り、1 つ目に場所を登録する（広さ 200 m）。残り 2 つは名前の無い居た所 */
async function placeToChange(request: APIRequestContext, label: string): Promise<{ name: string; id: string; here: Candidate; others: Candidate[] }> {
  const [here, ...others] = await seedVisits(request, 3);
  const name = `${label}-${TAG}`;
  const id = await registerViaApi(request, name, here, 200);
  return { name, id, here, others };
}

const changeButton = (card: Locator, name: string): Locator => card.getByRole("button", { name, exact: true });
const sendChange = (card: Locator): Locator => card.getByTestId("place-change-form").getByRole("button", { name: "変える", exact: true });

// Scenario: 名前を変えるとカードの名前が変わる
// Scenario: 変えるが受理されるとフォームが閉じる
test("名前を変えて送ると、フォームが閉じて場所が読み直され、カードの名前が変わって 1 枚のまま", async ({ page, request }) => {
  const { name } = await placeToChange(request, "職場");
  const renamed = `本社-${TAG}`;
  const { sent } = watch(page);
  await openPlaces(page);
  const total = await cards(page).count();
  const card = cardOf(page, name);
  await changeButton(card, "名前を変える").click();
  const input = card.getByLabel("名前", { exact: true });
  await expect(input).toHaveValue(name);
  await input.fill(renamed);
  const reads = countOf(sent, "GET", "/api/places");
  await sendChange(card).click();

  await expect(card.getByTestId("place-change-form")).toHaveCount(0);
  await expect(page.getByTestId("place-card")).toHaveCount(total);
  expect(countOf(sent, "GET", "/api/places"), "受理の後に場所が読み直される").toBeGreaterThan(reads);
  await page.reload();
  await expect(cardOf(page, renamed)).toHaveCount(1);
  await expect(cardOf(page, name)).toHaveCount(0);
  await expect(cards(page)).toHaveCount(total);
});

// Scenario: 広さを変えるといまの広さが選ばれている
// Scenario: 広さを変えるとカードの広さが変わる
test("広さを変えるを開くといまの 200 m が選ばれていて、300 m を選んで送るとカードの広さが変わる", async ({ page, request }) => {
  const { name } = await placeToChange(request, "広さ");
  await openPlaces(page);
  const card = cardOf(page, name);
  await expect(card.getByTestId("place-meta")).toContainText("広さ 200 m");
  await changeButton(card, "広さを変える").click();
  await expect(card.getByRole("radio", { name: "200 m" })).toBeChecked();
  await card.getByRole("radio", { name: "300 m" }).check();
  await sendChange(card).click();
  await expect(card.getByTestId("place-change-form")).toHaveCount(0);
  await page.reload();
  await expect(cardOf(page, name).getByTestId("place-meta")).toContainText("広さ 300 m");
  expect((await placeNamed(request, name)).radius_m).toBe(300);
});

// Scenario: 座標を変えるには直すか移ったかを選ぶ
// Scenario: 座標の新しい値は居た所から選ぶ
// Scenario: 間違いを直すといまの座標の記録を直す先に送る
// Scenario: 移ったを選ぶといつからを精度から入れる
// Scenario: 移ったを選ぶと前の時間もこの場所のままと出る
// Scenario: 移ったを送ると移ったの記録が送られる
test("座標を変えるは居た所から選び、直すか移ったかを選ぶまで送れず、直すと直す先つきで、移ったと年月つきで送られる", async ({ page, request }) => {
  const { name, id, others } = await placeToChange(request, "移転");
  const [near, far] = others;
  const { sent } = watch(page);
  await openPlaces(page);
  const card = cardOf(page, name);
  const form = card.getByTestId("place-change-form");

  // 居た所から選ぶ。緯度経度の欄といまの位置を使う操作は無い。どちらも選ばないうちは送れない
  await changeButton(card, "座標を変える").click();
  const nearOption = await showOption(card, near);
  await showOption(card, far);
  await expect(form.locator("input:not([type=radio])")).toHaveCount(0);
  await expect(form.getByLabel(/緯度|経度|lat|lon/i)).toHaveCount(0);
  await expect(form.getByRole("button", { name: /いまの位置|現在地|位置を使う|位置情報/ })).toHaveCount(0);
  await expect(sendChange(card)).toBeDisabled();
  await nearOption.getByRole("radio").check();
  await expect(sendChange(card), "居た所だけ選んでも、直すか移ったかが未選択のうちは送れない").toBeDisabled();

  // 間違いを直す
  const current = (await placeNamed(request, name)).coord.record_id;
  await card.getByRole("radio", { name: "前の座標が間違っていた" }).check();
  await expect(sendChange(card)).toBeEnabled();
  await sendChange(card).click();
  await expect(form).toHaveCount(0);
  const fix = rawsOf(bodiesOf(sent, "POST", "/api/ingest")[0])[0];
  expect(fix).toMatchObject({ field: "coord", change: "fix", supersedes: current, lat: near.lat, lon: near.lon, place: id });

  // 移った（年月 2026-04）。精度を選ぶと年と月の欄だけが出て、日の欄は出ない
  await page.reload();
  await changeButton(card, "座標を変える").click();
  await (await showOption(card, far)).getByRole("radio").check();
  await card.getByRole("radio", { name: "この場所が移った" }).check();
  await expect(card.getByTestId("move-note")).toContainText(`前の座標で居た時間も「${name}」のまま`);
  await card.getByRole("radio", { name: "年月", exact: true }).check();
  await expect(card.getByLabel("いつから（年）")).toBeVisible();
  await expect(card.getByLabel("いつから（月）")).toBeVisible();
  await expect(card.getByLabel("いつから（日）")).toHaveCount(0);
  await card.getByLabel("いつから（年）").fill("2026");
  await card.getByLabel("いつから（月）").fill("04");
  await sendChange(card).click();
  await expect(form).toHaveCount(0);
  const move = rawsOf(bodiesOf(sent, "POST", "/api/ingest")[1])[0];
  expect(move).toMatchObject({
    field: "coord", change: "move", lat: far.lat, lon: far.lon, place: id,
    valid_from: { precision: "month", date: "2026-04" }, supersedes: null,
  });
  const after = await placeNamed(request, name);
  expect(after.coord).toMatchObject({ lat: far.lat, lon: far.lon, change: "move" });
  expect(after.id, "識別子は変わらない").toBe(id);
});

// Scenario: 座標を変える先の居た所が無いと送れない
test("名前の無い居た所が無いと、座標を変えるに居た所がまだ無いことが出て、送れない", async ({ page, request }) => {
  const { name } = await placeToChange(request, "居た所なし");
  await stubCandidates(page, []);
  await openPlaces(page);
  const card = cardOf(page, name);
  await changeButton(card, "座標を変える").click();
  await expect(card.getByTestId("candidates-empty")).toContainText("居た所がまだありません");
  await card.getByRole("radio", { name: "前の座標が間違っていた" }).check();
  await expect(sendChange(card)).toBeDisabled();
});

// Scenario: 変えるを送っている間は押せない
test("名前を変えて送っている間は、送る操作が押せない", async ({ page, request }) => {
  const { name } = await placeToChange(request, "送信中の変更");
  const hold = gate();
  await page.route("**/api/ingest", async (route) => {
    await hold.wait;
    await route.fallback();
  });
  await openPlaces(page);
  const card = cardOf(page, name);
  await changeButton(card, "名前を変える").click();
  await card.getByLabel("名前", { exact: true }).fill(`変更中-${TAG}`);
  await expect(sendChange(card)).toBeEnabled();
  await sendChange(card).click();
  await expect(sendChange(card)).toBeDisabled();
  hold.open();
  await expect(card.getByTestId("place-change-form")).toHaveCount(0);
});

// Scenario: 変えるを 2 回押しても同じ原文を送る
test("広さを変えて応答が返らず押し直しても、求めの本文の原文は同じで、広さの記録は 1 件だけ増える", async ({ page, request }) => {
  const { name, id } = await placeToChange(request, "広さ二度押し");
  const { sent } = watch(page);
  const dropped = await dropFirstIngestReply(page);
  const before = await placeRecordCount(request, id, "radius");
  await openPlaces(page);
  const card = cardOf(page, name);
  await changeButton(card, "広さを変える").click();
  await card.getByRole("radio", { name: "50 m" }).check();
  await sendChange(card).click();
  await expect(page.getByTestId("place-problem")).toContainText("サーバに届きませんでした");
  expect(dropped.calls()).toBe(1);
  await sendChange(card).click();
  await expect(card.getByTestId("place-change-form")).toHaveCount(0);

  const ingests = bodiesOf(sent, "POST", "/api/ingest");
  expect(ingests).toHaveLength(2);
  expect(rawsOf(ingests[1]), "2 回の原文が一致する").toEqual(rawsOf(ingests[0]));
  expect(ingests[1]).toBe(ingests[0]);
  expect(await placeRecordCount(request, id, "radius"), "読み直した広さの記録").toBe(before + 1);
});

// Scenario: 変えるが届かなかったとき入力が残り届かなかったと出る
test("名前を変えるが届かないと、届かなかったと出て、入れた名前が残る", async ({ page, request }) => {
  const { name } = await placeToChange(request, "届かない変更");
  await page.route("**/api/ingest", (route) => route.abort("failed"));
  await openPlaces(page);
  const card = cardOf(page, name);
  await changeButton(card, "名前を変える").click();
  const typed = `打った名前-${TAG}`;
  await card.getByLabel("名前", { exact: true }).fill(typed);
  await sendChange(card).click();
  await expect(card.getByTestId("place-problem")).toContainText("サーバに届きませんでした");
  await expect(card.getByTestId("place-problem")).not.toContainText("受け付けられません");
  await expect(card.getByLabel("名前", { exact: true })).toHaveValue(typed);
});

// Scenario: 座標を変えて断られると入力が残る
test("座標を変えるが直す先が見つからないと断られたら、理由が出て、選んだ居た所と変え方が残る", async ({ page, request }) => {
  const { name, others } = await placeToChange(request, "断られる座標");
  const [pick] = others;
  await page.route("**/api/ingest", (route) => route.fulfill({ status: 400, json: refusal("invalid_coord_supersedes") }));
  await openPlaces(page);
  const card = cardOf(page, name);
  await changeButton(card, "座標を変える").click();
  const option = await showOption(card, pick);
  await option.getByRole("radio").check();
  await card.getByRole("radio", { name: "前の座標が間違っていた" }).check();
  await sendChange(card).click();
  await expect(card.getByTestId("place-problem")).toContainText("直す座標が見つかりません");
  await expect(option.getByRole("radio")).toBeChecked();
  await expect(card.getByRole("radio", { name: "前の座標が間違っていた" })).toBeChecked();
});

// Story の完了の判定（design D18）: 登録 → 名前を変える → 座標を直す の後で、識別子が登録のときと同じで、カードが 1 枚
test("登録して名前を変え座標を直しても、場所の識別子は登録のときのまま変わらず、カードは 1 枚", async ({ page, request }) => {
  const [here, moved] = await seedVisits(request, 2);
  const name = `登録-${TAG}`;
  const renamed = `改名-${TAG}`;
  const { sent } = watch(page);
  await openPlaces(page);
  const form = await fillAdd(page, here, name);
  await form.getByRole("button", { name: "登録する" }).click();
  await expect(cardOf(page, name)).toHaveCount(1);
  const registeredId = (JSON.parse(bodiesOf(sent, "POST", "/api/places")[0]) as { id: string }).id;
  expect((await placeNamed(request, name)).id).toBe(registeredId);

  const card = cardOf(page, name);
  await changeButton(card, "名前を変える").click();
  await card.getByLabel("名前", { exact: true }).fill(renamed);
  await sendChange(card).click();
  await expect(cardOf(page, renamed)).toHaveCount(1);

  const renamedCard = cardOf(page, renamed);
  await changeButton(renamedCard, "座標を変える").click();
  await (await showOption(renamedCard, moved)).getByRole("radio").check();
  await renamedCard.getByRole("radio", { name: "前の座標が間違っていた" }).check();
  await sendChange(renamedCard).click();
  await expect(renamedCard.getByTestId("place-change-form")).toHaveCount(0);

  await page.reload();
  const api = await apiPlaces(request);
  const mine = api.filter((p) => p.id === registeredId);
  expect(mine, "同じ識別子の場所は 1 つ").toHaveLength(1);
  expect(mine[0]).toMatchObject({ name: renamed, coord: { lat: moved.lat, lon: moved.lon } });
  expect(api.filter((p) => p.name === name || p.name === renamed)).toHaveLength(1);
  await expect(cardOf(page, renamed)).toHaveCount(1);
  await expect(cardOf(page, name)).toHaveCount(0);
});

// ================================================================== 10.4 下限（幅 360 CSS px）

// Scenario: 場所の読み出しの失敗と場所が無いことを区別する
test("場所の読み出しが失敗したら、失敗したことが出て、場所がまだ無いとは出ない", async ({ page }) => {
  await failWith(page, "/api/places", 500);
  await openPlaces(page);
  await expect(page.getByTestId("places-failed")).toContainText("読み出せませんでした");
  await expect(page.getByTestId("places-empty")).toHaveCount(0);
  expect(await page.evaluate(() => document.body.innerText)).not.toContain("場所がまだありません");
});

// Scenario: 名前の無い居た所の読み出しの失敗を区別する
test("名前の無い居た所の読み出しが失敗したら、失敗したことが出て、居た所がまだ無いとは出ない", async ({ page }) => {
  await stubPlaces(page, []);
  await failWith(page, "/api/places/candidates", 500);
  await openPlaces(page);
  await page.getByRole("button", { name: "場所を足す" }).click();
  await expect(page.getByTestId("candidates-failed")).toContainText("読み出せませんでした");
  await expect(page.getByTestId("candidates-empty")).toHaveCount(0);
  expect(await page.evaluate(() => document.body.innerText)).not.toContain("居た所がまだありません");
});

/** 差し替えた場所 2 つ・居た所 2 つで、画面の操作と入力を全部開いた状態にする（幅 360） */
async function openEverything(page: Page): Promise<void> {
  await page.setViewportSize({ width: 360, height: 640 });
  await stubPlaces(page, [fakePlace(1), fakePlace(2)]);
  await stubCandidates(page, [fakeCandidate(1), fakeCandidate(2)]);
  await openPlaces(page);
  const first = cards(page).first();
  await first.getByRole("button", { name: /^前の名前・座標 \d+/ }).click();
  await changeButton(first, "座標を変える").click();
  await first.getByRole("radio", { name: "この場所が移った" }).check();
  await first.getByRole("radio", { name: "年月", exact: true }).check();
  await page.getByRole("button", { name: "場所を足す" }).click();
  await page.getByTestId("place-candidate").first().getByRole("button", { name: "名前を付ける" }).click();
  await expect(page.getByTestId("place-add-form")).toBeVisible();
}

// Scenario: 場所の画面は触れる対象の下限を満たす
test("幅 360 で、タブ・カードの操作・前の名前・座標・場所を足す・名前を付ける・フォームの選択肢が幅も高さも 24 CSS px 以上", async ({ page }) => {
  await openEverything(page);
  const targets = page.locator("[data-testid=master-view] :is(button, a, input, [role=tab])");
  const small: string[] = [];
  let measured = 0;
  for (const el of await targets.all()) {
    if (!(await el.isVisible())) continue;
    const box = await el.boundingBox();
    measured += 1;
    const label = (await el.evaluate((n) => `${n.tagName.toLowerCase()} ${(n.getAttribute("aria-label") ?? n.textContent ?? "").trim().slice(0, 20)}`));
    if (box === null || box.width < 24 || box.height < 24) small.push(`${label}（${Math.round(box?.width ?? 0)}×${Math.round(box?.height ?? 0)}）`);
  }
  expect(measured, "測った対象の数").toBeGreaterThan(20);
  expect(small, "24 CSS px に満たない対象").toEqual([]);
  // 名前を付けるのフォーム・座標のフォームの選択肢は、開いた状態で測っている
  await expect(page.getByTestId("place-add-form").getByRole("radio")).toHaveCount(4);
  await expect(page.getByTestId("move-note")).toBeVisible();
});

for (const scheme of ["light", "dark"] as const) {
  // Scenario: 場所の画面は文字のコントラストの下限を満たす
  test(`${scheme} で、場所の画面の本文と補助の文字の色は背景に対して 4.5:1 以上`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openEverything(page);
    await expect(page.getByTestId("master-view")).toHaveAttribute("data-scheme", scheme);
    const painted = await paintedText(page);
    expect(painted.length, "測った文字の数").toBeGreaterThan(20);
    const low = painted
      .map((p) => ({ label: p.label, value: ratio(composite([p.fg]), composite(p.bgs)) }))
      .filter((p) => p.value < 4.5)
      .map((p) => `${p.label}: ${p.value.toFixed(2)}`);
    expect(low, "4.5:1 に満たない文字").toEqual([]);
  });

  // Scenario: 場所の画面はフォーカスの輪郭が見える
  test(`${scheme} で、Tab だけで「場所を足す」にフォーカスが移り、輪郭の色は隣の面の色に対して 3:1 以上`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.setViewportSize({ width: 360, height: 640 });
    await stubPlaces(page, [fakePlace(1)]);
    await openPlaces(page);
    let reached = false;
    for (let i = 0; i < 30 && !reached; i++) {
      await page.keyboard.press("Tab");
      reached = await page.evaluate(() => (document.activeElement as HTMLElement | null)?.textContent?.trim() === "場所を足す");
    }
    expect(reached, "Tab だけで「場所を足す」に届かなかった").toBe(true);
    const focused = page.getByRole("button", { name: "場所を足す" });
    await expect(focused).toBeFocused();
    const ring = await focused.evaluate((el) => {
      const style = getComputedStyle(el);
      const bgs: string[] = [];
      for (let n: Element | null = el.parentElement; n !== null; n = n.parentElement) bgs.push(getComputedStyle(n).backgroundColor);
      return { width: Number.parseFloat(style.outlineWidth), style: style.outlineStyle, color: style.outlineColor, bgs };
    });
    expect(ring.style, "輪郭が出ていない").not.toBe("none");
    expect(ring.width, "輪郭の太さ").toBeGreaterThan(0);
    const value = ratio(composite([ring.color]), composite(ring.bgs));
    expect(value, `輪郭 ${ring.color} と隣の面のコントラスト比 ${value.toFixed(2)}`).toBeGreaterThanOrEqual(3);
  });
}

// Scenario: 場所の画面は OS の明暗に追従する
test("OS の明暗を明にすると明の面で、設定が取得できないときは暗の面で描かれる", async ({ page }) => {
  await stubPlaces(page, [fakePlace(1)]);
  const surface = async (): Promise<number> => {
    const bgs = await page.getByTestId("place-card").first().evaluate((el) => {
      const out: string[] = [];
      for (let n: Element | null = el; n !== null; n = n.parentElement) out.push(getComputedStyle(n).backgroundColor);
      return out;
    });
    return luminance(composite(bgs));
  };

  await page.emulateMedia({ colorScheme: "light" });
  await openPlaces(page);
  await expect(page.getByTestId("master-view")).toHaveAttribute("data-scheme", "light");
  const light = await surface();
  expect(light, "明の面の輝度").toBeGreaterThan(0.5);

  // 設定が取得できない状態: `matchMedia` が無い。Chromium の `no-preference` は既定の明に解決されるので、**取得できない側は
  // `matchMedia` を消して**作る（画面は `typeof window.matchMedia !== "function"` のときダークにする。NFR-17）
  await page.addInitScript(() => {
    Object.defineProperty(window, "matchMedia", { value: undefined, configurable: true });
  });
  await openPlaces(page);
  await expect(page.getByTestId("master-view")).toHaveAttribute("data-scheme", "dark");
  const dark = await surface();
  expect(dark, "暗の面の輝度").toBeLessThan(0.1);
});
