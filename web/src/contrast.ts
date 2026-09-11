// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 相対輝度と対比比（WCAG 2.2）。**検査が値から計算し直すための道具。**
 *
 * トークンに書いた「3:1 以上」を信じない —— `ui-direction` の独立レビューは、
 * **測る色と実際に描く色がずれていて全状態の 23.4% が 4.5:1 未満**だった事例を
 * 実測している（UIR-13 / UIR-38）。だから検査は**描くのと同じ値**から計算する。
 */

/** HSL（彩度・明るさは %）を sRGB の 0〜1 へ。 */
export function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const sat = s / 100;
  const light = l / 100;
  const hue = ((h % 360) + 360) % 360 / 360;
  if (sat === 0) return [light, light, light];
  const q = light < 0.5 ? light * (1 + sat) : light + sat - light * sat;
  const p = 2 * light - q;
  const channel = (t0: number): number => {
    let t = t0;
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  return [channel(hue + 1 / 3), channel(hue), channel(hue - 1 / 3)];
}

/** 相対輝度（WCAG 2.2 の定義）。 */
export function relativeLuminance(h: number, s: number, l: number): number {
  const linear = (c: number): number => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  const [r, g, b] = hslToRgb(h, s, l);
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

/** 対比比。大きいほうを分子に置く。 */
export function contrastRatio(a: number, b: number): number {
  const hi = Math.max(a, b);
  const lo = Math.min(a, b);
  return (hi + 0.05) / (lo + 0.05);
}
