import { expect, test, type Page } from "@playwright/test";

/**
 * 主観入力の画面（S-3）の下限（ST17 / tasks 12.1 / NFR-17〜19 / NFR-22 / NFR-23）。本物のブラウザ・本物のサーバ。
 *
 * 数値は**リテラルで持つ**（トークンと一緒に動かない）。コントラストは `getComputedStyle` の色から計算し直した比、
 * 実寸は `getBoundingClientRect`、フォーカスは `Tab` で移して輪郭の色を測る。見るのは偽データの固定の日
 * （`tools/seed.sh normal` の主観の節）だけで、書かない。
 */

test.use({ viewport: { width: 360, height: 740 } });

const NFR18_TEXT = 4.5;
const NFR19_MIN_PX = 24;
const NFR23_NON_TEXT = 3;
const FOCUS_RING_PX = 3;
const VALENCE_BUTTON_MIN_PX = 64;
const SCHEMES = ["light", "dark"] as const;

/** 滞在 2 件（押された状態の行がある）。 */
const STAYS_DAY = "2026-09-15";
/** 未記入の日（破線）がある。 */
const MISSING_DAY = "2026-10-03";

async function open(page: Page, day: string): Promise<void> {
  await page.goto(`/#/feel/${day}`);
  await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("feel-stays-loading")).toHaveCount(0);
  await expect(page.getByTestId("feel-view")).toHaveAttribute("data-ready", "true");
}

// ------------------------------------------------------------------ 色（getComputedStyle から比を計算する）

type Rgb = [number, number, number];
type Rgba = [number, number, number, number];

function parseColor(s: string): Rgba {
  const nums = s.match(/-?[\d.]+/g)?.map(Number) ?? [];
  if (s.startsWith("color(")) return [nums[0] * 255, nums[1] * 255, nums[2] * 255, nums[3] ?? 1];
  if (s.startsWith("rgb")) return [nums[0], nums[1], nums[2], nums[3] ?? 1];
  throw new Error(`読めない色: ${s}`);
}

/** 外側から重ねる。いちばん外が透明なら白（ブラウザの既定の地）。 */
function composite(layers: string[]): Rgb {
  let out: Rgb = [255, 255, 255];
  for (const layer of [...layers].reverse()) {
    const [r, g, b, a] = parseColor(layer);
    out = [r * a + out[0] * (1 - a), g * a + out[1] * (1 - a), b * a + out[2] * (1 - a)];
  }
  return out;
}

