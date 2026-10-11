// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * **表面の下限を本物のブラウザで測る**（2026-10-12 表面の作り直し。NFR-17〜20 / NFR-22 / NFR-23）。
 *
 * 単体（`text-contrast.test.ts` / `state-contrast.test.ts`）は**トークンの組**を計算する。ここは
 * **描かれたもの**を測る —— ブラウザが計算した文字の色と、その文字の後ろに実際にある面の色の組。
 * トークンの組を全部測っても、部品がトークンの外の面に文字を置けば割れる（UIR-13 と同じ型）。
 *
 * - 明暗は `emulateMedia({ colorScheme })` で OS の設定を差し替える（NFR-17。S-1 にもライトがある）
 * - 幅はスマホ（360 px。NFR-19 が基準にしている端末）と机（Desktop Chrome の既定）
 * - 見ているのは `run.sh` と同じ `tools/stack.sh` が立てた偽データ（`SEED=normal`）
 */

/** NFR の数値を**リテラルで持つ**（トークンと一緒に動かない）。 */
const NFR18_TEXT = 4.5;
const NFR19_MIN_PX = 24;
const NFR20_DESTRUCTIVE_PX = 44;
const NFR23_NON_TEXT = 3;
const FOCUS_RING_PX = 3;

const DAY = "2026-09-07";
const SCHEMES = ["light", "dark"] as const;

const SCREENS = [
  { name: "稼働状況", url: "/", root: "coverage-view", ready: "coverage-loading" },
  { name: "1 日の滞在", url: `/#/day/${DAY}`, root: "day-view", ready: "day-loading" },
  { name: "マスタ管理", url: "/#/master", root: "master-view", ready: "master-loading" },
] as const;

async function open(page: Page, url: string, ready: string): Promise<void> {
  await page.goto(url);
  await page.reload(); // ハッシュだけの遷移では読み直さない（day-stays.spec.ts と同じ）
  await expect(page.getByTestId(ready)).toBeHidden({ timeout: 15_000 });
}

type Measured = { what: string; ratio: number };

/**
 * 画面の中で**自分の文字を持つ要素**すべてについて、文字の色と、その後ろの面（祖先をたどって最初の不透明な背景）の比を測る。
 * 読み上げに出さないしるし（`aria-hidden`）は文字ではない —— 別に 3:1 で測る。
 */
async function textContrasts(page: Page): Promise<Measured[]> {
  return page.evaluate(() => {
    const parse = (c: string): [number, number, number, number] => {
      const m = c.match(/[\d.]+/g)?.map(Number) ?? [0, 0, 0, 0];
      return [m[0], m[1], m[2], m.length > 3 ? m[3] : 1];
    };
    const lin = (v: number): number => {
      const s = v / 255;
      return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    };
    const lum = ([r, g, b]: number[]): number => 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
    const ratio = (a: number[], b: number[]): number => {
      const [x, y] = [lum(a), lum(b)];
      return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
    };
    const backOf = (el: Element | null): number[] => {
      for (let e = el; e !== null; e = e.parentElement) {
        const bg = parse(getComputedStyle(e).backgroundColor);
        if (bg[3] > 0) return bg;
      }
      return [255, 255, 255, 1]; // キャンバスの既定
    };
    const out: { what: string; ratio: number }[] = [];
    for (const el of document.querySelectorAll("body *")) {
      if (el.closest("[aria-hidden='true']")) continue;
      const own = [...el.childNodes].some((n) => n.nodeType === Node.TEXT_NODE && (n.textContent ?? "").trim() !== "");
      const field = el instanceof HTMLInputElement && el.type !== "radio" && el.type !== "checkbox";
      if (!own && !field) continue;
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      const fg = parse(getComputedStyle(el).color);
      const text = (el.textContent ?? "").trim() || (el as HTMLInputElement).value || el.tagName;
      out.push({ what: `<${el.tagName.toLowerCase()}> ${text.slice(0, 24)}`, ratio: ratio(fg, backOf(el)) });
    }
    return out;
  });
}

