// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

/**
 * 設定画面の「全記録の書き出し」を本物のブラウザで確かめる（ST33 / Task 9）。
 * アサートするのは数値と経路（実寸・可視・フォーカス・URL・scrollWidth）。スクリーンショット比較は使わない。
 *
 * 書き出しの台帳は追記のみで、全テストが同じ DB を見る。台帳の「空・失敗・途中で止まった・消せなかった」は
 * 共有の stack では作れないので、9.3 は `page.route` で `GET /api/export` を差し替える（サーバの側は Rust のテストが固定する）。
 */

/** テスト専用の合言葉（12 文字以上）。 */
const PW = "e2e-passphrase-0123";
const DAY = "2026-09-07";
const STORAGE_STATE = "./e2e/.auth/state.json";

type Last = { rows: number; bytes: number; file: string; erased_after: number | null };

/** 画面の大きさの書き方（`web/src/export.ts` の `sizeLabel` と同じ 1000 刻み）。 */
function sizeText(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(bytes >= 1e7 ? 0 : 1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1e3))} KB`;
}
type StatusBody = { running?: unknown; last?: Last | null; leftover: string[] };

/**
 * 読み出しは本物のブラウザの fetch でやる。ログインの印は `secure` で、Node 側の `request` は http の stack へ付けない
 * （`login.spec.ts` と同じ事情）。
 */
let ctx: BrowserContext;
let probe: Page;

async function call(method: "GET" | "POST", path: string, body?: unknown): Promise<{ ok: boolean; status: number; json: unknown }> {
  return probe.evaluate(
    async ({ method, path, body }) => {
      const res = await fetch(path, {
        method,
        headers: body === undefined ? {} : { "content-type": "application/json" },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
      const json: unknown = await res.json().catch(() => null);
      return { ok: res.ok, status: res.status, json };
    },
    { method, path, body },
  );
}

async function status(): Promise<StatusBody> {
  const res = await call("GET", "/api/export");
  expect(res.ok, `GET /api/export は ${res.status}`).toBe(true);
  return res.json as StatusBody;
}

/** `running` が消えるまで待つ（後続のテストが 409 に当たらないように）。 */
async function settle(): Promise<void> {
  await expect
    .poll(async () => {
      const s = await status();
      return s.running === undefined || s.running === null;
    }, { timeout: 90_000, intervals: [250] })
    .toBe(true);
}

// 書き出しは表ごとに 400 ms 待つ。終わるのを待つ分、既定の 30 秒では足りない
test.describe.configure({ timeout: 180_000 });

test.beforeAll(async ({ browser, baseURL }) => {
  test.setTimeout(180_000);
  ctx = await browser.newContext({ baseURL, storageState: STORAGE_STATE });
  probe = await ctx.newPage();
  await probe.goto("/");
  await settle();
  const res = await call("POST", "/api/export", { password: PW });
  expect(res.ok, `POST /api/export は ${res.status}`).toBe(true);
  // running は POST の中で写しより前に置かれるので、見えただけでは遅延の証拠にならない。
  // 表ごとの 400 ms は書き込みの段にだけ掛かる —— 行の総数が出てから running が消えるまでの時間で確かめる
  const started = Date.now();
  let sawTotal = false;
  for (;;) {
    const s = await status();
    const running = s.running as { total?: number } | undefined | null;
    if (running === undefined || running === null) break;
    if ((running.total ?? 0) > 0) sawTotal = true;
    if (Date.now() - started > 90_000) throw new Error("書き出しが 90 秒で終わらない");
    await probe.waitForTimeout(100);
  }
  const elapsed = Date.now() - started;
  if (!sawTotal || elapsed < 800) {
    throw new Error(
      `書き出しの遅延（ASHIATO_EXPORT_SELFTEST_DELAY_MS=400）が効いていない（running は ${elapsed} ms、行の総数を見た=${sawTotal}）。` +
        "手元の run.sh の stack を使い回している可能性がある —— run.sh を止めるか CI=1 で走らせる（spec-review R19）",
    );
  }
});

test.afterAll(async () => {
  await ctx.close();
});

// 書き出しを始めたテストの後始末（次のテストが 409 に当たらないように）
test.afterEach(async () => {
  await settle();
});

async function openSettings(page: Page): Promise<void> {
  await page.goto("/#/settings");
  await page.reload();
  await expect(page.getByTestId("settings-view")).toBeVisible();
  await expect(page.getByTestId("export-sources")).toBeVisible({ timeout: 90_000 });
}

function watchPosts(page: Page): string[] {
  const bodies: string[] = [];
  page.on("request", (req) => {
    if (req.method() === "POST" && new URL(req.url()).pathname === "/api/export") bodies.push(req.postData() ?? "");
  });
  return bodies;
}

const startButton = (page: Page) => page.getByTestId("export-start");
const goButton = (page: Page) => page.getByRole("button", { name: "書き出す", exact: true });
const pwInputs = (page: Page) => page.locator("input[type=password]");

async function openConfirm(page: Page): Promise<void> {
  await startButton(page).click();
  await expect(page.getByTestId("export-confirm")).toBeVisible();
}

async function fillBoth(page: Page, a: string, b: string): Promise<void> {
  await pwInputs(page).nth(0).fill(a);
  await pwInputs(page).nth(1).fill(b);
}

async function startExport(page: Page): Promise<void> {
  await openConfirm(page);
  await fillBoth(page, PW, PW);
  await goButton(page).click();
}

// ---------------------------------------------------------------- 9.2

// Scenario: 設定画面に全記録の書き出しの節がある
test("Scenario 9.2: 稼働状況のリンクから設定画面が開き、最初の節は「全記録の書き出し」", async ({ page }) => {
  await page.goto("/#/coverage");
  await page.reload();
  await page.getByTestId("to-settings").click();
  await expect(page).toHaveURL(/#\/settings$/);
  await expect(page.getByTestId("settings-view").locator("h2").first()).toHaveText("全記録の書き出し");
});

// Scenario: 押す前にソースごとの件数と期間が出る
test("Scenario 9.2: ソースごとの件数と期間が GET /api/export/summary と一致する", async ({ page }) => {
  await openSettings(page);
  const res = await call("GET", "/api/export/summary");
  expect(res.ok).toBe(true);
  const body = res.json as { sources: { display_name: string; count: number }[] };
  expect(body.sources.length).toBeGreaterThan(0);
  const rows = page.getByTestId("export-sources").locator("li");
  await expect(rows).toHaveCount(body.sources.length);
  for (const [i, s] of body.sources.entries()) {
    await expect(rows.nth(i)).toContainText(`${s.display_name}: ${s.count.toLocaleString("ja-JP")} 件`);
    await expect(rows.nth(i)).toContainText(/\d{4}\/\d+\/\d+ 〜 \d{4}\/\d+\/\d+/);
  }
});

// Scenario: 押す前に入らないものの一覧が出る
test("Scenario 9.2: 入らないものの 3 行があり、件数が付いていない", async ({ page }) => {
  await openSettings(page);
  await expect(page.getByText("入らないもの", { exact: true })).toBeVisible();
  const items = page.getByTestId("export-excluded").locator("li");
  await expect(items).toHaveCount(3);
  await expect(items.nth(0)).toContainText("本人が消した場面");
  await expect(items.nth(1)).toContainText("削除と消去の台帳");
  await expect(items.nth(2)).toContainText("写真・動画の原本");
  for (const t of await items.allInnerTexts()) {
    // 数字を持ち込まない（原本の呼び名「D-02」だけは名前の一部）
    expect(t.replace("D-02", ""), `件数が付いている: ${t}`).not.toMatch(/\d/);
  }
});

// Scenario: 押すとその場に確認が出て、合言葉を入れて書き出すを選ぶと進み具合が出る
test("Scenario 9.2: 押すと確認が出て、書き出すで求めが 1 回だけ送られ進み具合が出る", async ({ page }) => {
  const posts = watchPosts(page);
  await openSettings(page);
  expect((await status()).last, "前回の書き出しがある状態で確かめる").toBeTruthy();
  await openConfirm(page);
  expect(posts, "確認の時点で POST が送られている").toHaveLength(0);
  const confirm = page.getByTestId("export-confirm");
  await expect(confirm).toContainText(/約 [\d,]+ 行・約 .+・約 [\d,]+ 秒/);
  await expect(confirm).toContainText("この PC のフォルダ");
  await expect(confirm).toContainText("暗号化した 1 つのファイル");
  await expect(confirm).toContainText("合言葉は保存しません");
  await expect(confirm).toContainText("AES");
  await expect(confirm).toContainText("前回の書き出しのファイルは消えます");

  await fillBoth(page, PW, PW);
  await goButton(page).click();
  const progress = page.getByTestId("export-progress");
  await expect(progress).toBeVisible({ timeout: 60_000 });
  await expect(progress).toContainText(/書き出し中です（[\d,]+ \/ [\d,]+ 行）/);
  await expect(progress).toContainText("画面を離れても続きます");
  expect(posts).toHaveLength(1);
  expect((JSON.parse(posts[0]) as { password: string }).password).toBe(PW);
});

// Scenario: 合言葉が短いか確かめと一致しなければ書き出すを押せない
test("Scenario 9.2: 11 文字、または 12 文字で不一致なら書き出すを押せず、求めは送られない", async ({ page }) => {
  const posts = watchPosts(page);
  await openSettings(page);
  await openConfirm(page);
  const reason = page.getByTestId("export-reason");

  await fillBoth(page, "a".repeat(11), "a".repeat(11));
  await expect(goButton(page)).toBeDisabled();
  await expect(reason).toBeVisible();
  await expect(reason).toContainText("12 文字以上");

  await fillBoth(page, "a".repeat(12), "b".repeat(12));
  await expect(goButton(page)).toBeDisabled();
  await expect(reason).toContainText("一致していません");
  expect(posts).toHaveLength(0);
});

// Scenario: 書き出しを始めると合言葉は画面に残らない
test("Scenario 9.2: 書き出すの後、入力・content・URL・保存領域に合言葉が無い", async ({ page }) => {
  await openSettings(page);
  await startExport(page);
  await expect(page.getByTestId("export-progress")).toBeVisible({ timeout: 60_000 });
  await expect(pwInputs(page)).toHaveCount(0);
  expect(await page.content()).not.toContain(PW);
  expect(page.url()).not.toContain(PW);
  const stored = await page.evaluate(() => {
    const all: string[] = [];
    for (const s of [localStorage, sessionStorage]) {
      for (let i = 0; i < s.length; i++) {
        const k = s.key(i) ?? "";
        all.push(k, s.getItem(k) ?? "");
      }
    }
    return all;
  });
  for (const v of stored) expect(v).not.toContain(PW);
});

// Scenario: 合言葉の入力はブラウザに保存を促さない
test("Scenario 9.2: 入力の autocomplete は new-password でも current-password でもなく、URL は変わらない", async ({ page }) => {
  await openSettings(page);
  await openConfirm(page);
  for (const i of [0, 1]) {
    const ac = await pwInputs(page).nth(i).getAttribute("autocomplete");
    expect(ac).not.toBe("new-password");
    expect(ac).not.toBe("current-password");
  }
  const before = page.url();
  await fillBoth(page, PW, PW);
  await goButton(page).click();
  await expect(page.getByTestId("export-progress")).toBeVisible({ timeout: 60_000 });
  expect(page.url()).toBe(before);
});

// Scenario: 確認でやめると書き出しは始まらない
test("Scenario 9.2: 確認でやめると求めは送られず、押す前の形に戻る", async ({ page }) => {
  const posts = watchPosts(page);
  await openSettings(page);
  await openConfirm(page);
  await fillBoth(page, PW, PW);
  await page.getByRole("button", { name: "やめる" }).click();
  await expect(page.getByTestId("export-confirm")).toHaveCount(0);
  await expect(pwInputs(page)).toHaveCount(0);
  await expect(startButton(page)).toBeEnabled();
  expect(posts).toHaveLength(0);
});

// Scenario: 画面を離れて戻っても進み具合が出る
test("Scenario 9.2: 別の画面へ移って戻っても進み具合が出る", async ({ page }) => {
  await openSettings(page);
  await startExport(page);
  await expect(page.getByTestId("export-progress")).toBeVisible({ timeout: 60_000 });
  await page.evaluate((d) => {
    location.hash = `#/day/${d}`;
  }, DAY);
  await expect(page.getByTestId("day-view")).toBeVisible();
  await page.evaluate(() => {
    location.hash = "#/settings";
  });
  await expect(page.getByTestId("settings-view")).toBeVisible();
  await expect(page.getByTestId("export-progress")).toBeVisible({ timeout: 60_000 });
});