function luminance([r, g, b]: Rgb): number {
  const lin = (c: number): number => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function ratio(a: Rgb, b: Rgb): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/** 自分から外へ向かう背景の色（`background-color`）を、`from` の要素について集める。 */
const bgChain = `(el) => { const out = []; for (let n = el; n !== null; n = n.parentElement) out.push(getComputedStyle(n).backgroundColor); return out; }`;

interface Painted {
  label: string;
  fg: string;
  bgs: string[];
}

/** 画面の中の「文字を持つ要素」（と入力欄）の色と、重なる背景の色。 */
async function paintedText(page: Page): Promise<Painted[]> {
  return page.evaluate(() => {
    const root = document.querySelector("[data-testid=feel-view]");
    if (root === null) return [];
    const out: { label: string; fg: string; bgs: string[] }[] = [];
    for (const el of Array.from(root.querySelectorAll("*"))) {
      if (el.tagName === "STYLE") continue;
      const own = Array.from(el.childNodes).some((n) => n.nodeType === 3 && (n.textContent ?? "").trim() !== "");
      if (!own && !el.matches("input:not([type=radio]), textarea")) continue;
      if (el.getClientRects().length === 0) continue;
      const bgs: string[] = [];
      for (let n: Element | null = el; n !== null; n = n.parentElement) bgs.push(getComputedStyle(n).backgroundColor);
      out.push({ label: `<${el.tagName.toLowerCase()}> ${(el.textContent ?? "").trim().slice(0, 24)}`, fg: getComputedStyle(el).color, bgs });
    }
    return out;
  });
}

// ------------------------------------------------------------------ 実寸

// Scenario: 快不快のボタンは高さ 64 px 以上
// Scenario: 快不快のボタンは 1 列に並ぶ
test("幅 360 px で快–不快の 5 つのボタンは高さ 64 px 以上・幅 24 px 以上で、上端がそろっている", async ({ page }) => {
  await open(page, STAYS_DAY);
  const boxes = await page.getByTestId(/^valence-/).evaluateAll((els) =>
    els.map((e) => {
      const r = e.getBoundingClientRect();
      return { top: r.top, width: r.width, height: r.height };
    }),
  );
  expect(boxes).toHaveLength(5);
  for (const b of boxes) {
    expect(b.height).toBeGreaterThanOrEqual(VALENCE_BUTTON_MIN_PX);
    expect(b.width).toBeGreaterThanOrEqual(NFR19_MIN_PX);
  }
  expect(new Set(boxes.map((b) => Math.round(b.top))).size, "上端がそろっていない").toBe(1);
});

// Scenario: 幅 360 px で横のスクロールが出ない
test("幅 360 px で主観入力の画面を開いても、ページの横幅は 360 px を超えない", async ({ page }) => {
  for (const day of [STAYS_DAY, MISSING_DAY]) {
    await open(page, day);
    const width = await page.evaluate(() => document.documentElement.scrollWidth);
    expect(width, `${day} の横幅`).toBeLessThanOrEqual(360);
  }
});

// Scenario: 主観入力の画面は触れる対象の下限を満たす
test("ボタン・暦の日・滞在の行のボタン・入力欄は、どれも幅と高さが 24 px 以上", async ({ page }) => {
  await open(page, STAYS_DAY);
  const small = await page.getByTestId("feel-view").evaluate((root, min) => {
    const out: string[] = [];
    for (const el of Array.from(root.querySelectorAll("button, input:not([type=hidden]), textarea, select, a[href]"))) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 && r.height === 0) continue;
      if (r.width < min || r.height < min) out.push(`${el.tagName} ${el.getAttribute("data-testid") ?? ""} ${r.width}x${r.height}`);
    }
    return out;
  }, NFR19_MIN_PX);
  expect(small).toEqual([]);
  // 暦の日と滞在の行のボタンが測られている
  expect(await page.locator('[data-testid^="cal-20"]').count()).toBe(30);
  expect(await page.getByTestId("feel-stay-row").count()).toBeGreaterThan(0);
});

// ------------------------------------------------------------------ 色

// Scenario: 主観入力の画面は文字のコントラストの下限を満たす
test("明と暗のそれぞれで、本文と補助の文字は背景に対して 4.5:1 以上", async ({ page }) => {
  for (const scheme of SCHEMES) {
    await page.emulateMedia({ colorScheme: scheme });
    for (const day of [STAYS_DAY, MISSING_DAY]) {
      await open(page, day);
      await expect(page.getByTestId("feel-view")).toHaveAttribute("data-scheme", scheme);
      const painted = await paintedText(page);
      expect(painted.length).toBeGreaterThan(10);
      const low = painted.map((p) => ({ label: p.label, r: ratio(composite([p.fg]), composite(p.bgs)) })).filter((x) => x.r < NFR18_TEXT);
      expect(low, `${scheme} ${day}: 4.5:1 を割った文字 ${JSON.stringify(low)}`).toEqual([]);
    }
  }
});