/** 種類のしるし（線の輪）と格子の隣り合う段。**状態を表す非テキスト**（NFR-23）。 */
async function nonTextContrasts(page: Page): Promise<Measured[]> {
  return page.evaluate(() => {
    const parse = (c: string): number[] => c.match(/[\d.]+/g)?.map(Number) ?? [0, 0, 0];
    const lin = (v: number): number => {
      const s = v / 255;
      return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    };
    const lum = ([r, g, b]: number[]): number => 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
    const ratio = (a: number[], b: number[]): number => {
      const [x, y] = [lum(a), lum(b)];
      return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
    };
    const backOf = (el: Element | null): number[] => {
      for (let e = el; e !== null; e = e.parentElement) {
        const bg = parse(getComputedStyle(e).backgroundColor);
        if ((bg.length > 3 ? bg[3] : 1) > 0) return bg;
      }
      return [255, 255, 255];
    };
    const out: { what: string; ratio: number }[] = [];
    for (const mark of document.querySelectorAll("[data-mark]")) {
      const ring = parse(getComputedStyle(mark).borderTopColor);
      out.push({ what: `しるし ${mark.getAttribute("data-mark")}`, ratio: ratio(ring, backOf(mark.parentElement)) });
    }
    for (const week of document.querySelectorAll("[data-week]")) {
      const cells = [...week.querySelectorAll("[data-band]")].filter((c) => c.getAttribute("data-band") !== "empty");
      for (let i = 0; i + 1 < cells.length; i++) {
        const [a, b] = [cells[i], cells[i + 1]];
        if (a.getAttribute("data-band") === b.getAttribute("data-band")) continue;
        out.push({
          what: `格子 ${a.getAttribute("data-band")} と ${b.getAttribute("data-band")}`,
          ratio: ratio(parse(getComputedStyle(a).backgroundColor), parse(getComputedStyle(b).backgroundColor)),
        });
      }
    }
    return out;
  });
}

/** 操作できるもの（見えているもの）の実寸。 */
async function targets(page: Page): Promise<{ what: string; w: number; h: number }[]> {
  return page.evaluate(() =>
    [...document.querySelectorAll("button, a[href], input, select")]
      .map((el) => {
        const r = el.getBoundingClientRect();
        const name = el.getAttribute("aria-label") ?? (el.textContent ?? "").trim();
        return { what: `<${el.tagName.toLowerCase()}> ${name.slice(0, 24)}`, w: r.width, h: r.height };
      })
      .filter((t) => t.w > 0 && t.h > 0),
  );
}

const worst = (xs: Measured[]): string =>
  [...xs]
    .sort((a, b) => a.ratio - b.ratio)
    .slice(0, 3)
    .map((x) => `${x.what} ${x.ratio.toFixed(2)}:1`)
    .join(" / ");

