// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 1 日の画面（S-2）の頭の気分の欄と、滞在の行の気分（ST17 / tasks 10.2 / design D14・D20）。本物のブラウザ・本物のサーバ。
 *
 * 見え方の assert は、偽データ（`tools/seed.sh` の「主観」）が**固定で持つ日**に対してだけ行う（e2e はここで書き込まない）:
 *   09-04 使い始める前 / 09-06 記入あり（+1）/ 09-08・09-09 未記入 / 09-10 記入あり（+2）/ 09-12 前 +1・後 +2、吸収された滞在（0）/
 *   09-15 滞在 2 つ（書き直しで +2 と -1）。
 * 偽データに無い形（滞在にだけ書いた日・前と後の両方がある滞在）は、本物のサーバの応答の**中身だけを書き換えて**ブラウザに返す
 * （形は本物。滞在は DB の錠で消せない主観を日ごとに作り直せないため、書き込んで作る道を取らない）。
 */

test.use({ viewport: { width: 360, height: 740 }, colorScheme: "dark" });

function feeling(valence: number, phase: "before" | "after", n: number): Record<string, unknown> {
  return {
    id: `e2e-${valence}-${phase}-${n}`,
    scales: [{ id: "valence", version: 1, value: valence }],
    note: null,
    written_at: `2026-09-15T2${n}:00:00+09:00`,
    ingested_at: `2026-09-15T1${n}:00:00Z`,
    phase,
    target_stay: null,
  };
}

async function open(page: Page, day: string): Promise<void> {
  await page.goto(`/#/day/${day}`);
  await page.reload();
  await expect(page.getByTestId("day-view")).toBeVisible();
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-feeling-text")).not.toContainText("読み込み中", { timeout: 15_000 });
}

const head = (page: Page) => page.getByTestId("day-feeling");

interface StayBucket {
  latest: { before: unknown; after: unknown };
}
interface FeelingsDay {
  status: string;
  day: { latest: { before: unknown; after: unknown }; records: unknown[] };
  stays: StayBucket[];
  stays_with_records: number;
}

/** 本物の応答（`/api/feelings`）を取り、`edit` で書き換えて返す。 */
async function rewriteFeelings(page: Page, edit: (day: FeelingsDay) => void): Promise<void> {
  await page.route("**/api/feelings?*", async (route) => {
    const res = await route.fetch();
    const body = (await res.json()) as { days: FeelingsDay[] };
    for (const d of body.days) edit(d);
    await route.fulfill({ response: res, json: body });
  });
}

// Scenario: 書いた日の 1 日の画面の頭に気分が出る
test("「日」の最新が +1 の日の頭に「気分 +1 快」が出て、前と後がある日は後の値が出る", async ({ page }) => {
  await open(page, "2026-09-06");
  await expect(head(page)).toContainText("気分 +1 快");
  await expect(head(page)).not.toContainText("未記入");
  // 見出しの下、並びの上
  const y = async (id: string): Promise<number> => (await page.getByTestId(id).boundingBox())?.y ?? -1;
  expect(await y("day-title")).toBeLessThan(await y("day-feeling"));
  await open(page, "2026-09-12");
  await expect(head(page)).toContainText("気分 +2 とても快");
});

// Scenario: 書いていない日の 1 日の画面の頭に未記入が出る
test("「未記入」の日は破線の枠と「気分 未記入」と「書く」が出る", async ({ page }) => {
  await open(page, "2026-09-08");
  await expect(head(page)).toContainText("気分 未記入");
  await expect(head(page).getByRole("link", { name: "書く" })).toBeVisible();
  await expect(head(page)).toHaveCSS("outline-style", "dashed");
  // 枠の色は背景と 3:1 以上（色だけで区別しない。文字も出ている）
  const ratio = await page.evaluate(() => {
    const el = document.querySelector('[data-testid="day-feeling"]') as HTMLElement;
    const lum = (css: string): number => {
      const [r, g, b] = (css.match(/[\d.]+/g) ?? ["0", "0", "0"]).slice(0, 3).map((v) => {
        const s = Number(v) / 255;
        return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
      }) as [number, number, number];
      return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    };
    const a = lum(getComputedStyle(el).outlineColor);
    const b = lum(getComputedStyle(document.querySelector('[data-testid="day-view"]') as HTMLElement).backgroundColor);
    return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
  });
  expect(ratio).toBeGreaterThanOrEqual(3);
});

// Scenario: 1 日の画面の書くから主観入力の画面が開く
test("「書く」を押すと、その日を対象にした主観入力の画面が開く", async ({ page }) => {
  await open(page, "2026-09-08");
  await head(page).getByRole("link", { name: "書く" }).click();
  await expect(page).toHaveURL(/#\/feel\/2026-09-08$/);
  await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("feel-title")).toContainText("9 月 8 日");
});

// Scenario: 滞在にだけ書いた日は未記入と出ない
test("「日」の記録が無く 2 つの滞在にだけ記録がある日は「気分 滞在ごとに 2 件」", async ({ page }) => {
  await rewriteFeelings(page, (d) => {
    d.day = { latest: { before: null, after: null }, records: [] };
    d.status = "recorded";
    d.stays_with_records = 2;
  });
  await open(page, "2026-09-15");
  await expect(head(page)).toContainText("気分 滞在ごとに 2 件");
  await expect(head(page)).not.toContainText("未記入");
});

