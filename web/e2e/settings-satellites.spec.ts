// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";

/**
 * 設定画面の「衛星と資格情報」の節を本物のブラウザで確かめる（ST29 / Task 9 / design D12）。
 * アサートするのは数値と経路（実寸・可視・入力欄の下の文言・行の有無・値の有無）。スクリーンショット比較は使わない。
 *
 * 全テストが同じ DB を見る。登録簿の行は消えない（外すのは印を付けるだけ）ので、各テストの前に
 * 外していない自前の衛星を全部外し、最初の 2 つの資格情報を取り消してから、決まった形に積み直す。
 * 外した識別子で登録し直すと同じ行が戻る（200）ので、同じ識別子・名前を毎回使える。
 */

const STORAGE_STATE = "./e2e/.auth/state.json";
const PREFIX = "ashsat_dev.example.booklog_";

type Cred = { id: string; label: string };
type Sat = { id: string; identifier: string; name: string; builtin: boolean; removed_at: string | null; credentials: Cred[] };
type Seed = { booklog: string[]; sleep: string[] };
/** 積んだ資格情報の値。`booklog` の名前 → 値。 */
let values: Record<string, string> = {};

/**
 * 読み出しは本物のブラウザの fetch でやる。ログインの印は `secure` で、Node 側の `request` は http の stack へ付けない
 * （`login.spec.ts` と同じ事情）。
 */
let ctx: BrowserContext;
let probe: Page;

