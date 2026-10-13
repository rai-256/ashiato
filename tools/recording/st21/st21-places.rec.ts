// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST21「場所を登録し、識別子を変えない」を、マスタ管理の「場所」タブで撮る:
 * タブ（個人属性・場所）→ 場所のカード（最近居た順・合計・最後に居た日・広さ・帯・座標）→ 前の名前・座標を押して開く →
 * 「場所を足す」で名前の無い居た所 → 名前と広さを入れて登録 → カードが増える → 名前を変える →
 * カードは同じ場所のまま（識別子が変わらない）で、前の名前に元の名前が出る。
 *
 * 偽データは `tools/record-env.sh` が立てる `SEED=normal`（`tools/seed.sh` の「場所の登録（ST21）」）:
 * 場所は「職場」（前の名前「本社」）と「自宅」の 2 つ、名前の無い居た所が 7 か所。**応答の差し替え（route）は使わない。**
 * 居た所が 10 か所に届かないので「残り N か所」はこの動画には出ない（`e2e/places.spec.ts` が 13 か所を作って担保する）。
 * 登録と名前の変更は録画の専用環境（毎回作り直す DB）に書く。
 *
 * アサーションは `e2e/places.spec.ts` の主張の写し（タブ・並び・カードの中身・前の値・上位 10 件・登録で閉じる）に、
 * 名前を変えても場所の識別子が変わらないこと（`/api/places` の id）を足したもの。見るための停止（HOLD）だけを足した。
 */
declare const process: { env: Record<string, string | undefined> };

const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const NEW_NAME = "スーパー";
const RENAMED = "いつものスーパー";

type Place = { id: string; name: string; radius_m: number; stays: { minutes: number; last_day: string | null } };
type Candidate = { lat: number; lon: number };

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

const coordText = (lat: number, lon: number): string => `${lat.toFixed(4)}, ${lon.toFixed(4)}`;

/** 画面の中から、画面と同じ口（ログインの印）で読む */
async function api<T>(page: Page, path: string): Promise<T> {
  const got = await page.evaluate(async (p) => {
    const res = await fetch(p);
    return { status: res.status, body: res.ok ? ((await res.json()) as unknown) : null };
  }, path);
  expect(got.status, `${path} の応答`).toBe(200);
  return got.body as T;
}
const places = async (page: Page): Promise<Place[]> => (await api<{ places: Place[] }>(page, "/api/places")).places;
const candidates = async (page: Page): Promise<Candidate[]> =>
  (await api<{ candidates: Candidate[] }>(page, "/api/places/candidates")).candidates;

const cards = (page: Page): Locator => page.getByTestId("place-card");
const cardOf = (page: Page, name: string): Locator =>
  cards(page).filter({ has: page.getByRole("heading", { name, exact: true }) });