// Scenario: 押された状態と未記入の破線は 3:1 以上
test("明と暗のそれぞれで、押された快–不快のボタンの縁と未記入の日の破線は、隣の面に対して 3:1 以上", async ({ page }) => {
  for (const scheme of SCHEMES) {
    await page.emulateMedia({ colorScheme: scheme });

    await open(page, STAYS_DAY);
    const pressed = page.locator('[data-testid="feel-stay-row"] [data-testid^="stay-"][aria-pressed="true"]').first();
    await expect(pressed).toBeVisible();
    // 押された状態の目印は 2 px のアクセントの縁（面の塗りは文字の 4.5:1 を優先した暗い紫で、暗では隣の面と 3:1 にならない）
    const [edge, beside] = await pressed.evaluate((el, chain) => {
      const bgOf = eval(chain) as (e: Element | null) => string[];
      return [getComputedStyle(el).borderTopColor, bgOf(el.parentElement)];
    }, bgChain);
    expect(ratio(composite([edge]), composite(beside)), `${scheme}: 押された縁と隣の面`).toBeGreaterThanOrEqual(NFR23_NON_TEXT);

    await open(page, MISSING_DAY);
    const dashed = page.locator('[data-testid^="cal-20"][data-status="missing"]').first();
    await expect(dashed).toBeVisible();
    await expect(dashed).toHaveCSS("border-top-style", "dashed");
    const [line, behind] = await dashed.evaluate((el, chain) => {
      const bgOf = eval(chain) as (e: Element | null) => string[];
      return [getComputedStyle(el).borderTopColor, bgOf(el.parentElement)];
    }, bgChain);
    expect(ratio(composite([line]), composite(behind)), `${scheme}: 未記入の破線と隣の面`).toBeGreaterThanOrEqual(NFR23_NON_TEXT);
  }
});

// Scenario: 主観入力の画面はフォーカスの輪郭が見える
test("明と暗のそれぞれで、Tab で快–不快のボタンに移すと輪郭が出て、その色は隣の面に対して 3:1 以上", async ({ page }) => {
  for (const scheme of SCHEMES) {
    await page.emulateMedia({ colorScheme: scheme });
    await open(page, STAYS_DAY);
    await page.getByTestId("feel-prev").focus();
    let reached = false;
    for (let i = 0; i < 12 && !reached; i++) {
      await page.keyboard.press("Tab");
      reached = await page.evaluate(() => (document.activeElement?.getAttribute("data-testid") ?? "").startsWith("valence-"));
    }
    expect(reached, `${scheme}: 快–不快のボタンに Tab で届かない`).toBe(true);
    const ring = await page.evaluate((chain) => {
      const bgOf = eval(chain) as (e: Element | null) => string[];
      const el = document.activeElement as HTMLElement;
      const s = getComputedStyle(el);
      return { style: s.outlineStyle, width: Number.parseFloat(s.outlineWidth), color: s.outlineColor, beside: bgOf(el.parentElement) };
    }, bgChain);
    expect(ring.style).toBe("solid");
    expect(ring.width, `${scheme}: 輪郭の太さ`).toBeGreaterThanOrEqual(FOCUS_RING_PX);
    expect(ratio(composite([ring.color]), composite(ring.beside)), `${scheme}: 輪郭と隣の面`).toBeGreaterThanOrEqual(NFR23_NON_TEXT);
  }
});

// Scenario: 主観入力の画面は OS の明暗に追従する
test("OS の明暗を明にすると明の面で、設定が取得できないときは暗の面で描かれる", async ({ page }) => {
  const surface = async (): Promise<number> => {
    const bgs = await page.getByTestId("feel-view").evaluate(new Function("el", `return (${bgChain})(el)`) as (el: Element) => string[]);
    return luminance(composite(bgs));
  };

  await page.emulateMedia({ colorScheme: "light" });
  await open(page, STAYS_DAY);
  await expect(page.getByTestId("feel-view")).toHaveAttribute("data-scheme", "light");
  expect(await surface(), "明の面の輝度").toBeGreaterThan(0.5);

  // 取得できない状態: `matchMedia` が無い（画面は `typeof window.matchMedia !== "function"` のときダークにする。NFR-17）
  await page.addInitScript(() => {
    Object.defineProperty(window, "matchMedia", { value: undefined, configurable: true });
  });
  await page.reload();
  await expect(page.getByTestId("valence-1")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("feel-view")).toHaveAttribute("data-scheme", "dark");
  expect(await surface(), "暗の面の輝度").toBeLessThan(0.1);
});