// Scenario: 書き終えると前回の書き出しが新しくなる
test("Scenario 9.2: 書き終えると前回の書き出しの行の数が GET /api/export と一致する", async ({ page }) => {
  const before = (await status()).last;
  await openSettings(page);
  await startExport(page);
  await expect(page.getByTestId("export-progress")).toBeVisible({ timeout: 60_000 });
  await expect(page.getByTestId("export-progress")).toBeHidden({ timeout: 90_000 });
  const last = (await status()).last;
  expect(last).toBeTruthy();
  // 前回のままではなく新しくなっている
  expect((last as Last).file, "前回の書き出しが新しくなっていない").not.toBe(before?.file);
  const box = page.getByTestId("export-last");
  await expect(box).toContainText(`${(last as Last).rows.toLocaleString("ja-JP")} 行`);
  await expect(box).toContainText(sizeText((last as Last).bytes));
  await expect(box).toContainText((last as Last).file);
  await expect(box).toContainText(/\d{4}\/\d+\/\d+ \d{2}:\d{2}/);
});

// ---------------------------------------------------------------- 9.3

const STARTED = "2026-10-13T00:00:00Z";

/** `GET /api/export` だけを差し替える（`/api/export/summary` と `POST` は本物）。 */
async function stubStatus(page: Page, body: Record<string, unknown>): Promise<void> {
  await page.route("**/api/export", async (route) => {
    if (route.request().method() !== "GET") return route.continue();
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
}

// Scenario: まだ書き出していなければそう出る
test("Scenario 9.3: まだ書き出していなければそう出て、確認に前回のファイルが消える文が無い", async ({ page }) => {
  await stubStatus(page, { last: null, leftover: [] });
  await openSettings(page);
  await expect(page.getByTestId("export-last")).toContainText("まだ書き出していません。");
  await openConfirm(page);
  await expect(page.getByTestId("export-confirm")).not.toContainText("前回の書き出しのファイルは消えます");
});

// Scenario: 失敗した書き出しと途中で止まった書き出しは節に出て、押し直せる
test("Scenario 9.3: 失敗と途中で止まった書き出しは節に出て、押し直せる", async ({ page }) => {
  await stubStatus(page, { last_failure: { at: STARTED, kind: "disk_full" }, leftover: [] });
  await openSettings(page);
  await expect(page.getByTestId("export-section").getByTestId("export-line")).toContainText("書き出しに失敗しました");
  await expect(startButton(page)).toBeEnabled();

  await page.unroute("**/api/export");
  await stubStatus(page, { interrupted: { started_at: STARTED, file: "/data/export/ashiato-partial.zip" }, leftover: [] });
  await openSettings(page);
  await expect(page.getByTestId("export-section").getByTestId("export-line")).toContainText("途中で止まりました");
  await expect(page.getByTestId("export-section").getByTestId("export-line")).toContainText("/data/export/ashiato-partial.zip");
  await expect(startButton(page)).toBeEnabled();
});

// Scenario: 前の書き出しのファイルを消せなかったら節に出る
test("Scenario 9.3: 消せなかったファイルのパスと、次の書き出しで消すことが出る", async ({ page }) => {
  await stubStatus(page, { leftover: ["/data/export/ashiato-old.zip"] });
  await openSettings(page);
  const line = page.getByTestId("export-section").getByTestId("export-line");
  await expect(line).toContainText("消せませんでした");
  await expect(line).toContainText("/data/export/ashiato-old.zip");
  await expect(line).toContainText("次の書き出しでもう一度消します");
});

// Scenario: 書き出しが走っている間は全記録を書き出すを押せない
test("Scenario 9.3: 書き出しが走っている間は進み具合が出て、全記録を書き出すは押せない", async ({ page }) => {
  await stubStatus(page, { running: { export_id: "e2e", started_at: STARTED, written: 5, total: 10 }, leftover: [] });
  await openSettings(page);
  await expect(page.getByTestId("export-progress")).toBeVisible();
  await expect(page.getByTestId("export-progress")).toContainText("5 / 10 行");
  await expect(startButton(page)).toBeDisabled();
});

// Scenario: 書き出しの求めが断られたらそのことが出て押す前の形に戻る
test("Scenario 9.3: 求めが 409 で断られたら 1 行出て、合言葉の入力は無く、押し直せる", async ({ page }) => {
  await page.route("**/api/export", async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    return route.fulfill({ status: 409, contentType: "application/json", body: JSON.stringify({ error: "export_running" }) });
  });
  await openSettings(page);
  await startExport(page);
  await expect(page.getByTestId("export-refusal")).toBeVisible();
  await expect(page.getByTestId("export-refusal")).toContainText("すでに書き出し中です");
  await expect(pwInputs(page)).toHaveCount(0);
  await expect(startButton(page)).toBeEnabled();
});

// ---------------------------------------------------------------- 9.4

// Scenario: 書き出した後に記録を消すと、前回の書き出しの欄に件数が出る
test("Scenario 9.4: 書き出した後に滞在を 1 件消すと、前回の書き出しの欄に件数が出る", async ({ page }) => {
  // 合言葉を入れて書き終える（画面から）
  await openSettings(page);
  await startExport(page);
  await expect(page.getByTestId("export-progress")).toBeVisible({ timeout: 60_000 });
  await expect(page.getByTestId("export-progress")).toBeHidden({ timeout: 90_000 });

  await page.goto(`/#/day/${DAY}`);
  await page.reload();
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  const row = page.getByTestId("row-stay").first();
  await expect(row).toBeVisible();
  const range = (await row.locator("h2").innerText()).trim();
  await row.getByRole("button").click();
  await row.getByRole("button", { name: "この滞在を消す" }).click();
  await row.getByRole("button", { name: "消す" }).click();
  const erased = page.getByTestId("row-erased").filter({ hasText: range });
  let restored = false;
  try {
    await expect(erased).toHaveCount(1);
    await openSettings(page);
    const note = page.getByTestId("export-erased-after");
    await expect(note).toBeVisible();
    const text = (await note.innerText()).trim();
    const m = /^この書き出しの後に消した記録が ([\d,]+) 件あります。書き出したファイルの中には残っています。$/.exec(text);
    expect(m, `文言が違う: ${text}`).not.toBeNull();
    const n = Number((m as RegExpExecArray)[1].replace(/,/g, ""));
    const last = (await status()).last as Last;
    expect(n).toBe(last.erased_after);
    expect(n).toBeGreaterThanOrEqual(1);
  } finally {
    // 後続のテストが同じ DB を見る。消した滞在を戻す
    await page.goto(`/#/day/${DAY}`);
    await page.reload();
    await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
    const back = page.getByTestId("row-erased").filter({ hasText: range }).getByRole("button", { name: "戻す" });
    if (!restored && (await back.count()) > 0) {
      await back.click();
      restored = true;
      await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(1);
    }
  }
});

// ---------------------------------------------------------------- 9.5

test.describe("360 px", () => {
  test.use({ viewport: { width: 360, height: 800 } });

  // Scenario: 書き出すの操作は 24 px を下回らない
  test("Scenario 9.5: 書き出すの操作は幅も高さも 24 px 以上", async ({ page }) => {
    await openSettings(page);
    const start = await startButton(page).boundingBox();
    expect(start?.width ?? 0, "全記録を書き出すの幅").toBeGreaterThanOrEqual(24);
    expect(start?.height ?? 0, "全記録を書き出すの高さ").toBeGreaterThanOrEqual(24);
    await openConfirm(page);
    const targets = [
      ["合言葉", pwInputs(page).nth(0)],
      ["もう一度", pwInputs(page).nth(1)],
      ["書き出す", goButton(page)],
      ["やめる", page.getByRole("button", { name: "やめる" })],
    ] as const;
    for (const [name, loc] of targets) {
      const box = await loc.boundingBox();
      expect(box?.width ?? 0, `${name} の幅`).toBeGreaterThanOrEqual(24);
      expect(box?.height ?? 0, `${name} の高さ`).toBeGreaterThanOrEqual(24);
    }
  });

  // Scenario: キーボードで書き出すを押せてフォーカスの位置が見える
  test("Scenario 9.5: Tab と Enter だけで書き出せて、フォーカスの輪郭が見える", async ({ page }) => {
    await openSettings(page);
    const outlined = (): Promise<boolean> =>
      page.evaluate(() => {
        const el = document.activeElement;
        if (el === null) return false;
        const s = getComputedStyle(el);
        return s.outlineStyle !== "none" && parseFloat(s.outlineWidth) > 0;
      });
    const onStart = (): Promise<boolean> =>
      page.evaluate(() => document.activeElement?.getAttribute("data-testid") === "export-start");
    for (let i = 0; i < 20 && !(await onStart()); i++) await page.keyboard.press("Tab");
    expect(await onStart(), "Tab で全記録を書き出すへ移れない").toBe(true);
    expect(await outlined(), "全記録を書き出すに輪郭が無い").toBe(true);

    await page.keyboard.press("Enter");
    await expect(page.getByTestId("export-confirm")).toBeVisible();
    await expect
      .poll(() => page.evaluate(() => document.activeElement === document.querySelectorAll("input[type=password]")[0]))
      .toBe(true);
    expect(await outlined(), "合言葉の入力に輪郭が無い").toBe(true);

    await page.keyboard.type(PW);
    await page.keyboard.press("Tab");
    expect(
      await page.evaluate(() => document.activeElement === document.querySelectorAll("input[type=password]")[1]),
      "Tab で確かめの入力へ移れない",
    ).toBe(true);
    expect(await outlined(), "確かめの入力に輪郭が無い").toBe(true);
    await page.keyboard.type(PW);
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("export-progress")).toBeVisible({ timeout: 60_000 });
  });

  // Scenario: 設定画面は 360 px の幅で横にスクロールしない
  test("Scenario 9.5: ファイルのパスが出ていて確認を開いても横にスクロールしない", async ({ page }) => {
    await openSettings(page);
    const last = (await status()).last as Last;
    await expect(page.getByTestId("export-last")).toContainText(last.file);
    const fits = (): Promise<boolean> =>
      page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth);
    expect(await fits(), "確認を開く前").toBe(true);
    await openConfirm(page);
    expect(await fits(), "確認を開いた後").toBe(true);
  });
});

