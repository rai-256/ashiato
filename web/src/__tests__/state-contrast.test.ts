// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 格子の 3 段（tasks 8.3 / NFR-23 / WCAG SC 1.4.11）。
 *
 * **トークンに書いた「3:1 以上」を信じない。** 描くのと同じ値から相対輝度を計算する ——
 * `ui-direction` の独立レビューは、**測る色と実際に描く色がずれていて
 * 全状態の 23.4% が 4.5:1 未満**だった事例を実測している（UIR-13 / UIR-38）。
 */
import { describe, expect, it } from "vitest";
import { contrastRatio, relativeLuminance } from "../contrast";
import { bandOf, type DayState } from "../coverage";
import { BAND, HUE, SAT, SURFACE } from "../tokens";

const lum = (l: number): number => relativeLuminance(HUE, SAT, l);

describe("格子の 3 段", () => {
  // Scenario: グレースケールでも格子の 3 段が区別できる
  it("隣り合う 2 段の相対輝度比が 3:1 以上", () => {
    // 暗い順に並べる。**隣接だけを見る**（WCAG SC 1.4.11 が求めるのは隣接）
    const ordered = [BAND.other, BAND.alive_no_record, BAND.recorded];
    for (let i = 0; i + 1 < ordered.length; i++) {
      const r = contrastRatio(lum(ordered[i]), lum(ordered[i + 1]));
      expect(r, `${ordered[i]}% と ${ordered[i + 1]}% の比が ${r.toFixed(3)}`).toBeGreaterThanOrEqual(3);
    }
  });

  it("段は 3 つだけ（7 段にしない）", () => {
    // **7 段は計算で不成立が確定している** —— 隣接 3:1 を 6 区間積むと 3^6 = 729:1 が要り、
    // sRGB の理論最大は 21:1。21:1 を 6 等分しても隣接 1.661:1 にしかならない。
    expect(Object.keys(BAND)).toHaveLength(3);
    const seven = 3 ** 6;
    const srgbMax = contrastRatio(relativeLuminance(0, 0, 100), relativeLuminance(0, 0, 0));
    expect(srgbMax).toBeLessThan(seven);
    expect(srgbMax).toBeCloseTo(21, 0);
  });

  it("7 状態がこの 3 段に畳まれる", () => {
    const all: DayState[] = [
      "recorded",
      "alive_no_record",
      "alive_not_capturable",
      "stopped",
      "dropped",
      "outage",
      "before_start",
    ];
    expect(all.map(bandOf)).toEqual([
      "recorded",
      "alive_no_record",
      "other",
      "other",
      "other",
      "other",
      "other",
    ]);
    // **畳んだ先が 3 段のどれかであること**（新しい状態を足しても段が増えない）
    for (const s of all) expect(BAND[bandOf(s)]).toBeTypeOf("number");
  });

  it("いちばん暗い段も、格子を置く面の上で見える", () => {
    // 段どうしの 3:1 は満たしても、**地と同じ色**だと格子の広がりが読めない。
    // 格子は surface2 の上に置く（tokens.ts）
    const r = contrastRatio(lum(BAND.other), lum(SURFACE.surface2));
    expect(r, `「それ以外」と面の比が ${r.toFixed(3)}`).toBeGreaterThan(1.5);
  });
});
