// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 録画専用（人間が後から動画で見る）。ST33「全記録の書き出し」を、設定画面で 1 本の流れで撮る:
 * 稼働状況のリンク → 設定画面（ソースごとの件数と期間・入らないもの・まだ書き出していない）→ 確認
 * （短い合言葉・不一致では押せない → やめる）→ 合言葉を入れて書き出す → 前回の書き出しが新しくなる
 * （受け取る操作は無い）→ 滞在を 1 件消すと前回の書き出しの欄に件数が出る → 後始末で戻す
 * → 幅 360 px・ダーク（確認を開いても横にはみ出さない・Tab でフォーカスの輪郭が見える）。
 *
 * アサーションは `e2e/settings-export.spec.ts` の主張の写し（spec を import すると向こうのテストまで登録されるので、
 * helper はここに写す）。足したのは見るための停止（HOLD）だけ。
 * 進み具合（`export-progress`）は主張しない: e2e は `ASHIATO_EXPORT_SELFTEST_DELAY_MS=400` で書き出しを遅らせて
 * 進み具合を確かめるが、録画の環境（`tools/record-env.sh`）はその遅延を入れないので、一瞬で終わって見えないことがある。
 * 代わりに「書き終えた結果」（前回の書き出しの行の数・ファイル）を `GET /api/export` と突き合わせる。
 */
declare const process: { env: Record<string, string | undefined> };

/** 録画専用の合言葉（12 文字以上。e2e と同じ値） */
const PW = "e2e-passphrase-0123";
const DAY = "2026-09-07";
const PHONE = { width: 360, height: 800 };
const DESKTOP = { width: 1280, height: 720 };
const HOLD = Number(process.env.REC_HOLD_MS ?? 1200);

type Last = { rows: number; bytes: number; file: string; erased_after: number | null };
type StatusBody = { running?: unknown; last?: Last | null; leftover: string[] };

async function hold(page: Page, times = 1): Promise<void> {
  await page.waitForTimeout(HOLD * times);
}