// Scenario: 画面から書き出したファイルを受け取る操作は無い
test("Scenario 9.5: 書き終えた後の操作できる要素にダウンロード・保存・file: が無い", async ({ page }) => {
  await openSettings(page);
  await startExport(page);
  await expect(page.getByTestId("export-progress")).toBeVisible({ timeout: 60_000 });
  await expect(page.getByTestId("export-progress")).toBeHidden({ timeout: 90_000 });
  await expect(page.getByTestId("export-last")).toContainText("ファイル:");
  const ops = await page.locator("a, button, [download]").evaluateAll((els) =>
    els.map((el) => ({
      text: (el.textContent ?? "").trim(),
      download: el.hasAttribute("download"),
      href: el.getAttribute("href") ?? "",
    })),
  );
  expect(ops.length).toBeGreaterThan(0);
  for (const o of ops) {
    expect(o.download, `download 属性: ${o.text}`).toBe(false);
    expect(o.href, `file: のリンク: ${o.text}`).not.toMatch(/^file:/i);
    expect(o.href, `ファイルへのリンク: ${o.text}`).not.toMatch(/\.zip$/i);
    expect(o.text, `受け取る操作: ${o.text}`).not.toMatch(/ダウンロード|保存|download/i);
  }
});

// Scenario: 設定画面の文字はライトでもダークでも 4.5:1 を下回らない
for (const scheme of ["light", "dark"] as const) {
  test(`Scenario 9.5: ${scheme} で設定画面の文字は 4.5:1 以上`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openSettings(page);
    await expect(page.getByTestId("export-last")).toContainText("ファイル:");
    const measured = await page.getByTestId("settings-view").evaluate((root) => {
      const parse = (c: string): number[] => c.match(/[\d.]+/g)?.map(Number) ?? [0, 0, 0, 0];
      const lin = (v: number): number => {
        const s = v / 255;
        return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
      };
      const lum = (c: number[]): number => 0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2]);
      const backOf = (el: Element | null): number[] => {
        for (let e = el; e !== null; e = e.parentElement) {
          const bg = parse(getComputedStyle(e).backgroundColor);
          if ((bg.length > 3 ? bg[3] : 1) > 0) return bg;
        }
        return [255, 255, 255, 1];
      };
      const out: { what: string; ratio: number }[] = [];
      for (const el of [root, ...root.querySelectorAll("*")]) {
        const own = [...el.childNodes].some((n) => n.nodeType === Node.TEXT_NODE && (n.textContent ?? "").trim() !== "");
        if (!own) continue;
        const r = el.getBoundingClientRect();
        if (r.width === 0 || r.height === 0) continue;
        const [x, y] = [lum(parse(getComputedStyle(el).color)), lum(backOf(el))];
        out.push({
          what: `<${el.tagName.toLowerCase()}> ${(el.textContent ?? "").trim().slice(0, 24)}`,
          ratio: (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05),
        });
      }
      return out;
    });
    expect(measured.length).toBeGreaterThan(5);
    for (const t of measured) expect(t.ratio, `${scheme}: ${t.what}`).toBeGreaterThanOrEqual(4.5);
  });
}
