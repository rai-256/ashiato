// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST29「衛星と資格情報」を、設定画面で 1 本の流れで撮る:
 * 節の頭（今は読み書きできない旨）と最初の 2 つ（「最初から」・「外す」が無い）→ 形に合わない識別子は入力欄の下に理由が出て
 * 登録されない → 正しい識別子で登録すると資格情報 0 本の小節が出る → 発行すると値が 1 回だけ出て、閉じると消え、開き直しても出ない
 * → 取り消すと行が消え、その値の読み出しは 401 → 外すを押すと確認（資格情報の本数・記録は残る・登録し直すと続き）→ やめると残る
 * → 確認の外すで小節が消え、外した衛星の並びに出て、値の読み出しは 401 → 幅 360 px・ダーク（横に揺れない・確認を開いても揺れない）。
 *
 * アサーションは `e2e/settings-satellites.spec.ts` の主張の写し（spec を import すると向こうのテストまで登録されるので、
 * helper はここに写す）。足したのは見るための停止（HOLD）と、工程をつなぐための積み直しの省略だけ。
 */
declare const process: { env: Record<string, string | undefined> };

const PREFIX = "ashsat_dev.example.booklog_";
const BOOKLOG = "dev.example.booklog";
const KAKEIBO = "dev.example.kakeibo";
const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);

type Cred = { id: string; label: string };
type Sat = { id: string; identifier: string; name: string; builtin: boolean; removed_at: string | null; credentials: Cred[] };

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

/** 読み出しは本物のブラウザの fetch でやる（ログインの印は `secure`。e2e と同じ事情）。 */
async function must(page: Page, method: "GET" | "POST" | "DELETE", path: string, body?: unknown): Promise<unknown> {
  const res = await page.evaluate(
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
  expect(res.ok, `${method} ${path} は ${res.status}`).toBe(true);
  return res.json;
}

/** 資格情報の値を付けて記録の読み出しを呼ぶ（ログインの印は付けない）。 */
async function readWith(page: Page, value: string): Promise<number> {
  const res = await page.request.get("/api/events", { headers: { authorization: `Bearer ${value}` } });
  return res.status();
}

/** 衛星 5 つ（最初の 2 つ + 読書ログ・睡眠・散歩）に積む。読書ログの資格情報 2 本の値を返す（e2e の `seed` の写し）。 */
async function seed(page: Page): Promise<Record<string, string>> {
  const before = (await must(page, "GET", "/api/satellites")) as Sat[];
  for (const s of before.filter((x) => x.removed_at === null)) {
    if (s.builtin) {
      for (const c of s.credentials) await must(page, "DELETE", `/api/satellites/${s.id}/credentials/${c.id}`);
    } else {
      await must(page, "DELETE", `/api/satellites/${s.id}`);
    }
  }
  const external = before.find((x) => x.builtin && x.identifier === "ashiato.ai-external");
  expect(external, "最初の 2 つがある").toBeDefined();
  const issue = async (sat: Sat, label: string): Promise<string> =>
    ((await must(page, "POST", `/api/satellites/${sat.id}/credentials`, { label })) as { value: string }).value;
  const register = async (name: string, identifier: string): Promise<Sat> =>
    (await must(page, "POST", "/api/satellites", { name, identifier })) as Sat;
  await issue(external!, "Claude Desktop");
  const booklog = await register("読書ログ", BOOKLOG);
  const sleep = await register("睡眠", "dev.example.sleep");
  await register("散歩", "dev.example.walk");
  const values: Record<string, string> = {};
  for (const label of ["携帯", "ノート PC"]) values[label] = await issue(booklog, label);
  await issue(sleep, "携帯");
  return values;
}

async function openSettings(page: Page, count: number): Promise<void> {
  await page.goto("/#/settings");
  await page.reload();
  await expect(page.getByTestId("settings-view")).toBeVisible();
  await expect(page.getByTestId("satellite")).toHaveCount(count);
}

const card = (page: Page, identifier: string): Locator => page.locator(`[data-testid=satellite][data-identifier="${identifier}"]`);

/** 横に揺れない（文書全体と節の両方。e2e と同じ）。 */
async function fits(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const section = document.querySelector("[data-testid=satellites-section]")!;
    return document.documentElement.scrollWidth <= document.documentElement.clientWidth && section.scrollWidth <= section.clientWidth;
  });
}