test("ST21 録画: 場所のタブ → カードと前の値 → 場所を足す → 登録 → 名前を変えても同じ場所", async ({ page, browser, browserName }) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  // 見るための停止と操作の間隔（slowMo）で、既定の 30 秒を使い切る
  test.setTimeout(120_000);

  await test.step("1. マスタ管理: タブは「個人属性」「場所」の 2 つ。場所を押すと場所の画面", async () => {
    await page.goto("/#/master");
    await page.reload();
    const tabs = page.getByRole("tab");
    await expect(tabs).toHaveText(["個人属性", "場所"], { timeout: 15_000 });
    await expect(page.getByRole("tab", { name: /人物/ })).toHaveCount(0);
    await hold(page);
    await tabs.nth(1).click();
    await expect(page).toHaveURL(/#\/master\/places$/);
    await expect(page.getByTestId("places-view")).toBeVisible();
    await expect(page.getByTestId("places-loading")).toBeHidden({ timeout: 15_000 });
    await hold(page);
  });

  await test.step("2. 場所のカード: 登録した場所だけが最近居た順に、合計・最後に居た日・広さ・帯・座標つきで並ぶ", async () => {
    const list = await places(page);
    expect(list.length, "偽データの場所（職場・自宅）").toBeGreaterThanOrEqual(2);
    await expect(cards(page)).toHaveCount(list.length);
    await expect(page.getByTestId("place-name")).toHaveText(list.map((p) => p.name));
    for (const p of list) {
      const card = cardOf(page, p.name);
      await expect(card.locator("[data-testid=place-band] [data-hour]")).toHaveCount(24);
      await expect(card.getByTestId("place-meta")).toContainText(`広さ ${p.radius_m} m`);
      if (p.stays.last_day !== null) await expect(card.getByTestId("place-meta")).toContainText(`最後に居た日 ${p.stays.last_day}`);
      await expect(card.getByTestId("place-coord")).toBeVisible();
    }
    await hold(page, 3);
  });

  await test.step("3. 「職場」の「前の名前・座標」を押すと、前の名前「本社」が出る（押すまでは出ない）", async () => {
    const card = cardOf(page, "職場");
    await expect(card.getByTestId("place-previous")).toHaveCount(0);
    await expect(card).not.toContainText("本社");
    await card.getByRole("button", { name: /^前の名前・座標 \d+/ }).click();
    await expect(card.getByTestId("previous-name")).toContainText("前の名前: 本社");
    await hold(page, 3);
    await card.getByRole("button", { name: /^前の名前・座標 \d+/ }).click();
    await expect(card.getByTestId("place-previous")).toHaveCount(0);
  });

  const target = await test.step("4. 「場所を足す」: 名前の無い居た所が上位 10 件まで、合計・帯・件数・期間つきで出る", async () => {
    await page.getByRole("button", { name: "場所を足す" }).click();
    const cands = await candidates(page);
    expect(cands.length, "偽データの名前の無い居た所").toBeGreaterThan(0);
    const rows = page.getByTestId("place-candidate");
    await expect(rows).toHaveCount(Math.min(10, cands.length));
    await expect(page.getByTestId("candidate-coord")).toHaveText(cands.slice(0, 10).map((c) => coordText(c.lat, c.lon)));
    const rest = page.getByRole("button", { name: /^残り \d+ か所$/ });
    await expect(rest).toHaveCount(cands.length > 10 ? 1 : 0);
    await rows.first().scrollIntoViewIfNeeded();
    await hold(page, 2);
    await page.mouse.wheel(0, 500);
    await hold(page, 2);
    return cands[0];
  });

  const created = await test.step(`5. 先頭の居た所に「名前を付ける」→「${NEW_NAME}」・広さ 200 m で登録 → フォームが閉じてカードが増える`, async () => {
    const before = (await places(page)).length;
    const row = page.getByTestId("place-candidate").filter({ hasText: coordText(target.lat, target.lon) });
    await row.scrollIntoViewIfNeeded();
    await row.getByRole("button", { name: "名前を付ける" }).click();
    const form = row.getByTestId("place-add-form");
    // 座標は居た所の中心の文字で、書き換える欄は無い（入力は名前と補足の 2 つ）
    await expect(form.getByTestId("place-add-coord")).toHaveText(coordText(target.lat, target.lon));
    await expect(form.locator("input:not([type=radio])")).toHaveCount(2);
    await expect(form.getByRole("radio", { name: "100 m" })).toBeChecked();
    await hold(page);
    await form.getByLabel("名前", { exact: true }).pressSequentially(NEW_NAME, { delay: 120 });
    await form.getByRole("radio", { name: "200 m" }).check();
    await hold(page);
    await form.getByRole("button", { name: "登録する" }).click();
    await expect(page.getByTestId("place-add-form")).toHaveCount(0);
    const card = cardOf(page, NEW_NAME);
    await expect(card).toHaveCount(1, { timeout: 15_000 });
    await expect(card.getByTestId("place-coord")).toHaveText(coordText(target.lat, target.lon));
    await expect(card.getByTestId("place-meta")).toContainText("広さ 200 m");
    const after = await places(page);
    expect(after.length).toBe(before + 1);
    await card.scrollIntoViewIfNeeded();
    await hold(page, 3);
    return after.find((p) => p.name === NEW_NAME)!;
  });

  await test.step(`6. 「名前を変える」で「${RENAMED}」に → 同じ場所（識別子は変わらない）のまま、前の名前に「${NEW_NAME}」`, async () => {
    const card = cardOf(page, NEW_NAME);
    await card.getByRole("button", { name: "名前を変える" }).click();
    const form = card.getByTestId("place-change-form");
    await expect(form).toBeVisible();
    const field = form.getByLabel("名前", { exact: true });
    await field.fill("");
    await field.pressSequentially(RENAMED, { delay: 120 });
    await hold(page);
    await form.getByRole("button", { name: "変える" }).click();
    const renamed = cardOf(page, RENAMED);
    await expect(renamed).toHaveCount(1, { timeout: 15_000 });
    await expect(cardOf(page, NEW_NAME)).toHaveCount(0);
    const list = await places(page);
    const now = list.find((p) => p.name === RENAMED);
    expect(now?.id, "名前を変えたら場所の識別子が変わった").toBe(created.id);
    expect(list.filter((p) => p.name === NEW_NAME)).toEqual([]);
    await renamed.scrollIntoViewIfNeeded();
    await renamed.getByRole("button", { name: /^前の名前・座標 \d+/ }).click();
    await expect(renamed.getByTestId("previous-name")).toContainText(`前の名前: ${NEW_NAME}`);
    // どの識別子も画面の文字に出ない
    expect(await page.evaluate(() => document.body.innerText)).not.toContain(created.id);
    await hold(page, 3);
  });
});
