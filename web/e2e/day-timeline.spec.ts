// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * **ST25（1 日を時刻順に見る）の画面の Scenario を本物のブラウザで担保する。**
 *
 * `run.sh` と同じ `tools/stack.sh` が立てた偽データ（`SEED=normal`）の `#/day/2026-09-07` を使う。
 * アサートするのは数値と経路（実寸・可視・フォーカス・URL・背景の色）で、スクリーンショットの比較は使わない。
 */
const DAY = "2026-09-07";
const PHONE = { width: 360, height: 640 };

// spec の値をリテラルで持つ（実装の定数と突き合わせると、定数を変えても緑のままになる）
const ROW_MAX_PX = 200;
const OPEN_TARGET_PX = 24;
const DESTRUCTIVE_PX = 44;

async function openDay(page: Page, day = DAY) {
  await page.goto(`/#/day/${day}`);
  await page.reload();
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-error")).toHaveCount(0);
  await expect(page.getByTestId("day-view")).toBeVisible();
}

const rows = (page: Page) =>
  page.getByRole("list", { name: "1 日の並び" }).locator("> li");

/** 背景色（rgb）の輝度 0〜255 の目安 */
async function luminance(page: Page, testId: string): Promise<number> {
  const bg = await page
    .getByTestId(testId)
    .evaluate((el) => getComputedStyle(el).backgroundColor);
  const m = bg.match(/\d+(\.\d+)?/g)?.map(Number) ?? [];
  expect(m.length, `背景色を読めない: ${bg}`).toBeGreaterThanOrEqual(3);
  return 0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2];
}

/**
 * 滞在の行を先頭から 1 つずつ開き、`want` を満たした最初の行を開いたまま返す
 * （位置だけの滞在は記録の一覧が空なので、偽データのどの滞在が当たるかに頼らない）。
 */
async function openStayWhere(
  page: Page,
  what: string,
  want: (records: Locator) => Promise<boolean>,
): Promise<Locator> {
  const stays = page.getByTestId("row-stay");
  const n = await stays.count();
  for (let i = 0; i < n; i++) {
    const stay = stays.nth(i);
    await stay.getByRole("button", { expanded: false }).first().click();
    await expect(stay.getByTestId("stay-detail")).toBeVisible();
    await expect(stay.getByText("記録を読み込み中…")).toHaveCount(0);
    if (await want(stay.getByTestId("records"))) return stay;
    await stay.getByRole("button", { expanded: true }).first().click();
  }
  throw new Error(`${what} 滞在が無い`);
}

const hasFolded = (records: Locator) =>
  records
    .getByRole("button")
    .count()
    .then((c) => c > 0);

type Rec = { kind: string; source: string; title?: string };

/** その日の記録（`GET /day/records`）。**画面の中から**画面と同じ口で読む（ログインの印と送り元は画面のもの）。 */
async function dayRecords(page: Page): Promise<Rec[]> {
  const got = await page.evaluate(async (day) => {
    const res = await fetch(`/api/day/records?date=${day}`);
    return { status: res.status, body: res.ok ? ((await res.json()) as unknown) : null };
  }, DAY);
  expect(got.status, "/api/day/records の応答").toBe(200);
  return got.body as Rec[];
}

// spec の「PC の記録が 1,000 件以上ある日」（リテラルで持つ）
const PREMISE_PC_RECORDS = 1000;

// Scenario: 閉じた画面に記録 1 件ごとの行は出ない
test("閉じた画面に PC の前景の題名が出ず、行は 100 未満", async ({ page }) => {
  await page.setViewportSize(PHONE);
  await openDay(page);
  // 前提: PC の記録が 1,000 件以上ある日（偽データがそうでなくなったら、この試験は何も確かめていない）
  const records = await dayRecords(page);
  const pc = records.filter((r) => r.source === "pc");
  expect(pc.length, "PC の記録が 1,000 件に届かない（Scenario の前提が崩れている）").toBeGreaterThanOrEqual(PREMISE_PC_RECORDS);
  // 題名は偽データの形に頼らず、サーバが返した前景の題名そのものを探す
  const titles = [...new Set(pc.filter((r) => r.kind === "foreground").map((r) => r.title ?? ""))].filter((t) => t.length >= 3);
  expect(titles.length, "前景の題名が 1 つも無い").toBeGreaterThan(0);
  const text = await page.getByTestId("day-view").innerText();
  const shown = titles.filter((t) => text.includes(t));
  expect(shown, "閉じた画面に前景の題名が出ている").toEqual([]);
  const n = await rows(page).count();
  expect(n, "行が 1 つも無い").toBeGreaterThan(0);
  expect(n).toBeLessThan(100);
});

