// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 表面のトークン。**`docs/ui-direction.md` の確定値から引く 1 か所**（design D12）。
 *
 * 確定値: 色相 132° / 彩度 30% / 地の明るさ 12 / 面 3 段。
 * **S-1 のためだけの色を足さない** —— 足すと ST25（S-2 主表現）が別のトークン系を持つ。
 */

/** 色相。`ui-direction.md` の確定値（緑 = 苔の方向）。 */
export const HUE = 132;
/** 彩度。同上。**低く抑える**側で決まっている。 */
export const SAT = 30;

/**
 * 面の 3 段。地の明るさ 12 から、`ui-direction.md` の「面 3 段」で積む。
 * 格子は `surface2` の上に置く —— **最も暗い段のセルがそこで読める**（下の `BAND` を参照）。
 */
export const SURFACE = { ground: 12, surface1: 18, surface2: 24 } as const;

/**
 * 格子のセルが担う **3 段**（design D10 / 深掘り 第 5 回 Q21）。
 *
 * **7 段にしない。** WCAG SC 1.4.11（NFR-23）は隣接する色に 3:1 を求めるが、
 * 7 段を隣接 3:1 で並べると 3^6 = 729:1 が要り、**sRGB の理論最大は 21:1**。
 * 21:1 を 6 等分しても隣接 1.661:1 にしかならず、**計算で不成立が確定している**。
 * 7 状態の区別は**週を選んだときの文字**が担う。
 *
 * この 3 つの明るさは隣接比が 3.89:1 と 3.81:1 で、NFR-23 を満たす
 * （`state-contrast.test.ts` が値から計算して確かめる。ここに書いた数を信じない）。
 */
export const BAND = {
  /** ③④⑤⑥⑦ を畳んだ段。地に近い側 */
  other: 9,
  /** ② 動いていた・記録なし */
  alive_no_record: 40,
  /** ① 記録あり */
  recorded: 92,
} as const;

/** 文字の明るさ。NFR-18（4.5:1）は `text-contrast.test.ts` が確かめる。 */
export const TEXT = { normal: 92, muted: 72 } as const;

/** HSL の文字列。**色相と彩度をここでしか触らせない。** */
export function tone(lightness: number): string {
  return `hsl(${HUE} ${SAT}% ${lightness}%)`;
}

/**
 * 操作できるものの最小の大きさ（NFR-19）。
 *
 * **週の帯がこれを満たす。セルは操作対象にしない**（深掘り 第 4 回 Q16）——
 * 1 週 7 日を横に並べると 1 セルが 24 px に満たない幅になりうる。
 * 選ぶ単位を週にすれば、セルの大きさに関わらず NFR-19 を満たせる。
 */
export const MIN_TARGET_PX = 24;

/** 開いた直後に見せる週数（深掘り 第 7 回 Q28）。**5 ソース × これ**が 1 画面に収まる。 */
export const INITIAL_WEEKS = 5;

/** 伸ばしたときの週数（1 年）。 */
export const YEAR_WEEKS = 53;