test("ST29 録画: 設定画面の衛星と資格情報（登録・発行・取り消し・外す・360 px）", async ({ page, browser, browserName }) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  test.setTimeout(240_000);
  await page.goto("/");
  const values = await seed(page);

  await test.step("1. 節の頭に今は読み書きできない旨があり、最初の 2 つに「最初から」があって「外す」が無い", async () => {
    await openSettings(page, 5);
    const section = page.getByTestId("satellites-section");
    await section.scrollIntoViewIfNeeded();
    await expect(section.getByRole("heading", { level: 2, name: "衛星と資格情報" })).toBeVisible();
    await expect(page.getByTestId("satellites-lead")).toContainText("記録を読むことも書くこともできません");
    for (const [identifier, name] of [
      ["ashiato.ai-external", "外部 AI"],
      ["ashiato.ai-local", "ローカル AI"],
    ] as const) {
      const c = card(page, identifier);
      await expect(c).toContainText(name);
      await expect(c.getByTestId("satellite-builtin")).toHaveText("最初から");
      await expect(c.getByRole("button", { name: "外す" })).toHaveCount(0);
    }
    await expect(card(page, BOOKLOG).getByTestId("satellite-builtin")).toHaveCount(0);
    await expect(card(page, BOOKLOG).getByRole("button", { name: "外す" })).toBeVisible();
    const [a, b] = await Promise.all([section.boundingBox(), page.getByTestId("export-section").boundingBox()]);
    expect(a!.y, "衛星の節は書き出しの節より上").toBeLessThan(b!.y);
    await hold(page, 2);
  });

  await test.step("2. 識別子が形に合わないと入力欄の下に理由が出て、小節は増えない", async () => {
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
    await hold(page, 2);
  });

  await test.step("3. 正しい識別子で登録すると、資格情報 0 本の小節が出る", async () => {
    const form = page.getByTestId("satellite-register-form");
    await form.getByRole("textbox").nth(1).fill(KAKEIBO);
    await form.getByRole("button", { name: "登録する" }).click();
    const c = card(page, KAKEIBO);
    await expect(c).toBeVisible();
    await expect(c).toContainText("家計簿");
    await expect(c.getByTestId("credential")).toHaveCount(0);
    await expect(c.getByRole("button", { name: "資格情報を発行" })).toBeVisible();
    await expect(c.getByRole("button", { name: "外す" })).toBeVisible();
    await expect(page.getByTestId("satellite")).toHaveCount(6);
    await expect(form).toHaveCount(0);
    await c.scrollIntoViewIfNeeded();
    await hold(page, 2);
  });

  await test.step("4. 読書ログに資格情報を発行すると、値が小節の中に 1 回だけ出て、閉じると消え、開き直しても出ない", async () => {
    const booklog = card(page, BOOKLOG);
    await booklog.scrollIntoViewIfNeeded();
    await booklog.getByRole("button", { name: "資格情報を発行" }).click();
    const form = booklog.getByTestId("satellite-issue-form");
    await expect(form.getByRole("heading")).toContainText("読書ログ");
    await form.getByRole("textbox").fill("タブレット");
    await form.getByRole("button", { name: "発行する" }).click();
    const value = booklog.getByTestId("satellite-credential-value");
    await expect(value).toBeVisible();
    expect((await value.innerText()).startsWith(PREFIX), "値の頭").toBe(true);
    await expect(booklog.getByTestId("satellite-reveal")).toContainText("閉じると二度と表示できません");
    await expect(page.getByText(PREFIX)).toHaveCount(1);
    await hold(page, 3);

    await booklog.getByRole("button", { name: "設定したので閉じる" }).click();
    await expect(page.getByText(PREFIX)).toHaveCount(0);
    await expect(booklog.getByTestId("satellite-reveal")).toHaveCount(0);
    await hold(page);

    await page.reload();
    await expect(page.getByTestId("satellite")).toHaveCount(6);
    expect(await page.content(), "開き直した画面に値が無い").not.toContain("ashsat_");
    expect(await page.evaluate(() => localStorage.length + sessionStorage.length), "保存領域に何も書かれていない").toBe(0);
    const row = card(page, BOOKLOG).getByTestId("credential").filter({ hasText: "タブレット" });
    await expect(row).toHaveCount(1);
    await expect(row).toContainText("まだ使われていない");
    await row.scrollIntoViewIfNeeded();
    await hold(page, 2);
  });

  await test.step("5. 「携帯」を取り消すと行が消え、その値の読み出しは 401 になる", async () => {
    const mobile = values["携帯"]!;
    expect(await readWith(page, mobile), "取り消す前は断られるが 401 ではない（承認がまだ）").toBe(403);
    const booklog = card(page, BOOKLOG);
    const row = booklog.getByTestId("credential").filter({ hasText: "携帯" });
    await expect(row).toHaveCount(1);
    await hold(page);
    await row.getByRole("button", { name: "取り消す" }).click();
    await expect(row).toHaveCount(0);
    await expect(booklog.getByTestId("credential")).toHaveCount(2);
    expect(await readWith(page, mobile), "取り消した値の読み出し").toBe(401);
    await hold(page, 2);
  });

  await test.step("6. 外すを押すと確認が並び、やめると確認が閉じて小節と資格情報は残る", async () => {
    const booklog = card(page, BOOKLOG);
    await booklog.getByRole("button", { name: "外す" }).click();
    const confirm = booklog.getByTestId("satellite-remove-confirm");
    await expect(confirm).toBeVisible();
    await expect(confirm).toContainText("資格情報 2 本が今すぐ使えなくなります");
    await expect(confirm).toContainText("この衛星が書いた記録は消えずに残ります");
    await expect(confirm).toContainText(`同じ識別子（${BOOKLOG}）で登録し直すと、この衛星の続きになります`);
    await hold(page, 3);
    await booklog.getByRole("button", { name: "やめる" }).click();
    await expect(confirm).toHaveCount(0);
    await expect(booklog).toBeVisible();
    await expect(booklog.getByTestId("credential")).toHaveCount(2);
    await expect(page.getByTestId("satellites-gone").filter({ hasText: BOOKLOG })).toHaveCount(0);
    expect(await readWith(page, values["ノート PC"]!), "効いたまま（401 ではない）").toBe(403);
    await hold(page, 2);
  });

  await test.step("7. 確認の外すで小節が消え、外した衛星の並びに出て、値の読み出しは 401 になる", async () => {
    const booklog = card(page, BOOKLOG);
    await booklog.getByRole("button", { name: "外す" }).click();
    const confirm = booklog.getByTestId("satellite-remove-confirm");
    await expect(confirm).toBeVisible();
    await hold(page);
    await confirm.getByRole("button", { name: "外す" }).click();
    await expect(card(page, BOOKLOG)).toHaveCount(0);
    await expect(page.getByTestId("satellite")).toHaveCount(5);
    const gone = page.getByTestId("satellites-gone");
    await expect(gone).toContainText("読書ログ");
    await expect(gone).toContainText(BOOKLOG);
    await gone.scrollIntoViewIfNeeded();
    expect(await readWith(page, values["ノート PC"]!), "外した衛星の値の読み出し").toBe(401);
    await hold(page, 2);
  });

  await test.step("8. 幅 360 px・ダークでも横に揺れない（外すの確認を開いても）", async () => {
    await page.setViewportSize({ width: 360, height: 800 });
    await page.emulateMedia({ colorScheme: "dark" });
    await openSettings(page, 5);
    expect(page.viewportSize()?.width).toBe(360);
    await page.getByTestId("satellites-section").scrollIntoViewIfNeeded();
    expect(await fits(page), "ふだん: 横に揺れない").toBe(true);
    await hold(page, 2);
    const kakeibo = card(page, KAKEIBO);
    await kakeibo.scrollIntoViewIfNeeded();
    await kakeibo.getByRole("button", { name: "外す" }).click();
    await expect(kakeibo.getByTestId("satellite-remove-confirm")).toBeVisible();
    expect(await fits(page), "外すの確認: 横に揺れない").toBe(true);
    await hold(page, 2);
    await kakeibo.getByRole("button", { name: "やめる" }).click();
    await expect(kakeibo.getByTestId("satellite-remove-confirm")).toHaveCount(0);
  });
});
