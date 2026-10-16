// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Locator, type Page, type Response } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST17「その日どう感じたかを残す」を、幅 360 px・暗で撮る:
 * 通知から開いた今日の主観入力 → 快を 1 回押すと保存（保存と記入の日時と対象の日）→ ひとこと → 元気さ →
 * 書き直した履歴 → 前の日へ → 暦（偽データの 30 日。未記入の破線と「未」）→ 滞在の行と履歴（09-15）→
 * 通知の時刻を変える（後始末で 21:00 に戻す）→ 1 日の画面の頭の気分（記入あり・未記入から「書く」）と滞在の行の気分 → 明。
 *
 * 書く工程は、実行ごとに乱数で選んだ 2100 年以降の日を `page.clock` の「今日」にして書く（`e2e/feel.spec.ts` と同じ）。
 * 時計はページを開く前に 1 回だけ入れる（`page.clock.install` は 1 ページに 1 回）。偽データの日（`tools/seed.sh` の主観の節。
 * 2026-09-04〜10-03）はどれも「過ぎた日」として読まれ、見え方の assert はそこに対して行う（書かない）。
 * アサーションは `e2e/feel.spec.ts` `e2e/feel-parts.spec.ts` `e2e/day-feeling.spec.ts` の主張の写し。足したのは見るための停止（HOLD）だけ。
 *
 * 通知の時刻は録画の DB に残り、続く Android のシナリオ（ST17-reminder）が本物のサーバから読むので、必ず 21:00 に戻す。
 */
declare const process: { env: Record<string, string | undefined> };

test.use({ viewport: { width: 360, height: 740 }, colorScheme: "dark" });

/** 画面の外から `/api` を直に叩く要求は API の合言葉（Bearer）で通す（e2e と同じ。録画では `tools/record-env.sh` が渡す）。 */
const BEARER = { Authorization: `Bearer ${process.env.API_TOKEN ?? "dev-token-0123456789abcdef"}` };
const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);
const DAY_MS = 86_400_000;

const isIngest = (r: Response): boolean => r.url().includes("/api/ingest") && r.request().method() === "POST";
const shift = (day: string, n: number): string => new Date(Date.parse(`${day}T12:00:00Z`) + n * DAY_MS).toISOString().slice(0, 10);
const at = (day: string, hm: string): Date => new Date(`${day}T${hm}:00+09:00`);

/** 2100-01-01 から約 8 年の中の、実行ごとに違う日（e2e と同じ）。 */
function freshDay(): string {
  return shift("2100-01-01", Math.floor(Math.random() * 3000));
}

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

interface Stored {
  scales: { id: string; value: number }[];
  note: string | null;
}
async function records(page: Page, day: string): Promise<Stored[]> {
  const res = await page.request.get(`/api/feelings?from=${day}&to=${day}`, { headers: BEARER });
  expect(res.status()).toBe(200);
  const body = (await res.json()) as { days: { day: { records: Stored[] } }[] };
  return body.days[0]?.day.records ?? [];
}

/** 主観入力の画面を開き、ボタンと滞在の読み出しが済むまで待つ。 */
async function openFeel(page: Page, hash: string): Promise<void> {
  await page.goto(`/${hash}`);
  await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("feel-stays-loading")).toHaveCount(0);
}

/** 1 日の画面を開き、気分の欄の読み込みが済むまで待つ（e2e と同じ。ハッシュだけの遷移は読み直さない）。 */
async function openDay(page: Page, day: string): Promise<void> {
  await page.goto(`/#/day/${day}`);
  await page.reload();
  await expect(page.getByTestId("day-view")).toBeVisible();
  await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.getByTestId("day-feeling-text")).not.toContainText("読み込み中", { timeout: 15_000 });
}

async function pressSaved(page: Page, target: Locator): Promise<void> {
  const [res] = await Promise.all([page.waitForResponse(isIngest), target.click()]);
  expect(res.status()).toBe(200);
}