// Scenario: 同じ滞在の書き直しは 1 と数える
test("1 つの滞在にだけ記録が 2 件ある日は「気分 滞在ごとに 1 件」", async ({ page }) => {
  await rewriteFeelings(page, (d) => {
    d.day = { latest: { before: null, after: null }, records: [] };
    d.status = "recorded";
    d.stays = d.stays.slice(0, 1);
    d.stays_with_records = 1;
  });
  await open(page, "2026-09-15");
  await expect(head(page)).toContainText("気分 滞在ごとに 1 件");
});

// Scenario: 使い始める前の日の 1 日の画面に未記入と出ない
test("最初の記録より前の日は「気分 使い始める前」で、「未記入」も破線も出ない", async ({ page }) => {
  await open(page, "2026-09-04");
  await expect(head(page)).toContainText("気分 使い始める前");
  await expect(head(page)).not.toContainText("未記入");
  await expect(head(page)).not.toHaveCSS("outline-style", "dashed");
});

// Scenario: これからの日の 1 日の画面に未記入と出ない
test("これからの日は「未記入」が出ず「書く」が出る", async ({ page }) => {
  await open(page, "2099-01-01");
  await expect(head(page)).not.toContainText("未記入");
  await expect(head(page).getByRole("link", { name: "書く" })).toBeVisible();
  await expect(head(page)).not.toHaveCSS("outline-style", "dashed");
});

// Scenario: 気分の読み出しの失敗は未記入と出ない
test("主観の読み出しが 500 のとき、失敗が出て「未記入」は出ず、その日の並びは出る", async ({ page }) => {
  await page.route("**/api/feelings?*", (route) => route.fulfill({ status: 500, body: "" }));
  await open(page, "2026-09-07");
  await expect(head(page)).toContainText("気分を読み出せませんでした");
  await expect(head(page)).not.toContainText("未記入");
  await expect(page.getByTestId("row-stay").first()).toBeVisible();
  await expect(page.getByTestId("row-feeling")).toHaveCount(0);
});

// Scenario: 日を移ると気分の欄もその日のものになる
test("「記入あり」の日から前の日（未記入）へ移ると、欄が「気分 未記入」に変わる", async ({ page }) => {
  await open(page, "2026-09-10");
  await expect(head(page)).toContainText("気分 +2 とても快");
  await page.getByRole("button", { name: "前の日" }).click();
  await expect(page).toHaveURL(/#\/day\/2026-09-09$/);
  await expect(head(page)).toContainText("気分 未記入");
});

// Scenario: 気分の欄は 1 行で 40 px 以下
test("幅 360 px で「未記入」の日も「記入あり」の日も、欄の高さは 40 px 以下で横にはみ出さない", async ({ page }) => {
  for (const day of ["2026-09-09", "2026-09-10"]) {
    await open(page, day);
    const box = await head(page).boundingBox();
    expect(box?.height ?? 999, `${day} の欄の高さ`).toBeLessThanOrEqual(40);
    expect(box?.height ?? 0, `${day} の欄に高さが無い`).toBeGreaterThan(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth), "横スクロールが出ている").toBe(true);
  }
});

// Scenario: 気分を書いた滞在の行に気分が出る
test("滞在に書いた日は、その滞在の行に「気分 …」が 1 行添えてあり、行を開く操作は変わらない", async ({ page }) => {
  await open(page, "2026-09-15");
  const rows = page.getByTestId("row-stay");
  await expect(rows.getByTestId("row-feeling")).toHaveText(["気分 +2 とても快", "気分 -1 不快"]);
  await rows.first().getByRole("button").first().click();
  await expect(rows.first().getByTestId("stay-detail")).toBeVisible();
});

// Scenario: 滞在の行は後の気分を前より先に出す
test("前（+2）と後（-1）がある滞在の行は「気分 -1 不快」", async ({ page }) => {
  await rewriteFeelings(page, (d) => {
    const first = d.stays[0];
    if (first === undefined) throw new Error("09-15 に滞在の気分が無い（偽データの前提が崩れている）");
    first.latest = { before: feeling(2, "before", 0), after: feeling(-1, "after", 1) };
  });
  await open(page, "2026-09-15");
  await expect(page.getByTestId("row-stay").first().getByTestId("row-feeling")).toHaveText("気分 -1 不快");
});

// Scenario: 1 日の画面で吸収された滞在の気分は吸収先の行に出る
test("あとから届いた位置で 2 つが 1 つになった日は、吸収先の行に吸収された滞在に書いた気分が出る", async ({ page }) => {
  await open(page, "2026-09-12");
  await expect(page.getByTestId("row-feeling")).toHaveText(["気分 0 どちらでもない"]);
  await expect(page.getByTestId("row-stay").filter({ has: page.getByTestId("row-feeling") })).toHaveCount(1);
});

// Scenario: 気分の無い滞在の行には何も添えない
test("記録の無い滞在の行には気分も「未記入」も添えない", async ({ page }) => {
  await open(page, "2026-09-07");
  const rows = page.getByTestId("row-stay");
  expect(await rows.count()).toBeGreaterThan(0);
  await expect(rows.getByTestId("row-feeling")).toHaveCount(0);
  for (const row of await rows.all()) {
    await expect(row).not.toContainText("気分");
    await expect(row).not.toContainText("未記入");
  }
});