for (const width of [360, 1280] as const) {
  test.describe(`幅 ${width} px`, () => {
    test.use({ viewport: { width, height: width === 360 ? 640 : 800 } });

    for (const scheme of SCHEMES) {
      for (const screen of SCREENS) {
        // Scenario: 一覧の文字はライトでもダークでも 4.5:1 を下回らない
        test(`${screen.name}（${scheme}）: 明暗に追従し、文字 4.5:1・しるしと段 3:1・操作 24 px・横に溢れない`, async ({ page }) => {
          await page.emulateMedia({ colorScheme: scheme });
          await open(page, screen.url, screen.ready);
          if (screen.root === "day-view") {
            // 詳細を開いて、消す操作（NFR-20）と詳細の面まで出す
            const row = page.getByTestId("row-stay").first();
            await row.getByRole("button").first().click();
            await expect(row.getByRole("button", { name: "この滞在を消す" })).toBeVisible();
          }

          // NFR-17: OS の明暗に追従している（S-1 は 2026-10-12 までダークしか無かった）
          await expect(page.getByTestId(screen.root)).toHaveAttribute("data-scheme", scheme);

          // NFR-18: 描かれた文字すべて
          const text = await textContrasts(page);
          expect(text.length, "測る文字が見つからない").toBeGreaterThan(5);
          const lowText = text.filter((t) => t.ratio < NFR18_TEXT);
          expect(lowText, `4.5:1 を割った文字: ${worst(text)}`).toEqual([]);

          // NFR-23: しるしと格子の隣り合う段
          const nonText = await nonTextContrasts(page);
          // 格子の隣り合う段は偽データ次第で出ない週もある。しるしは 1 日の滞在に必ず出る
          if (screen.root === "day-view") expect(nonText.length, "しるしが見つからない").toBeGreaterThan(0);
          const lowNonText = nonText.filter((t) => t.ratio < NFR23_NON_TEXT);
          expect(lowNonText, `3:1 を割った非テキスト: ${worst(nonText)}`).toEqual([]);

          // NFR-19: 見えている操作対象すべて
          const small = (await targets(page)).filter((t) => t.w < NFR19_MIN_PX - 0.05 || t.h < NFR19_MIN_PX - 0.05);
          expect(small, "24 × 24 CSS px を割った操作対象").toEqual([]);

          if (screen.root === "day-view") {
            // NFR-20: 消す操作は 44 × 44
            const box = await page.getByRole("button", { name: "この滞在を消す" }).boundingBox();
            expect(box?.width ?? 0).toBeGreaterThanOrEqual(NFR20_DESTRUCTIVE_PX);
            expect(box?.height ?? 0).toBeGreaterThanOrEqual(NFR20_DESTRUCTIVE_PX);
          }

          // 横スクロールを出さない
          const overflow = await page.evaluate(
            () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
          );
          expect(overflow, "横スクロールが出ている").toBeLessThanOrEqual(0);
        });
      }
    }
  });
}

// Scenario: キーボードで移るとフォーカスの位置が見える
test("稼働状況の週の帯にも、Tab で本文の色・3 px の輪郭が出る（選択中でも消えない）", async ({ page }) => {
  for (const scheme of SCHEMES) {
    await page.emulateMedia({ colorScheme: scheme });
    await open(page, "/", "coverage-loading");
    // 選んだ週は内側の影で示す。**style 属性の outline でフォーカスの輪郭を消さない**
    const week = page.locator("[data-week]").first();
    await week.click();
    await expect(week).toHaveAttribute("aria-pressed", "true");
    await week.blur();
    let ring = 0;
    for (let i = 0; i < 40 && ring === 0; i++) {
      await page.keyboard.press("Tab");
      ring = await page.evaluate(() => {
        const el = document.activeElement as HTMLElement | null;
        if (!el?.hasAttribute("data-week") || el.getAttribute("aria-pressed") !== "true") return 0;
        const s = getComputedStyle(el);
        return s.outlineStyle === "solid" && s.outlineColor === getComputedStyle(document.querySelector("main") as Element).color
          ? Number.parseFloat(s.outlineWidth)
          : -1;
      });
    }
    expect(ring, `${scheme}: 選んだ週に本文の色の輪郭が出ていない`).toBeGreaterThanOrEqual(FOCUS_RING_PX);
  }
});

test.describe("ログイン", () => {
  test.use({ storageState: { cookies: [], origins: [] } });

  for (const scheme of SCHEMES) {
    test(`ログイン（${scheme}）: 主ボタンはアクセントの塗り、文字 4.5:1、操作 44 px`, async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await page.goto("/");
      await expect(page.getByLabel("合言葉")).toBeVisible();
      await expect(page.getByTestId("login")).toHaveAttribute("data-scheme", scheme);
      const text = await textContrasts(page);
      expect(text.filter((t) => t.ratio < NFR18_TEXT), `4.5:1 を割った文字: ${worst(text)}`).toEqual([]);
      const button = page.getByRole("button", { name: "ログイン" });
      const box = await button.boundingBox();
      expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);
      // 主ボタンは塗る（地と同じ色にしない）
      const [fill, ground] = await page.evaluate(() => [
        getComputedStyle(document.querySelector("button[type=submit]") as Element).backgroundColor,
        getComputedStyle(document.querySelector("[data-testid=login]") as Element).backgroundColor,
      ]);
      expect(fill).not.toBe(ground);
    });
  }
});