const cal = (page: Page, date: string): Locator => page.getByTestId(`cal-${date}`);
const mark = (page: Page, date: string): Locator => cal(page, date).getByTestId("cal-mark");
const stayRow = (page: Page, i: number): Locator => page.getByTestId("feel-stay-row").nth(i);
const btn = (r: Locator, name: string): Locator => r.getByRole("button", { name, exact: true });
const head = (page: Page): Locator => page.getByTestId("day-feeling");

test("ST17 録画: その日どう感じたかを残す（押した瞬間に保存・暦・滞在・通知の時刻・1 日の画面の気分）", async ({ page, browser, browserName }) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  // 見るための停止と操作の間隔（slowMo）で、既定の 30 秒を使い切る
  test.setTimeout(240_000);
  const day = freshDay();
  await page.clock.install({ time: at(day, "21:05") });

  try {
    await test.step("1. 通知から今日の主観入力を開く（日付の無いアドレス。見出しは暦の今日）", async () => {
      await openFeel(page, "#/feel");
      await expect(page.getByTestId("feel-title").locator("time")).toHaveAttribute("datetime", day);
      const group = page.getByRole("group", { name: "快–不快" });
      const expected = ["-2 とても不快", "-1 不快", "0 どちらでもない", "+1 快", "+2 とても快"];
      await expect(group.getByRole("button")).toHaveCount(5);
      for (const [i, name] of expected.entries()) await expect(group.getByRole("button").nth(i)).toHaveAccessibleName(name);
      // ひとことと元気さは押す前から見えている
      await expect(page.getByTestId("feel-note")).toBeVisible();
      await expect(page.getByRole("group", { name: "元気さ" }).getByRole("button")).toHaveCount(5);
      // 前に書くか後に書くかを選ぶ操作は無い
      await expect(page.getByRole("radio")).toHaveCount(0);
      await expect(page.getByRole("combobox")).toHaveCount(0);
      expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(0);
      await hold(page, 2);
    });

    await test.step("2. 快を 1 回押すと、その場で保存される（保存・記入の日時・対象の日が出る）", async () => {
      const before = (await records(page, day)).length;
      await pressSaved(page, page.getByTestId("valence-1"));
      const after = await records(page, day);
      expect(after.length - before).toBe(1);
      expect(after[0].scales.find((s) => s.id === "valence")?.value).toBe(1);
      const ok = page.getByTestId("feel-notice-ok");
      await expect(ok).toContainText("保存しました");
      await expect(ok).toContainText("+1 快");
      await expect(ok).toContainText("記入");
      await expect(ok).toContainText(`対象の日 ${day}`);
      await hold(page, 2);
    });

    await test.step("3. ひとこと「散歩した」を入れて欄を離れると、記録がもう 1 件積まれる", async () => {
      await page.getByTestId("feel-note").fill("散歩した");
      const [res] = await Promise.all([page.waitForResponse(isIngest), page.getByTestId("feel-note").blur()]);
      expect(res.status()).toBe(200);
      const all = await records(page, day);
      expect(all).toHaveLength(2);
      expect(all[0].note).toBe("散歩した");
      expect(all[0].scales.find((s) => s.id === "valence")?.value).toBe(1);
      await hold(page, 2);
    });

    await test.step("4. 元気さ +2 を押すと、今の快–不快と一緒にもう 1 件積まれる", async () => {
      await pressSaved(page, page.getByTestId("energy-2"));
      const all = await records(page, day);
      expect(all).toHaveLength(3);
      expect(all[0].scales.map((s) => [s.id, s.value])).toEqual([
        ["valence", 1],
        ["energy", 2],
      ]);
      await hold(page, 2);
    });

    await test.step("5. 開き直すと最新の値が選ばれていて、書き直した履歴 3 件を開ける（書き換える操作は無い）", async () => {
      await page.reload();
      await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
      await expect(page.getByTestId("valence-1")).toHaveAttribute("aria-pressed", "true");
      await page.getByRole("button", { name: "書き直した履歴 3 件" }).click();
      const rows = page.getByTestId("feel-history-row");
      await expect(rows).toHaveCount(3);
      for (const row of await rows.all()) await expect(row).toContainText("記入");
      await expect(page.getByRole("button", { name: /書き換|削除|消す|編集/ })).toHaveCount(0);
      await expect(page.getByTestId("feel-history").getByRole("button")).toHaveCount(0);
      await rows.first().scrollIntoViewIfNeeded();
      await hold(page, 2);
    });

    await test.step("6. 「前の日へ」を 1 回押すと昨日を書ける", async () => {
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.getByRole("button", { name: "前の日へ" }).click();
      await expect(page).toHaveURL(new RegExp(`#/feel/${shift(day, -1)}$`));
      await expect(page.getByTestId("feel-title").locator("time")).toHaveAttribute("datetime", shift(day, -1));
      await hold(page, 2);
    });

    await test.step("7. 暦（2026-10-03 で終わる 30 日）: 未記入の日は破線と「未」、使い始める前の日には付かない、記入ありの日は値", async () => {
      await openFeel(page, "#/feel/2026-10-03");
      const cells = page.locator('[data-testid^="cal-20"]');
      await expect(cells).toHaveCount(30);
      await expect(cells.first()).toHaveAttribute("data-testid", "cal-2026-09-04");
      await expect(cells.last()).toHaveAttribute("data-testid", "cal-2026-10-03");
      for (const d of ["2026-09-08", "2026-09-09", "2026-10-01"]) {
        await expect(cal(page, d)).toHaveCSS("border-top-style", "dashed");
        await expect(mark(page, d)).toHaveText("未");
      }
      for (const d of ["2026-09-04", "2026-09-05"]) {
        await expect(cal(page, d)).toHaveAttribute("data-status", "before_start");
        await expect(mark(page, d)).toHaveText("");
      }
      await expect(mark(page, "2026-09-07")).toHaveText("+1");
      await expect(mark(page, "2026-09-12")).toHaveText("+2");
      await cal(page, "2026-09-08").scrollIntoViewIfNeeded();
      await hold(page, 3);
    });

    await test.step("8. 暦の 09-15 を押すと、その日の滞在 2 件が始まりの順に並び、最新の気分が押された状態で出る", async () => {
      await cal(page, "2026-09-15").click();
      await expect(page).toHaveURL(/#\/feel\/2026-09-15$/);
      await expect(page.getByTestId("feel-stays-loading")).toHaveCount(0);
      await expect(page.getByTestId("feel-stay-row")).toHaveCount(2);
      await expect(page.getByTestId("feel-stay-range")).toHaveText(["10:00 – 10:40", "15:00 – 15:40"]);
      await expect(btn(stayRow(page, 0), "+2 とても快")).toHaveAttribute("aria-pressed", "true");
      await expect(btn(stayRow(page, 1), "-1 不快")).toHaveAttribute("aria-pressed", "true");
      await stayRow(page, 0).scrollIntoViewIfNeeded();
      await hold(page, 2);
      await stayRow(page, 0).getByRole("button", { name: "書き直した履歴 2 件" }).click();
      const lines = stayRow(page, 0).getByTestId("feel-stay-history-row");
      await expect(lines).toHaveCount(2);
      await expect(lines.nth(0)).toContainText("+2");
      await expect(lines.nth(1)).toContainText("+1");
      await hold(page, 2);
    });

    await test.step("9. 前の日から続く滞在の行（09-11）: 「前の日から」と、前の日に書いた +1", async () => {
      await openFeel(page, "#/feel/2026-09-11");
      const carried = stayRow(page, 0);
      await expect(carried.getByTestId("feel-stay-carried")).toHaveText("前の日から");
      await expect(carried.getByTestId("feel-stay-range")).toContainText("9/10 19:00");
      await expect(btn(carried, "+1 快")).toHaveAttribute("aria-pressed", "true");
      await carried.scrollIntoViewIfNeeded();
      await hold(page, 2);
    });

    await test.step("10. 通知の時刻を 22:30 にして保存すると、いまの時刻と通知の判断の応答が 22:30 になる", async () => {
      await openFeel(page, `#/feel/${day}`);
      await page.getByTestId("feel-reminder-at").scrollIntoViewIfNeeded();
      await expect(page.getByTestId("feel-reminder-current")).toHaveText(/^いまの通知の時刻: \d\d:\d\d$/);
      await hold(page);
      await page.getByTestId("feel-reminder-at").fill("22:30");
      const [res] = await Promise.all([
        page.waitForResponse((r) => r.url().includes("/api/feelings/reminder") && r.request().method() === "POST"),
        page.getByTestId("feel-reminder-save").click(),
      ]);
      expect(res.status()).toBe(200);
      await expect(page.getByTestId("feel-reminder-current")).toHaveText("いまの通知の時刻: 22:30");
      await expect(page.getByTestId("feel-reminder-ok")).toHaveText("通知の時刻を 22:30 にしました");
      const decision = await page.request.get(`/api/feelings/reminder?date=${day}`, { headers: BEARER });
      expect(decision.status()).toBe(200);
      expect(((await decision.json()) as { at: string }).at).toBe("22:30");
      await hold(page, 2);
    });

    await test.step("11. 1 日の画面の頭: 書いた日（09-06）は「気分 +1 快」、前と後がある日（09-12）は後の値", async () => {
      await openDay(page, "2026-09-06");
      await expect(head(page)).toContainText("気分 +1 快");
      await expect(head(page)).not.toContainText("未記入");
      await hold(page, 2);
      await openDay(page, "2026-09-12");
      await expect(head(page)).toContainText("気分 +2 とても快");
      await hold(page);
    });

    await test.step("12. 書いていない日（09-08）は破線の枠と「気分 未記入」と「書く」。「書く」でその日の主観入力が開く", async () => {
      await openDay(page, "2026-09-08");
      await expect(head(page)).toContainText("気分 未記入");
      await expect(head(page)).toHaveCSS("outline-style", "dashed");
      await hold(page, 2);
      await head(page).getByRole("link", { name: "書く" }).click();
      await expect(page).toHaveURL(/#\/feel\/2026-09-08$/);
      await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
      await expect(page.getByTestId("feel-title")).toContainText("9 月 8 日");
      await hold(page, 2);
    });

    await test.step("13. 使い始める前の日（09-04）は「気分 使い始める前」で、未記入も破線も出ない", async () => {
      await openDay(page, "2026-09-04");
      await expect(head(page)).toContainText("気分 使い始める前");
      await expect(head(page)).not.toContainText("未記入");
      await expect(head(page)).not.toHaveCSS("outline-style", "dashed");
      await hold(page, 2);
    });

    await test.step("14. 滞在に書いた日（09-15）は、その滞在の行に「気分 …」が 1 行添えてある", async () => {
      await openDay(page, "2026-09-15");
      await expect(page.getByTestId("row-stay").getByTestId("row-feeling")).toHaveText(["気分 +2 とても快", "気分 -1 不快"]);
      await page.getByTestId("row-stay").first().scrollIntoViewIfNeeded();
      await hold(page, 3);
    });

    await test.step("15. OS をライトにすると主観入力の画面も明るくなる", async () => {
      await openFeel(page, `#/feel/${day}`);
      await page.emulateMedia({ colorScheme: "light" });
      await hold(page, 3);
    });
  } finally {
    // 通知の時刻を既定の 21:00 へ戻す（台帳は追記のみなので、戻す行を足す。e2e と同じ）
    const res = await page.request.post("/api/feelings/reminder", { headers: BEARER, data: { at: "21:00" } });
    expect(res.status()).toBe(200);
  }
});
