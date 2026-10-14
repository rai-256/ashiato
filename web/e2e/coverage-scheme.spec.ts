// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test, type Page } from "@playwright/test";

/**
 * 稼働状況の画面は OS の明暗に追従する（ST14 / tasks 10.4 / design D21）。コードは変えず、試験で固定する。
 * `surface.spec.ts` の測りは残す。ここは Scenario ごとに何を測るかを 1 本ずつ書く。
 * 面の色・比の下限は**リテラル**で持つ（トークンと一緒に動かない）。
 */

// 面の背景（地）。ライト hsl(140 18% 99%)・ダーク hsl(140 18% 13%)
const LIGHT_GROUND = [252, 253, 252];
const DARK_GROUND = [27, 39, 31];
const NON_TEXT_MIN = 3;
const TEXT_MIN = 4.5;

async function open(page: Page): Promise<void> {
  await page.goto("/#/coverage");
  await page.reload();
  await expect(page.getByTestId("coverage-loading")).toBeHidden({ timeout: 15_000 });
  await expect(page.locator("[data-role=grid]").first()).toBeVisible({ timeout: 15_000 });
}

async function groundOf(page: Page): Promise<number[]> {
  return page.getByTestId("coverage-view").evaluate((el) =>
    (getComputedStyle(el).backgroundColor.match(/[\d.]+/g) ?? []).slice(0, 3).map(Number),
  );
}

function near(actual: number[], want: number[]): void {
  expect(actual).toHaveLength(3);
  actual.forEach((v, i) => expect(Math.abs(v - want[i]), `チャンネル ${i}: ${actual} ≠ ${want}`).toBeLessThanOrEqual(1));
}

// Scenario: OS がライトなら稼働状況もライトで出る
test("OS の設定がライトなら、背景はライトの面の色", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await open(page);
  await expect(page.getByTestId("coverage-view")).toHaveAttribute("data-scheme", "light");
  near(await groundOf(page), LIGHT_GROUND);
});

// Scenario: OS がダークなら稼働状況もダークで出る
test("OS の設定がダークなら、背景はダークの面の色", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await open(page);
  await expect(page.getByTestId("coverage-view")).toHaveAttribute("data-scheme", "dark");
  near(await groundOf(page), DARK_GROUND);
});

// Scenario: 明暗の設定が取れなければダークで出る
test("明暗の設定を取得できない（matchMedia が無い）とき、背景はダークの面の色", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, "matchMedia", { value: undefined, configurable: true });
  });
  // 取得できないので、emulateMedia の値には従えない
  await page.emulateMedia({ colorScheme: "light" });
  await open(page);
  await expect(page.getByTestId("coverage-view")).toHaveAttribute("data-scheme", "dark");
  near(await groundOf(page), DARK_GROUND);
});

// 上の Scenario の「取れない」は matchMedia が無いこと。Chromium は `no-preference` を取得できた「ライトに合う」として返すので、
// 画面はそれに従う（取れなかったのではない）。この振る舞いはコードを変えずに記録しておく（design D21）
test("no-preference は Chromium が (prefers-color-scheme: light) に合うと答えるので、画面はそれに従う", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "no-preference" });
  await open(page);
  const matchesLight = await page.evaluate(() => window.matchMedia("(prefers-color-scheme: light)").matches);
  await expect(page.getByTestId("coverage-view")).toHaveAttribute("data-scheme", matchesLight ? "light" : "dark");
  near(await groundOf(page), matchesLight ? LIGHT_GROUND : DARK_GROUND);
});

type Measured = { what: string; ratio: number };

/** ライトで、格子の隣り合う 2 段・セルと節の面・破棄の印・本文の比を、描かれた色から測る。 */
async function lightRatios(page: Page): Promise<{ bands: Measured[]; cells: Measured[]; mark: Measured[]; text: Measured[] }> {
  return page.evaluate(() => {
    const parse = (c: string): number[] => (c.match(/[\d.]+/g) ?? []).map(Number);
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
    const bandColor = (band: string): number[] | null => {
      const el = document.querySelector(`[data-cell='day'][data-band='${band}']`);
      return el ? parse(getComputedStyle(el).backgroundColor) : null;
    };
    const [rec, alive, other] = ["recorded", "alive_no_record", "other"].map(bandColor);
    const bands: Measured[] = [];
    if (rec && alive) bands.push({ what: "記録あり と 動いていた・記録なし", ratio: ratio(rec, alive) });
    if (alive && other) bands.push({ what: "動いていた・記録なし と それ以外", ratio: ratio(alive, other) });
    // セルの広がり: 「記録あり」「動いていた・記録なし」は塗りと節の面。「それ以外」は節の面に近いので 1 px の縁（box-shadow）を、
    // 節の面とセルの塗りの両方に対して測る
    const cells: Measured[] = [];
    for (const band of ["recorded", "alive_no_record", "other"]) {
      const cell = document.querySelector(`[data-cell='day'][data-band='${band}']`);
      if (!cell) continue;
      const fill = parse(getComputedStyle(cell).backgroundColor);
      const face = backOf(cell.closest("section"));
      if (band !== "other") {
        cells.push({ what: `セル ${band} と節の面`, ratio: ratio(fill, face) });
        continue;
      }
      const edge = parse(getComputedStyle(cell).boxShadow).slice(0, 3);
      cells.push({ what: "セル other の縁と節の面", ratio: ratio(edge, face) });
      cells.push({ what: "セル other の縁と塗り", ratio: ratio(edge, fill) });
    }
    const mark: Measured[] = [];
    for (const m of document.querySelectorAll("[data-drop-mark]")) {
      mark.push({ what: "破棄の印", ratio: ratio(parse(getComputedStyle(m).borderBottomColor), backOf(m.parentElement)) });
    }
    const text: Measured[] = [];
    for (const el of document.querySelectorAll("main h2, main p, main dt, main dd")) {
      if (el.closest("[aria-hidden='true']")) continue;
      if (!(el.textContent ?? "").trim()) continue;
      text.push({ what: (el.textContent ?? "").trim().slice(0, 20), ratio: ratio(parse(getComputedStyle(el).color), backOf(el)) });
    }
    return { bands, cells, mark, text };
  });
}

// Scenario: ライトでも格子の 3 段と印と文字が下限を満たす
test("ライトで、格子の隣り合う 2 段・印・セルは 3:1、本文は 4.5:1 以上", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await open(page);
  // 破棄の印は偽データの 2026-09-07（位置）に付く。直近 4 週の外なので伸ばす
  await page.locator("section[data-source='c01-location']").getByRole("button", { name: "1 年ぶんを見る" }).click();
  const r = await lightRatios(page);
  expect(r.bands.length, "3 段の隣り合う組が測れていない").toBeGreaterThan(0);
  expect(r.mark.length, "破棄の印が画面に無い（偽データの不足）").toBeGreaterThan(0);
  for (const x of [...r.bands, ...r.cells, ...r.mark]) {
    expect(x.ratio, `${x.what} ${x.ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(NON_TEXT_MIN);
  }
  expect(r.text.length, "測る本文が見つからない").toBeGreaterThan(3);
  for (const x of r.text) expect(x.ratio, `${x.what} ${x.ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(TEXT_MIN);
});