async function call(method: "GET" | "POST" | "DELETE", path: string, body?: unknown): Promise<{ ok: boolean; status: number; json: unknown }> {
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

async function must(method: "GET" | "POST" | "DELETE", path: string, body?: unknown): Promise<unknown> {
  const res = await call(method, path, body);
  expect(res.ok, `${method} ${path} は ${res.status}`).toBe(true);
  return res.json;
}

/** 資格情報の値を付けて記録の読み出しを呼ぶ（ログインの印は付けない。Node 側の `request` は印を送らない）。 */
async function readWith(page: Page, value: string): Promise<number> {
  const res = await page.request.get("/api/events", { headers: { authorization: `Bearer ${value}` } });
  return res.status();
}

async function register(name: string, identifier: string): Promise<Sat> {
  return (await must("POST", "/api/satellites", { name, identifier })) as Sat;
}

async function issue(sat: Sat, label: string): Promise<string> {
  const out = (await must("POST", `/api/satellites/${sat.id}/credentials`, { label })) as { value: string };
  return out.value;
}

/** 前の試験の残りを片付けて、衛星 5 つ（最初の 2 つ + 3 つ）・資格情報 0〜2 本ずつに積む。 */
async function seed(spec: Seed = { booklog: ["携帯", "ノート PC"], sleep: ["携帯"] }): Promise<void> {
  const before = (await must("GET", "/api/satellites")) as Sat[];
  for (const s of before.filter((x) => x.removed_at === null)) {
    if (s.builtin) {
      for (const c of s.credentials) await must("DELETE", `/api/satellites/${s.id}/credentials/${c.id}`);
    } else {
      await must("DELETE", `/api/satellites/${s.id}`);
    }
  }
  const builtins = before.filter((x) => x.builtin);
  const external = builtins.find((x) => x.identifier === "ashiato.ai-external");
  expect(external, "最初の 2 つがある").toBeDefined();
  await issue(external!, "Claude Desktop");
  const booklog = await register("読書ログ", "dev.example.booklog");
  const sleep = await register("睡眠", "dev.example.sleep");
  await register("散歩", "dev.example.walk");
  values = {};
  for (const label of spec.booklog) values[label] = await issue(booklog, label);
  for (const label of spec.sleep) await issue(sleep, label);
}

test.beforeAll(async ({ browser, baseURL }) => {
  ctx = await browser.newContext({ baseURL, storageState: STORAGE_STATE });
  probe = await ctx.newPage();
  await probe.goto("/");
});

test.afterAll(async () => {
  await ctx.close();
});

async function openSettings(page: Page): Promise<void> {
  await page.goto("/#/settings");
  await page.reload();
  await expect(page.getByTestId("settings-view")).toBeVisible();
  await expect(page.getByTestId("satellite")).toHaveCount(5);
}

const card = (page: Page, identifier: string): Locator => page.locator(`[data-testid=satellite][data-identifier="${identifier}"]`);
const BOOKLOG = "dev.example.booklog";

// ---------------------------------------------------------------- 節が出る

// Scenario: 設定画面に衛星と資格情報の節が出る
test("Scenario 9.2: 節の頭に今は読み書きできない旨があり、最初の 2 つに「最初から」があって「外す」が無い", async ({ page }) => {
  await seed();
  await openSettings(page);
  const section = page.getByTestId("satellites-section");
  await expect(section.getByRole("heading", { level: 2, name: "衛星と資格情報" })).toBeVisible();
  await expect(page.getByTestId("satellites-lead")).toContainText("記録を読むことも書くこともできません");
  for (const [identifier, name] of [
    ["ashiato.ai-external", "外部 AI"],
    ["ashiato.ai-local", "ローカル AI"],
  ]) {
    const c = card(page, identifier);
    await expect(c).toContainText(name);
    await expect(c.getByTestId("satellite-builtin")).toHaveText("最初から");
    await expect(c.getByRole("button", { name: "外す" })).toHaveCount(0);
  }
  await expect(card(page, BOOKLOG).getByTestId("satellite-builtin")).toHaveCount(0);
  await expect(card(page, BOOKLOG).getByRole("button", { name: "外す" })).toBeVisible();
  // 書き出しの節の前に置く
  const [a, b] = await Promise.all([section.boundingBox(), page.getByTestId("export-section").boundingBox()]);
  expect(a!.y, "衛星の節は書き出しの節より上").toBeLessThan(b!.y);
});

// ---------------------------------------------------------------- 発行

// Scenario: 画面から資格情報を発行すると値がその場に 1 回だけ出る
test("Scenario 9.2: 発行すると値が小節の中に 1 回だけ出て、閉じると消え、開き直しても出ない", async ({ page }) => {
  await seed({ booklog: ["ノート PC"], sleep: ["携帯"] });
  await openSettings(page);
  const booklog = card(page, BOOKLOG);
  await booklog.getByRole("button", { name: "資格情報を発行" }).click();
  const form = booklog.getByTestId("satellite-issue-form");
  await expect(form.getByRole("heading")).toContainText("読書ログ");
  await form.getByRole("textbox").fill("携帯");
  await form.getByRole("button", { name: "発行する" }).click();

  const value = booklog.getByTestId("satellite-credential-value");
  await expect(value).toBeVisible();
  expect((await value.innerText()).startsWith(PREFIX), "値の頭").toBe(true);
  await expect(booklog.getByTestId("satellite-reveal")).toContainText("閉じると二度と表示できません");
  await expect(page.getByText(PREFIX)).toHaveCount(1);

  await booklog.getByRole("button", { name: "設定したので閉じる" }).click();
  await expect(page.getByText(PREFIX)).toHaveCount(0);
  await expect(booklog.getByTestId("satellite-reveal")).toHaveCount(0);

  await page.reload();
  await expect(page.getByTestId("satellite")).toHaveCount(5);
  expect(await page.content(), "開き直した画面に値が無い").not.toContain("ashsat_");
  expect(
    await page.evaluate(() => localStorage.length + sessionStorage.length),
    "保存領域に何も書かれていない",
  ).toBe(0);
  const row = card(page, BOOKLOG).getByTestId("credential").filter({ hasText: "携帯" });
  await expect(row).toHaveCount(1);
  await expect(row).toContainText("まだ使われていない");
});

// ---------------------------------------------------------------- 取り消す

// Scenario: 画面から資格情報を取り消すと行が消える
test("Scenario 9.2: 取り消すと行が消え、その値の読み出しは 401 になる", async ({ page }) => {
  await seed();
  await openSettings(page);
  const mobile = values["携帯"]!;
  expect(await readWith(page, mobile), "取り消す前は断られるが 401 ではない（承認がまだ）").toBe(403);
  const booklog = card(page, BOOKLOG);
  const row = booklog.getByTestId("credential").filter({ hasText: "携帯" });
  await expect(row).toHaveCount(1);
  await row.getByRole("button", { name: "取り消す" }).click();
  await expect(row).toHaveCount(0);
  await expect(booklog.getByTestId("credential")).toHaveCount(1);
  await expect(booklog.getByTestId("credential")).toContainText("ノート PC");
  expect(await readWith(page, mobile), "取り消した値の読み出し").toBe(401);
});

// ---------------------------------------------------------------- 外す

// Scenario: 画面から衛星を外すと確認をはさんでから外れる
test("Scenario 9.2: 外すを押すと確認が並び、確認の外すで小節が消えて外した衛星の並びに出る", async ({ page }) => {
  await seed();
  await openSettings(page);
  const values2 = Object.values(values);
  expect(values2.length).toBe(2);
  const booklog = card(page, BOOKLOG);
  await booklog.getByRole("button", { name: "外す" }).click();
  const confirm = booklog.getByTestId("satellite-remove-confirm");
  await expect(confirm).toBeVisible();
  await expect(confirm).toContainText("資格情報 2 本が今すぐ使えなくなります");
  await expect(confirm).toContainText("この衛星が書いた記録は消えずに残ります");
  await expect(confirm).toContainText("同じ識別子（dev.example.booklog）で登録し直すと、この衛星の続きになります");
  // まだ外れていない
  expect(await readWith(page, values2[0]!), "確認の前").toBe(403);

  await confirm.getByRole("button", { name: "外す" }).click();
  await expect(card(page, BOOKLOG)).toHaveCount(0);
  await expect(page.getByTestId("satellite")).toHaveCount(4);
  const gone = page.getByTestId("satellites-gone");
  await expect(gone).toContainText("読書ログ");
  await expect(gone).toContainText("dev.example.booklog");
  for (const v of values2) expect(await readWith(page, v), "外した衛星の値の読み出し").toBe(401);
});

// Scenario: 確認で「やめる」を押すと衛星は外れない
test("Scenario 9.2: やめるを押すと確認が閉じ、小節と資格情報は残る", async ({ page }) => {
  await seed();
  await openSettings(page);
  const booklog = card(page, BOOKLOG);
  await booklog.getByRole("button", { name: "外す" }).click();
  await expect(booklog.getByTestId("satellite-remove-confirm")).toBeVisible();
  await booklog.getByRole("button", { name: "やめる" }).click();
  await expect(booklog.getByTestId("satellite-remove-confirm")).toHaveCount(0);
  await expect(booklog).toBeVisible();
  await expect(booklog.getByTestId("credential")).toHaveCount(2);
  // 外した衛星の並び（前の試験の分が残りうる）に「読書ログ」は出ない
  await expect(page.getByTestId("satellites-gone").filter({ hasText: "dev.example.booklog" })).toHaveCount(0);
  for (const v of Object.values(values)) expect(await readWith(page, v), "効いたまま（401 ではない）").toBe(403);
});

// ---------------------------------------------------------------- 登録

// Scenario: 画面から衛星を登録できる
test("Scenario 9.2: 名前と識別子を入れて登録すると、資格情報 0 本の小節が出る", async ({ page }) => {
  await seed();
  await openSettings(page);
  await page.getByRole("button", { name: "衛星を登録" }).click();
  const form = page.getByTestId("satellite-register-form");
  await form.getByRole("textbox").nth(0).fill("家計簿");
  await form.getByRole("textbox").nth(1).fill("dev.example.kakeibo");
  await form.getByRole("button", { name: "登録する" }).click();
  const c = card(page, "dev.example.kakeibo");
  await expect(c).toBeVisible();
  await expect(c).toContainText("家計簿");
  await expect(c.getByTestId("credential")).toHaveCount(0);
  await expect(c.getByRole("button", { name: "資格情報を発行" })).toBeVisible();
  await expect(c.getByRole("button", { name: "外す" })).toBeVisible();
  await expect(page.getByTestId("satellite")).toHaveCount(6);
  await expect(form).toHaveCount(0);
});

// Scenario: 画面で形に合わない識別子を入れると理由が出て登録されない
test("Scenario 9.2: 識別子が形に合わないと入力欄の下に理由が出て、小節は増えない", async ({ page }) => {
  await seed();
  await openSettings(page);
  await page.getByRole("button", { name: "衛星を登録" }).click();
  const form = page.getByTestId("satellite-register-form");
  const id = form.getByRole("textbox").nth(1);
  await form.getByRole("textbox").nth(0).fill("家計簿");
  await id.fill("kakeibo");
  await form.getByRole("button", { name: "登録する" }).click();
  const reason = form.getByTestId("satellite-reason");
  await expect(reason).toBeVisible();
  await expect(reason).toContainText("識別子は、英小文字・数字・「-」の部分を「.」で 2 つ以上つないだ 64 文字までの形です");
  const [r, i] = await Promise.all([reason.boundingBox(), id.boundingBox()]);
  expect(r!.y, "理由は識別子の入力欄の下").toBeGreaterThanOrEqual(i!.y + i!.height);
  await expect(page.getByTestId("satellite")).toHaveCount(5);
  await expect(card(page, "kakeibo")).toHaveCount(0);
});

// ---------------------------------------------------------------- 360 px

test.describe("360 px", () => {
  test.use({ viewport: { width: 360, height: 800 } });

  /** 節の操作の対象（ボタン・入力欄）の実寸。 */
  async function sizes(page: Page): Promise<{ what: string; w: number; h: number; destructive: boolean }[]> {
    return page.getByTestId("satellites-section").locator("button, input, a").evaluateAll((els) =>
      els.map((el) => {
        const r = el.getBoundingClientRect();
        const text = (el.textContent ?? "").trim();
        return { what: `${el.tagName.toLowerCase()} ${text}`, w: r.width, h: r.height, destructive: el.tagName === "BUTTON" && (text === "外す" || (text === "やめる" && el.closest("[data-testid=satellite-remove-confirm]") !== null)) };
      }),
    );
  }

  /** 横に揺れない（文書全体と節の両方）。 */
  async function fits(page: Page): Promise<boolean> {
    return page.evaluate(() => {
      const section = document.querySelector("[data-testid=satellites-section]")!;
      return (
        document.documentElement.scrollWidth <= document.documentElement.clientWidth &&
        section.scrollWidth <= section.clientWidth
      );
    });
  }

  // Scenario: 衛星と資格情報の節は触れる大きさと幅の下限を守る
  test("Scenario 9.2: 360 px で操作の対象は 24 px 以上、外すと確認は 44 px 以上、横に揺れず、フォーカスの枠が見える", async ({ page }) => {
    await seed();
    await openSettings(page);
    expect(page.viewportSize()?.width).toBe(360);

    const check = async (stage: string, minDestructive: number): Promise<void> => {
      const all = await sizes(page);
      expect(all.length, `${stage}: 操作の対象`).toBeGreaterThan(5);
      for (const t of all) {
        expect(t.w, `${stage}: ${t.what} の幅`).toBeGreaterThanOrEqual(24);
        expect(t.h, `${stage}: ${t.what} の高さ`).toBeGreaterThanOrEqual(24);
      }
      const big = all.filter((t) => t.destructive);
      expect(big.length, `${stage}: 外す・やめる`).toBeGreaterThanOrEqual(minDestructive);
      for (const t of big) {
        expect(t.w, `${stage}: ${t.what} の幅`).toBeGreaterThanOrEqual(44);
        expect(t.h, `${stage}: ${t.what} の高さ`).toBeGreaterThanOrEqual(44);
      }
      expect(await fits(page), `${stage}: 横に揺れない`).toBe(true);
    };

    await check("ふだん", 3);

    const booklog = card(page, BOOKLOG);
    await booklog.getByRole("button", { name: "外す" }).click();
    await expect(booklog.getByTestId("satellite-remove-confirm")).toBeVisible();
    await check("外すの確認", 5);
    await booklog.getByRole("button", { name: "やめる" }).click();

    await page.getByRole("button", { name: "衛星を登録" }).click();
    await page.getByTestId("satellite-register-form").getByRole("textbox").nth(1).fill("kakeibo");
    await page.getByTestId("satellite-register-form").getByRole("button", { name: "登録する" }).click();
    await expect(page.getByTestId("satellite-reason")).toBeVisible();
    await check("登録の欄と理由", 3);
    await page.getByTestId("satellite-register-form").getByRole("button", { name: "やめる" }).click();

    await booklog.getByRole("button", { name: "資格情報を発行" }).click();
    await booklog.getByTestId("satellite-issue-form").getByRole("button", { name: "発行する" }).click();
    await expect(booklog.getByTestId("satellite-credential-value")).toBeVisible();
    await check("発行した値", 3);

    // キーボードで移ったフォーカスの枠（輪郭の幅）が見える
    await page.evaluate(() => {
      (document.activeElement as HTMLElement | null)?.blur();
      window.scrollTo(0, 0);
    });
    const outlined = (): Promise<{ inSection: boolean; width: number; style: string; what: string }> =>
      page.evaluate(() => {
        const el = document.activeElement;
        const s = el === null ? null : getComputedStyle(el);
        return {
          inSection: el !== null && document.querySelector("[data-testid=satellites-section]")!.contains(el),
          width: s === null ? 0 : parseFloat(s.outlineWidth),
          style: s?.outlineStyle ?? "none",
          what: el === null ? "" : `${el.tagName.toLowerCase()} ${(el.textContent ?? "").trim()}`,
        };
      });
    let seen = 0;
    for (let i = 0; i < 60 && seen < 8; i++) {
      await page.keyboard.press("Tab");
      const f = await outlined();
      if (!f.inSection) continue;
      seen++;
      expect(f.style, `${f.what} の輪郭の種類`).not.toBe("none");
      expect(f.width, `${f.what} の輪郭の幅`).toBeGreaterThanOrEqual(2);
    }
    expect(seen, "Tab で節の中を移れた数").toBeGreaterThanOrEqual(8);
  });
});