/** 画面の大きさの書き方（`web/src/export.ts` の `sizeLabel` と同じ 1000 刻み。e2e と同じ） */
function sizeText(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(bytes >= 1e7 ? 0 : 1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1e3))} KB`;
}

/** 読み出しは本物のブラウザの fetch でやる（ログインの印は `secure`。e2e と同じ事情） */
async function call(page: Page, path: string): Promise<{ ok: boolean; status: number; json: unknown }> {
  return page.evaluate(async (path) => {
    const res = await fetch(path);
    const json: unknown = await res.json().catch(() => null);
    return { ok: res.ok, status: res.status, json };
  }, path);
}

async function status(page: Page): Promise<StatusBody> {
  const res = await call(page, "/api/export");
  expect(res.ok, `GET /api/export は ${res.status}`).toBe(true);
  return res.json as StatusBody;
}

async function settle(page: Page): Promise<void> {
  await expect
    .poll(async () => {
      const s = await status(page);
      return s.running === undefined || s.running === null;
    }, { timeout: 90_000, intervals: [250] })
    .toBe(true);
}

async function openSettings(page: Page): Promise<void> {
  await page.goto("/#/settings");
  await page.reload();
  await expect(page.getByTestId("settings-view")).toBeVisible();
  await expect(page.getByTestId("export-sources")).toBeVisible({ timeout: 90_000 });
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

test("ST33 録画: 設定画面から全記録を書き出す（確認・合言葉・前回の書き出し・消した後の件数・電話の幅）", async ({
  page,
  browser,
  browserName,
}) => {
  test.info().annotations.push({ type: "browser", description: `${browserName} ${browser.version()}` });
  // 見るための停止・書き出しの終わり待ち・電話の幅まで撮る
  test.setTimeout(300_000);
  const posts: string[] = [];
  page.on("request", (req) => {
    if (req.method() === "POST" && new URL(req.url()).pathname === "/api/export") posts.push(req.postData() ?? "");
  });

  await test.step("1. 稼働状況の「設定」から設定画面を開く（最初の節は「全記録の書き出し」）", async () => {
    await page.goto("/#/coverage");
    await page.reload();
    await hold(page);
    await page.getByTestId("to-settings").click();
    await expect(page).toHaveURL(/#\/settings$/);
    await expect(page.getByTestId("settings-view").locator("h2").first()).toHaveText("全記録の書き出し");
    await expect(page.getByTestId("export-sources")).toBeVisible({ timeout: 90_000 });
    await settle(page);
    await hold(page, 2);
  });

  const before = await test.step("2. 押す前: ソースごとの件数と期間、入らないもの 3 行（件数なし）、前回の書き出し", async () => {
    const res = await call(page, "/api/export/summary");
    expect(res.ok).toBe(true);
    const body = res.json as { sources: { display_name: string; count: number }[] };
    expect(body.sources.length).toBeGreaterThan(0);
    const rows = page.getByTestId("export-sources").locator("li");
    await expect(rows).toHaveCount(body.sources.length);
    for (const [i, s] of body.sources.entries()) {
      await expect(rows.nth(i)).toContainText(`${s.display_name}: ${s.count.toLocaleString("ja-JP")} 件`);
      await expect(rows.nth(i)).toContainText(/\d{4}\/\d+\/\d+ 〜 \d{4}\/\d+\/\d+/);
    }
    await expect(page.getByText("入らないもの", { exact: true })).toBeVisible();
    const items = page.getByTestId("export-excluded").locator("li");
    await expect(items).toHaveCount(3);
    await expect(items.nth(0)).toContainText("本人が消した場面");
    await expect(items.nth(1)).toContainText("削除と消去の台帳");
    await expect(items.nth(2)).toContainText("写真・動画の原本");
    for (const t of await items.allInnerTexts()) {
      expect(t.replace("D-02", ""), `件数が付いている: ${t}`).not.toMatch(/\d/);
    }
    const last = (await status(page)).last ?? null;
    // 録画の環境は毎回作り直すので、ふつうはまだ書き出していない
    if (last === null) await expect(page.getByTestId("export-last")).toContainText("まだ書き出していません。");
    await hold(page, 2);
    await page.mouse.wheel(0, 500);
    await hold(page);
    await page.mouse.wheel(0, -500);
    return last;
  });

  await test.step("3. 押すとその場に確認が出る（行数・大きさ・時間の目安、置き場、暗号化、合言葉は保存しない）", async () => {
    await openConfirm(page);
    expect(posts, "確認の時点で POST が送られている").toHaveLength(0);
    const confirm = page.getByTestId("export-confirm");
    await expect(confirm).toContainText(/約 [\d,]+ 行・約 .+・約 [\d,]+ 秒/);
    await expect(confirm).toContainText("この PC のフォルダ");
    await expect(confirm).toContainText("暗号化した 1 つのファイル");
    await expect(confirm).toContainText("合言葉は保存しません");
    await expect(confirm).toContainText("AES");
    if (before === null) await expect(confirm).not.toContainText("前回の書き出しのファイルは消えます");
    else await expect(confirm).toContainText("前回の書き出しのファイルは消えます");
    for (const i of [0, 1]) {
      const ac = await pwInputs(page).nth(i).getAttribute("autocomplete");
      expect(ac).not.toBe("new-password");
      expect(ac).not.toBe("current-password");
    }
    await hold(page, 2);
  });

  await test.step("4. 合言葉が 11 文字、または 12 文字で不一致なら「書き出す」は押せない", async () => {
    const reason = page.getByTestId("export-reason");
    await fillBoth(page, "a".repeat(11), "a".repeat(11));
    await expect(goButton(page)).toBeDisabled();
    await expect(reason).toBeVisible();
    await expect(reason).toContainText("12 文字以上");
    await hold(page, 2);
    await fillBoth(page, "a".repeat(12), "b".repeat(12));
    await expect(goButton(page)).toBeDisabled();
    await expect(reason).toContainText("一致していません");
    await hold(page, 2);
    expect(posts).toHaveLength(0);
  });

  await test.step("5. 確認で「やめる」と書き出しは始まらず、押す前の形に戻る", async () => {
    await fillBoth(page, PW, PW);
    await hold(page);
    await page.getByRole("button", { name: "やめる" }).click();
    await expect(page.getByTestId("export-confirm")).toHaveCount(0);
    await expect(pwInputs(page)).toHaveCount(0);
    await expect(startButton(page)).toBeEnabled();
    expect(posts).toHaveLength(0);
    await hold(page);
  });

  await test.step("6. 合言葉を 2 回入れて「書き出す」→ 求めは 1 回だけ、合言葉は画面にも URL にも残らない", async () => {
    await openConfirm(page);
    const url = page.url();
    await fillBoth(page, PW, PW);
    await hold(page);
    await goButton(page).click();
    await expect(pwInputs(page)).toHaveCount(0);
    expect(posts).toHaveLength(1);
    expect((JSON.parse(posts[0]) as { password: string }).password).toBe(PW);
    expect(await page.content()).not.toContain(PW);
    expect(page.url()).toBe(url);
    await hold(page);
  });

  await test.step("7. 書き終えると前回の書き出しが新しくなる（行の数・大きさ・ファイル。受け取る操作は無い）", async () => {
    await settle(page);
    await expect(page.getByTestId("export-progress")).toBeHidden({ timeout: 90_000 });
    const last = (await status(page)).last;
    expect(last).toBeTruthy();
    expect((last as Last).file, "前回の書き出しが新しくなっていない").not.toBe(before?.file);
    const box = page.getByTestId("export-last");
    await expect(box).toContainText(`${(last as Last).rows.toLocaleString("ja-JP")} 行`);
    await expect(box).toContainText(sizeText((last as Last).bytes));
    await expect(box).toContainText((last as Last).file);
    await expect(box).toContainText(/\d{4}\/\d+\/\d+ \d{2}:\d{2}/);
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
    await box.scrollIntoViewIfNeeded();
    await hold(page, 3);
  });

  const row = page.getByTestId("row-stay").first();
  let range = "";
  let erased = false;
  try {
    await test.step(`8. 書き出した後に ${DAY} の滞在を 1 件消す`, async () => {
      await page.goto(`/#/day/${DAY}`);
      await page.reload();
      await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
      await expect(row).toBeVisible();
      range = (await row.locator("h2").innerText()).trim();
      await hold(page);
      await row.getByRole("button").first().click();
      await row.getByRole("button", { name: "この滞在を消す" }).click();
      await hold(page);
      await row.getByRole("button", { name: "消す" }).click();
      erased = true;
      await expect(page.getByTestId("row-erased").filter({ hasText: range })).toHaveCount(1);
      await hold(page);
    });

    await test.step("9. 設定画面に戻ると、前回の書き出しの欄に「この書き出しの後に消した記録」の件数が出る", async () => {
      await openSettings(page);
      const note = page.getByTestId("export-erased-after");
      await expect(note).toBeVisible();
      const text = (await note.innerText()).trim();
      const m = /^この書き出しの後に消した記録が ([\d,]+) 件あります。書き出したファイルの中には残っています。$/.exec(text);
      expect(m, `文言が違う: ${text}`).not.toBeNull();
      const n = Number((m as RegExpExecArray)[1].replace(/,/g, ""));
      const last = (await status(page)).last as Last;
      expect(n).toBe(last.erased_after);
      expect(n).toBeGreaterThanOrEqual(1);
      await note.scrollIntoViewIfNeeded();
      await hold(page, 3);
    });

    await test.step(`後始末: 「戻す」で ${DAY} の滞在を元に戻す`, async () => {
      await page.goto(`/#/day/${DAY}`);
      await page.reload();
      await expect(page.getByTestId("day-loading")).toBeHidden({ timeout: 15_000 });
      await page.getByTestId("row-erased").filter({ hasText: range }).getByRole("button", { name: "戻す" }).click();
      erased = false;
      await expect(page.getByTestId("row-stay").filter({ hasText: range })).toHaveCount(1);
      await hold(page);
    });
  } finally {
    if (erased && range !== "") {
      await page
        .getByTestId("row-erased")
        .filter({ hasText: range })
        .getByRole("button", { name: "戻す" })
        .click({ timeout: 5_000 })
        .catch(() => undefined);
    }
  }

  await test.step("10. 幅 360 px・ダーク: ファイルのパスが出ていて、確認を開いても横にはみ出さない", async () => {
    await page.setViewportSize(PHONE);
    await page.emulateMedia({ colorScheme: "dark" });
    await openSettings(page);
    const last = (await status(page)).last as Last;
    await expect(page.getByTestId("export-last")).toContainText(last.file);
    const fits = (): Promise<boolean> =>
      page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth);
    expect(await fits(), "確認を開く前").toBe(true);
    await hold(page, 2);
    await openConfirm(page);
    expect(await fits(), "確認を開いた後").toBe(true);
    await hold(page, 2);
    await page.getByRole("button", { name: "やめる" }).click();
    await expect(page.getByTestId("export-confirm")).toHaveCount(0);
  });

  await test.step("11. キーボードだけで「全記録を書き出す」→ 合言葉の入力へ（フォーカスの輪郭が見える）", async () => {
    const outlined = (): Promise<boolean> =>
      page.evaluate(() => {
        const el = document.activeElement;
        if (el === null) return false;
        const s = getComputedStyle(el);
        return s.outlineStyle !== "none" && parseFloat(s.outlineWidth) > 0;
      });
    const onStart = (): Promise<boolean> =>
      page.evaluate(() => document.activeElement?.getAttribute("data-testid") === "export-start");
    await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    for (let i = 0; i < 20 && !(await onStart()); i++) await page.keyboard.press("Tab");
    expect(await onStart(), "Tab で全記録を書き出すへ移れない").toBe(true);
    expect(await outlined(), "全記録を書き出すに輪郭が無い").toBe(true);
    await hold(page);
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("export-confirm")).toBeVisible();
    await expect
      .poll(() => page.evaluate(() => document.activeElement === document.querySelectorAll("input[type=password]")[0]))
      .toBe(true);
    expect(await outlined(), "合言葉の入力に輪郭が無い").toBe(true);
    await hold(page, 2);
    await page.getByRole("button", { name: "やめる" }).click();
    await expect(page.getByTestId("export-confirm")).toHaveCount(0);
    await page.setViewportSize(DESKTOP);
  });
});