// Scenario: 幅 360 px で開いてすぐ 6 行以上見える
test("開いてすぐ、全体が画面に入る行が 6 行以上ある", async ({ page }) => {
  await page.setViewportSize(PHONE);
  await openDay(page);
  const n = await rows(page).count();
  let inside = 0;
  for (let i = 0; i < n; i++) {
    const box = await rows(page).nth(i).boundingBox();
    if (box && box.y >= 0 && box.y + box.height <= PHONE.height) inside++;
  }
  expect(inside).toBeGreaterThanOrEqual(6);
});

// Scenario: 幅 360 px で横にはみ出さない
test("横スクロールが出ない", async ({ page }) => {
  await page.setViewportSize(PHONE);
  await openDay(page);
  const overflow = await page.evaluate(
    () =>
      document.documentElement.scrollWidth -
      document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
});

// Scenario: 閉じた行は 200 px を超えない
test("閉じた行はどれも 200 px 以下", async ({ page }) => {
  await page.setViewportSize(PHONE);
  await openDay(page);
  const n = await rows(page).count();
  expect(n).toBeGreaterThan(0);
  for (let i = 0; i < n; i++) {
    const box = await rows(page).nth(i).boundingBox();
    expect(box?.height ?? 0, `${i} 番目の行`).toBeLessThanOrEqual(ROW_MAX_PX);
  }
});

const measure24 = async (loc: Locator, what: string) => {
  const box = await loc.boundingBox();
  expect(box?.width ?? 0, `${what} の幅`).toBeGreaterThanOrEqual(
    OPEN_TARGET_PX,
  );
  expect(box?.height ?? 0, `${what} の高さ`).toBeGreaterThanOrEqual(
    OPEN_TARGET_PX,
  );
};

// Scenario: 開く操作と行き先は 24 px を下回らない
test("稼働状況・マスタ管理への行き先は 24 px 以上", async ({ page }) => {
  await page.setViewportSize(PHONE);
  await openDay(page);
  await measure24(page.getByRole("link", { name: "稼働状況" }), "稼働状況");
  await measure24(page.getByRole("link", { name: "マスタ管理" }), "マスタ管理");
});

// Scenario: 開く操作と行き先は 24 px を下回らない
test("行を開く操作・畳んだ行を開く操作は 24 px 以上", async ({ page }) => {
  await page.setViewportSize(PHONE);
  await openDay(page);
  await measure24(
    page.getByTestId("row-stay").first().getByRole("button").first(),
    "滞在の行を開く操作",
  );
  // 畳んだ行は、同じアプリの連続がある滞在の中にある
  const withFold = await openStayWhere(page, "畳んだ行のある", hasFolded);
  await measure24(
    withFold.getByTestId("records").getByRole("button").first(),
    "畳んだ行を開く操作",
  );
});

// Scenario: 時刻順の記録を足しても消す操作は 44 px を下回らない
test("時刻順の記録が 100 件以上ある滞在でも「この滞在を消す」は 44 px 以上", async ({
  page,
}) => {
  await page.setViewportSize(PHONE);
  await openDay(page);
  const stay = await openStayWhere(
    page,
    "時刻順の記録が 100 件以上ある",
    async (r) => (await r.locator("li").count()) >= 100,
  );
  const box = await stay
    .getByRole("button", { name: "この滞在を消す" })
    .boundingBox();
  expect(box?.width ?? 0).toBeGreaterThanOrEqual(DESTRUCTIVE_PX);
  expect(box?.height ?? 0).toBeGreaterThanOrEqual(DESTRUCTIVE_PX);
});

// Scenario: 1 日の画面から稼働状況へ移れる
test("稼働状況への行き先で #/coverage になり、稼働状況の画面が出る", async ({
  page,
}) => {
  await openDay(page);
  await page.getByRole("link", { name: "稼働状況" }).click();
  await expect(page).toHaveURL(/#\/coverage$/);
  await expect(page.locator("[data-role=grid]").first()).toBeVisible();
});

// Scenario: 1 日の画面からマスタ管理へ移れる
test("マスタ管理への行き先で #/master になり、マスタ管理の画面が出る", async ({
  page,
}) => {
  await openDay(page);
  await page.getByRole("link", { name: "マスタ管理" }).click();
  await expect(page).toHaveURL(/#\/master$/);
  await expect(page.getByTestId("master-view")).toBeVisible();
});

/**
 * 畳んだ行のある滞在を Tab → Enter で開き、その中の畳んだ行へ Tab → Enter で開く。
 * 滞在の行・畳んだ行にフォーカスがあるときの outline-style を、そのたびに測って返す。
 */
async function openByKeyboard(page: Page) {
  const outline = () =>
    page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      return el && el !== document.body
        ? getComputedStyle(el).outlineStyle
        : "none";
    });
  const tabTo = async (target: Locator, limit: number) => {
    for (let i = 0; i < limit; i++) {
      await page.keyboard.press("Tab");
      if (await target.evaluate((el) => el === document.activeElement))
        return true;
    }
    return false;
  };

  // どの滞在に畳んだ行があるかはマウスで調べて閉じ、そのうえでキーボードだけで開き直す
  const target = await openStayWhere(page, "畳んだ行のある", hasFolded);
  const stayButton = target.getByRole("button").first();
  await stayButton.click();
  await expect(target.getByTestId("records")).toHaveCount(0);
  await page.evaluate(() =>
    (document.activeElement as HTMLElement | null)?.blur(),
  );

  expect(await tabTo(stayButton, 80), "Tab で滞在の行へ届かなかった").toBe(
    true,
  );
  const stayOutline = await outline();
  await page.keyboard.press("Enter");
  await expect(target.getByTestId("records")).toBeVisible();

  const fold = target.getByTestId("records").getByRole("button").first();
  expect(await tabTo(fold, 5), "Tab で畳んだ行へ届かなかった").toBe(true);
  const foldOutline = await outline();
  await page.keyboard.press("Enter");
  return { target, stayOutline, foldOutline };
}

// Scenario: 畳んだ行はキーボードで開ける
test("Tab → Enter で畳んだ行を開くと、畳んだ件数と同じ数の記録が出る", async ({ page }) => {
  await openDay(page);
  const { target } = await openByKeyboard(page);
  await expect(target.getByTestId("fold-records")).toBeVisible();
  // 畳んだ行の文字の末尾は「<件数> 件」
  const label = await target.getByTestId("records").getByRole("button", { expanded: true }).first().innerText();
  const folded = Number(/(\d+) 件\s*$/.exec(label)?.[1] ?? NaN);
  expect(folded, `畳んだ行の件数を読めない: ${label}`).toBeGreaterThan(1);
  expect(await target.getByTestId("fold-records").locator("li").count()).toBe(folded);
});

// Scenario: キーボードで行を開くとフォーカスの位置が見える
test("フォーカスが滞在の行にあるときも畳んだ行にあるときも輪郭が出る", async ({
  page,
}) => {
  await openDay(page);
  const { stayOutline, foldOutline } = await openByKeyboard(page);
  expect(stayOutline, "滞在の行に輪郭が無い").not.toBe("none");
  expect(foldOutline, "畳んだ行に輪郭が無い").not.toBe("none");
});

// Scenario: OS をライトに切り替えると 1 日の画面が明るくなる
test("ダーク → ライト → ダークで背景の輝度が上がって戻る", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await openDay(page);
  const dark = await luminance(page, "day-view");
  await page.emulateMedia({ colorScheme: "light" });
  await expect
    .poll(() => luminance(page, "day-view"))
    .toBeGreaterThan(dark + 100);
  await page.emulateMedia({ colorScheme: "dark" });
  await expect.poll(() => luminance(page, "day-view")).toBeLessThan(60);
});

// Scenario: OS の明暗が取得できないときはダーク
test("明暗を取得できないとき、背景はダークの色", async ({ page }) => {
  // Chromium の `colorScheme: "no-preference"` は light と同じに matchMedia へ出る（実測 2026-10-02）ので、
  // 「取得できない」は matchMedia そのものが無い状態で作る
  await page.addInitScript(() => {
    Object.defineProperty(window, "matchMedia", {
      value: undefined,
      configurable: true,
    });
  });
  await openDay(page);
  expect(await luminance(page, "day-view")).toBeLessThan(60);
});

// Scenario: 暦に無い日付の画面も OS の明暗に追従する
test("ライトで暦に無い日付を開くと、背景はライトの色", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/#/day/2026-13-45");
  await page.reload();
  await expect(page.getByTestId("day-invalid")).toBeVisible();
  expect(await luminance(page, "day-invalid")).toBeGreaterThan(200);
});
